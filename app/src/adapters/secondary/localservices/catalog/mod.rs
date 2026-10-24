//! Which local services a configuration needs and how each is started: the programs, models,
//! ports and arguments of the service scripts (`dev-tools`), found on this machine.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use voicepuncher_copy_pool::{ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET};

use crate::adapters::secondary::logging::file::logs_dir;
use crate::config::Config;
use crate::core::application::{ProbeAnswer, ServiceProbe};
use crate::core::domain::{ServiceKind, ServiceSpec};

const WHISPER: &str = "whisper-server";
const COMMAND_SPOTTING: &str = "voicepuncher-command-spotting";
const DICTATION_MODEL: &str = "ggml-large-v3-turbo-q5_0.bin";
const COMMAND_MODEL: &str = "ggml-small.bin";
const WHISPER_VAD_PREFIX: &str = "ggml-silero-";
const KWS_VAD_MODEL: &str = "silero_vad.onnx";
const KWS_MODEL_DIR_PREFIX: &str = "sherpa-onnx-kws-";
const RUN_SCRIPTS_HINT: &str = "run 'just models'";
/// The folder of the bundle's engines, next to the app's program (`Contents/MacOS/engines`).
const ENGINES_DIR: &str = "engines";
/// The file of an engine folder that holds the engine's version, written by `just engines`.
const VERSION_FILE: &str = "VERSION";

/// The engine folder inside the bundle that holds a program.
fn engine_folder(program: &str) -> &'static str {
    match program {
        WHISPER => "whisper",
        _ => "command-spotting",
    }
}

/// What the services are looked up on: the home folder, the environment, `$PATH`, the
/// folder of the running program and its processor cores.
#[derive(Debug, Clone, Default)]
pub struct Machine {
    pub home: PathBuf,
    pub env: HashMap<String, String>,
    pub path: Vec<PathBuf>,
    pub app_dir: Option<PathBuf>,
    pub cores: usize,
}

impl Machine {
    pub fn current() -> Self {
        Self {
            home: dirs::home_dir().unwrap_or_default(),
            env: std::env::vars().collect(),
            path: std::env::var_os("PATH")
                .map(|path| std::env::split_paths(&path).collect())
                .unwrap_or_default(),
            app_dir: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf)),
            cores: std::thread::available_parallelism().map_or(1, usize::from),
        }
    }

    fn var(&self, name: &str) -> Option<PathBuf> {
        self.env
            .get(name)
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
    }
}

/// Where a service's program came from, for the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineSource {
    Bundle,
    /// The variable that names the program.
    Environment(String),
    Path,
    BuildFolder,
}

/// A service's program, where it came from and its version when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineOrigin {
    pub program: String,
    pub version: Option<String>,
    pub source: EngineSource,
}

impl std::fmt::Display for EngineOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.version {
            Some(version) => write!(f, "{} {version}", self.program)?,
            None => write!(f, "{} (version unknown)", self.program)?,
        }
        match &self.source {
            EngineSource::Bundle => write!(f, " from the bundle"),
            EngineSource::Environment(var) => write!(f, " from {var}"),
            EngineSource::Path => write!(f, " from $PATH"),
            EngineSource::BuildFolder => write!(f, " from the build folder"),
        }
    }
}

/// Where a looked-up program came from — a variable naming it, the bundle's engine folders,
/// `$PATH`, else the build folder — and its version: the version file of its folder, else the
/// version folder of a Homebrew installation (`Cellar/<formula>/<version>/…`) it links to.
pub fn engine_origin(program: &Path, machine: &Machine) -> EngineOrigin {
    let named_by = machine
        .env
        .iter()
        .filter(|(_, value)| Path::new(value.trim()) == program)
        .map(|(name, _)| name.clone())
        .min();
    let source = if let Some(var) = named_by {
        EngineSource::Environment(var)
    } else if machine
        .app_dir
        .as_ref()
        .is_some_and(|dir| program.starts_with(dir.join(ENGINES_DIR)))
    {
        EngineSource::Bundle
    } else if program
        .parent()
        .is_some_and(|dir| machine.path.iter().any(|on_path| on_path == dir))
    {
        EngineSource::Path
    } else {
        EngineSource::BuildFolder
    };
    EngineOrigin {
        program: program
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        version: folder_version(program).or_else(|| cellar_version(program)),
        source,
    }
}

