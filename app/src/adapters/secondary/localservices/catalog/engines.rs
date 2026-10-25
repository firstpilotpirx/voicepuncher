//! Where each engine comes from and its version.

use super::{HashMap, Path, PathBuf, ENGINES_DIR, VERSION_FILE, WHISPER};

/// The engine folder inside the bundle that holds a program.
pub(super) fn engine_folder(program: &str) -> &'static str {
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

    pub(crate) fn var(&self, name: &str) -> Option<PathBuf> {
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
/// `$PATH`, else the build folder — and its version.
///
/// The version file of its folder, else the version folder of a Homebrew installation
/// (`Cellar/<formula>/<version>/…`) it links to.
pub fn engine_origin(program: &Path, machine: &Machine) -> EngineOrigin {
    EngineOrigin {
        program: program
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        version: folder_version(program).or_else(|| cellar_version(program)),
        source: engine_source(program, machine),
    }
}

/// Where `program` was found: a variable naming it, the bundle, the `PATH` or the build folder.
fn engine_source(program: &Path, machine: &Machine) -> EngineSource {
    let named_by = machine
        .env
        .iter()
        .filter(|(_, value)| Path::new(value.trim()) == program)
        .map(|(name, _)| name.clone())
        .min();
    if let Some(var) = named_by {
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
    }
}

pub(super) fn folder_version(program: &Path) -> Option<String> {
    let text = std::fs::read_to_string(program.with_file_name(VERSION_FILE)).ok()?;
    Some(text.trim().to_string()).filter(|version| !version.is_empty())
}

pub(super) fn cellar_version(program: &Path) -> Option<String> {
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