fn folder_version(program: &Path) -> Option<String> {
    let text = std::fs::read_to_string(program.with_file_name(VERSION_FILE)).ok()?;
    Some(text.trim().to_string()).filter(|version| !version.is_empty())
}

fn cellar_version(program: &Path) -> Option<String> {
    let resolved = program.canonicalize().ok()?;
    let parts: Vec<String> = resolved
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let cellar = parts.iter().position(|part| part == "Cellar")?;
    parts.get(cellar + 2).cloned()
}

/// The optional services the configuration turns on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Needs {
    pub voice_control: bool,
}

impl Needs {
    pub fn of(cfg: &Config) -> Self {
        Self { voice_control: cfg.voice_control.enabled }
    }
}

/// Every service process the configuration needs, in start order: the speech recognition
/// service for commands before the command spotting service, which asks it. A speech
/// recognition service that already runs as other copies than configured is taken as it runs.
pub fn needed(
    cfg: &Config,
    needs: Needs,
    machine: &Machine,
    probe: &dyn ServiceProbe,
) -> Vec<ServiceSpec> {
    let lookup = Lookup::new(cfg, machine);
    let mut specs = lookup.whisper(
        ServiceKind::Dictation,
        &cfg.transcription.base_url,
        cfg.services.dictation_copies,
        probe,
    );
    if needs.voice_control {
        specs.extend(lookup.whisper(
            ServiceKind::Commands,
            &cfg.services.commands_url,
            cfg.services.command_copies,
            probe,
        ));
        specs.extend(lookup.command_spotting(
            &cfg.voice_control.url,
            &cfg.services.commands_url,
            cfg.services.command_spotting_copies,
            probe,
        ));
    }
    specs
}

/// The ports of the copies of a service: one copy on the service's own
/// port, several on the first ports its clients look at; one on the service's own port too
/// when its port leaves no room for copy ports.
pub fn copy_ports(address: &ServiceAddress, copies: u32) -> Vec<u16> {
    let ports: Vec<u16> = address
        .copy_ports()
        .into_iter()
        .take(copies as usize)
        .collect();
    if copies <= 1 || ports.is_empty() {
        return vec![address.port()];
    }
    ports
}

/// Each port with the label of its copy: the service's name alone for a single copy.
fn labelled(kind: ServiceKind, ports: &[u16]) -> impl Iterator<Item = (u16, String)> + '_ {
    ports.iter().enumerate().map(move |(index, &port)| {
        let label = if ports.len() == 1 {
            kind.name().to_string()
        } else {
            format!("{}: copy {} of {}", kind.name(), index + 1, ports.len())
        };
        (port, label)
    })
}

/// `cores / copies`, at least one.
pub fn threads(cores: usize, copies: usize) -> usize {
    (cores / copies.max(1)).max(1)
}

struct Lookup<'a> {
    machine: &'a Machine,
    model_dirs: Vec<PathBuf>,
}

/// Something looked for and not found: what, and the hint to get it.
type Missing = String;

/// What every copy of a service shares: its kind, its program and what keeps it from starting.
struct Service {
    kind: ServiceKind,
    binary: &'static str,
    program: Result<PathBuf, Missing>,
    problem: Option<String>,
    copies: u32,
}

impl<'a> Lookup<'a> {
    fn new(cfg: &Config, machine: &'a Machine) -> Self {
        let voicepuncher = machine.home.join(".voicepuncher");
        let mut model_dirs = vec![cfg.services.resolved_models_dir(&machine.home)];
        for dir in [voicepuncher.join("models"), voicepuncher] {
            if !model_dirs.contains(&dir) {
                model_dirs.push(dir);
            }
        }
        Self { machine, model_dirs }
    }

    fn whisper(
        &self,
        kind: ServiceKind,
        url: &str,
        copies: u32,
        probe: &dyn ServiceProbe,
    ) -> Vec<ServiceSpec> {
        let Ok(address) = ServiceAddress::parse(url) else {
            return Vec::new();
        };
        let (model_var, model_file) = match kind {
            ServiceKind::Commands => ("VOICEPUNCHER_COMMAND_WHISPER_MODEL", COMMAND_MODEL),
            _ => ("VOICEPUNCHER_WHISPER_MODEL", DICTATION_MODEL),
        };
        let program = self.engine(
            WHISPER,
            "VOICEPUNCHER_WHISPER_BINARY",
            "brew install whisper-cpp",
        );
        let model = self.model(model_var, model_file);
        let vad = self.newest_vad();
        let problem = first_problem([&program, &model, &vad]);
        let service = Service { kind, binary: WHISPER, program, problem, copies };
        let cores = self.machine.cores;
        self.copies(&service, &address, probe, |port, count| {
            whisper_args(&shown(&model), port, threads(cores, count), &shown(&vad))
        })
    }

    fn command_spotting(
        &self,
        url: &str,
        commands_url: &str,
        copies: u32,
        probe: &dyn ServiceProbe,
    ) -> Vec<ServiceSpec> {
        let Ok(address) = ServiceAddress::parse(url) else {
            return Vec::new();
        };
        let address = address.with_copy_offset(COMMAND_SPOTTING_COPY_OFFSET);
        let program = self.command_spotting_program();
        let model_dir = self.kws_model_dir();
        let vad = self.model("VOICEPUNCHER_KWS_VAD_MODEL", KWS_VAD_MODEL);
        let problem = first_problem([&program, &model_dir, &vad]);
        let kind = ServiceKind::CommandSpotting;
        let service = Service { kind, binary: COMMAND_SPOTTING, program, problem, copies };
        self.copies(&service, &address, probe, |port, _| {
            command_spotting_args(port, &shown(&model_dir), &shown(&vad), commands_url)
        })
    }

    /// One spec per copy of `service`: on the ports it already answers on, else on the
    /// configured copies' ports; `args` takes a copy's port and the number of copies.
    fn copies(
        &self,
        service: &Service,
        address: &ServiceAddress,
        probe: &dyn ServiceProbe,
        args: impl Fn(u16, usize) -> Vec<String>,
    ) -> Vec<ServiceSpec> {
        let ports = running_copies(service.kind, address, probe)
            .unwrap_or_else(|| copy_ports(address, service.copies));
        labelled(service.kind, &ports)
            .map(|(port, label)| self.spec(service, label, port, args(port, ports.len())))
            .collect()
    }

    fn spec(&self, service: &Service, label: String, port: u16, args: Vec<String>) -> ServiceSpec {
        let binary = service.binary;
        ServiceSpec {
            kind: service.kind,
            label,
            program: service
                .program
                .clone()
                .unwrap_or_else(|_| PathBuf::from(binary)),
            args,
            port,
            log_file: logs_dir(&self.machine.home).join(format!("{binary}-{port}.log")),
            health_checked: true,
            problem: service.problem.clone(),
        }
    }

    /// The environment's path, the engine in the app bundle, then `$PATH`.
    fn engine(&self, name: &str, var: &str, install: &str) -> Result<PathBuf, Missing> {
        if let Some(path) = self.machine.var(var) {
            return existing(path, var);
        }
        let bundled = self
            .machine
            .app_dir
            .iter()
            .map(|dir| dir.join(ENGINES_DIR).join(engine_folder(name)).join(name));
        let on_path = self.machine.path.iter().map(|dir| dir.join(name));
        bundled
            .chain(on_path)
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| {
                format!("{name} not found in the app or on $PATH — {install} or set {var}")
            })
    }

    /// The command spotting service: the environment's path, the bundle, `$PATH`, then the
    /// build next to the running program (`target/release`, `target/debug`).
    fn command_spotting_program(&self) -> Result<PathBuf, Missing> {
        let var = "VOICEPUNCHER_KWS_BINARY";
        let hint = "build it with 'cargo build --release -p voicepuncher-command-spotting'";
        self.engine(COMMAND_SPOTTING, var, hint).or_else(|missing| {
            let app_dir = self.machine.app_dir.as_ref().ok_or(missing.clone())?;
            [
                app_dir.join(COMMAND_SPOTTING),
                app_dir.join("../release").join(COMMAND_SPOTTING),
            ]
            .into_iter()
            .find(|candidate| candidate.is_file())
            .ok_or(missing)
        })
    }

    /// The environment's path, else `file` in the models folder, then in `~/.voicepuncher/`.
    fn model(&self, var: &str, file: &str) -> Result<PathBuf, Missing> {
        if let Some(path) = self.machine.var(var) {
            return existing(path, var);
        }
        self.model_dirs
            .iter()
            .map(|dir| dir.join(file))
            .find(|candidate| candidate.exists())
            .ok_or_else(|| self.not_found(file, var))
    }

    /// The newest `ggml-silero-*.bin`, as the `justfile` picks it.
    fn newest_vad(&self) -> Result<PathBuf, Missing> {
        let var = "VOICEPUNCHER_VAD_MODEL";
        if let Some(path) = self.machine.var(var) {
            return existing(path, var);
        }
        newest(self.entries(|path| {
            path.is_file()
                && file_name_starts(path, WHISPER_VAD_PREFIX)
                && has_extension(path, "bin")
        }))
        .ok_or_else(|| self.not_found(&format!("{WHISPER_VAD_PREFIX}*.bin"), var))
    }

    /// The newest `sherpa-onnx-kws-*` folder, as the `justfile` picks it.
    fn kws_model_dir(&self) -> Result<PathBuf, Missing> {
        let var = "VOICEPUNCHER_KWS_MODEL_DIR";
        if let Some(path) = self.machine.var(var) {
            return existing(path, var);
        }
        newest(self.entries(|path| path.is_dir() && file_name_starts(path, KWS_MODEL_DIR_PREFIX)))
            .ok_or_else(|| self.not_found(&format!("{KWS_MODEL_DIR_PREFIX}*"), var))
    }

    fn entries(&self, keep: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
        self.model_dirs
            .iter()
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flatten()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| keep(path))
            .collect()
    }

    fn not_found(&self, file: &str, var: &str) -> Missing {
        let dirs: Vec<String> = self
            .model_dirs
            .iter()
            .map(|dir| dir.display().to_string())
            .collect();
        format!(
            "model {file} not found in {} — {RUN_SCRIPTS_HINT} or set {var}",
            dirs.join(", ")
        )
    }
}

/// The ports a service already answers on, its own or its copies', when
/// any does: the service scripts may run it as other copies than this configuration.
fn running_copies(
    kind: ServiceKind,
    address: &ServiceAddress,
    probe: &dyn ServiceProbe,
) -> Option<Vec<u16>> {
    let mut ports = vec![address.port()];
    ports.extend(address.copy_ports());
    let answering: Vec<u16> = ports
        .into_iter()
        .filter(|&port| probe.answer(kind, port) == ProbeAnswer::Ready)
        .collect();
    (!answering.is_empty()).then_some(answering)
}

/// `dev-tools`'s `whisper_args`, without the program.
pub fn whisper_args(model: &str, port: u16, threads: usize, vad: &str) -> Vec<String> {
    let port = port.to_string();
    let threads = threads.to_string();
    [
        "-m",
        model,
        "--host",
        "127.0.0.1",
        "--port",
        &port,
        "--inference-path",
        "/v1/audio/transcriptions",
        "-l",
        "auto",
        "-t",
        &threads,
        "-bo",
        "1",
        "-nf",
        "-sns",
        "-mc",
        "0",
        "--vad",
        "-vm",
        vad,
        "-vsd",
        "2000",
        "-vp",
        "500",
        "-nlp",
    ]
    .map(String::from)
    .to_vec()
}

/// `dev-tools`'s command spotting service line, without the program.
pub fn command_spotting_args(
    port: u16,
    model_dir: &str,
    vad: &str,
    commands_url: &str,
) -> Vec<String> {
    let port = port.to_string();
    [
        "--port",
        &port,
        "--model-dir",
        model_dir,
        "--vad-model",
        vad,
        "--transcription-url",
        commands_url,
    ]
    .map(String::from)
    .to_vec()
}

fn existing(path: PathBuf, var: &str) -> Result<PathBuf, Missing> {
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("{} not found (set by {var})", path.display()))
    }
}

fn shown(found: &Result<PathBuf, Missing>) -> String {
    found
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

fn first_problem<const N: usize>(found: [&Result<PathBuf, Missing>; N]) -> Option<String> {
    found
        .into_iter()
        .find_map(|item| item.as_ref().err().cloned())
}

/// The most recently changed of `found`; of equally recent ones, the first by name.
fn newest(found: Vec<PathBuf>) -> Option<PathBuf> {
    found.into_iter().max_by_key(|path| {
        let changed = path.metadata().and_then(|meta| meta.modified()).ok();
        (changed, std::cmp::Reverse(path.clone()))
    })
}

fn file_name_starts(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(prefix))
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension().is_some_and(|found| found == extension)
}

#[cfg(test)]
mod tests;
