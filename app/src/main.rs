use clap::Parser;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

use voicepuncher::adapters::primary::cli::Cli;
use voicepuncher::adapters::primary::menubar::macos::actions::{MacosMenuActions, MenuActions};
use voicepuncher::adapters::primary::menubar::macos::applier::RuntimeApplier;
use voicepuncher::adapters::primary::menubar::macos::indicator::StatusItemCell;
use voicepuncher::adapters::primary::menubar::macos::login_offer::LoginOfferObserver;
use voicepuncher::adapters::primary::menubar::macos::status_item::MacosStatusItem;
use voicepuncher::adapters::primary::modeldownload::macos::{
    self as modeldownload, DownloadWindow,
};
use voicepuncher::adapters::primary::permissions::macos::{self as permissions, PermissionsWindow};
use voicepuncher::adapters::primary::ptt::rdev::RdevHotkeyListener;
use voicepuncher::adapters::primary::settings::macos::login::LoginOffer;
use voicepuncher::adapters::primary::settings::macos::models_row::ModelsPanel;
use voicepuncher::adapters::primary::settings::macos::session::{SessionParts, SettingsSession};
use voicepuncher::adapters::primary::settings::macos::window::SettingsWindow;
use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::adapters::secondary::logging::file::{
    install_panic_hook, launch_header, logs_dir, terminal_stderr, RotatingFileWriter, TeeWriter,
    KEPT_BACKUPS, LOG_FILE_NAME, MAX_LOG_BYTES,
};
use voicepuncher::adapters::secondary::loginitem::macos::launch;
use voicepuncher::adapters::secondary::loginitem::macos::MacOsLoginItem;
use voicepuncher::adapters::secondary::permissions::macos::MacOsPermissionProbe;
use voicepuncher::adapters::secondary::recording::cpal::devices::CpalInputDevices;
use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use voicepuncher::adapters::secondary::textpasting::stdio::StdioTextPaster;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::adapters::secondary::visualfeedback::macos::{
    MacosPointerSign, MacosVisualIndicator,
};
use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::{
    PointerSignIndicator, FAILURE_NOTICE_LIFETIME,
};
use voicepuncher::config::{Config, FileOutcome};
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::visual_indicator::VisualIndicator;
use voicepuncher::core::application::ModelStore;
use voicepuncher::core::application::{LoginItem, PermissionsGate, TextRefiner};
use voicepuncher::core::domain::{model_catalog, FailureNotice, ModelNeeds};
use voicepuncher::runtime::models::needs_of;
use voicepuncher::runtime::services::SystemClock;
use voicepuncher::runtime::wiring::{self, WiringAssembler};
use voicepuncher::runtime::{
    signals, CatalogModels, LogObserver, ManagedServices, ModelsGate, PermissionsWatch, Runtime,
    SharedVisualIndicator,
};

struct NoopVisualIndicator;

impl VisualIndicator for NoopVisualIndicator {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, _notice: &FailureNotice) {}
}

#[expect(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "size baseline"
)]
fn main() {
    install_panic_hook();
    let cli = Cli::parse();

    let config_path = cli.config.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".voicepuncher/config.toml")
    });

    if cli.download_models {
        std::process::exit(download_models(&cli, &config_path));
    }

    // a first launch writes the defaults; the settings window changes them, nothing is asked
    let config_file = voicepuncher::config::ensure_exists(&config_path).unwrap_or_else(|e| {
        eprintln!("config error: {e:#}");
        std::process::exit(2);
    });

    let mut config = voicepuncher::config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("config error: {e:#}");
        std::process::exit(2);
    });

    let input_file = cli.input_file.clone();
    let archives_file_mode = cli.archives_file_mode();
    cli.overlay(&mut config);
    voicepuncher::config::validate(&config).unwrap_or_else(|e| {
        eprintln!("config error: {e:#}");
        std::process::exit(2);
    });

    let level = config
        .logging
        .level
        .parse()
        .unwrap_or(log::LevelFilter::Info);
    if input_file.is_some() {
        init_stderr_logger(level);
    } else {
        init_file_logger(level);
    }
    log::info!("{}", launch_header(&config_path));
    if config_file == FileOutcome::Created {
        log::info!(
            "config: created {} with the defaults",
            config_path.display()
        );
    }
    if config.audio.debug_save_wav {
        log::warn!(
            "[audio] debug_save_wav is retired: the dictation archive ([archive]) now keeps \
             every recording; remove the key from the config"
        );
    }

    if let Some(ref input_path) = input_file {
        run_file_mode(input_path, &config, archives_file_mode);
        return;
    }

    // the app starts the services itself and waits for them; when it leaves them to the
    // scripts, a missing service ends the launch at once, before the app waits for any permission
    let services = config.services.manage.then(|| {
        ManagedServices::start().unwrap_or_else(|e| {
            log::error!("services: {e:#}");
            std::process::exit(2);
        })
    });
    if services.is_none() && config.voice_control.enabled {
        if let Err(e) =
            SpottingClient::connect(&config.voice_control.url, std::time::Instant::now())
        {
            log::error!("{e:#}");
            std::process::exit(2);
        }
    }

    let mtm = MainThreadMarker::new().expect("main must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    launch::log_launch_origin(mtm);
    let status_item = MacosStatusItem::install(mtm);
    let status_cell = StatusItemCell::new(Box::new(status_item.icon()), FAILURE_NOTICE_LIFETIME);
    status_item.connect(status_cell.clone());
    let level_register = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let overlay =
        SharedVisualIndicator::new(Box::new(status_cell.indicator(PointerSignIndicator::new(
            MacosVisualIndicator::initialize(mtm, std::sync::Arc::clone(&level_register)),
            MacosPointerSign::initialize(mtm),
        ))));
    let assembler = WiringAssembler::new(overlay, level_register);
    let gate = std::sync::Arc::new(PermissionsGate::new(Box::new(MacOsPermissionProbe)));
    let permissions_window = PermissionsWindow::install(mtm, std::sync::Arc::clone(&gate));
    // a missing permission makes the runtime wait for it, not the process exit
    let mut runtime = Runtime::new(Box::new(assembler), Box::new(RdevHotkeyListener::new()))
        .with_permissions(PermissionsWatch::new(gate));
    let mut services_view = None;
    if let Some(services) = services {
        services_view = Some(services.view());
        signals::stop_on_termination(services.stopper()).unwrap_or_else(|e| {
            log::error!("{e:#}");
            std::process::exit(2);
        });
        // the services start only once the models they load are on disk
        runtime = runtime
            .with_models(Box::new(CatalogModels::current()))
            .with_services(Box::new(services));
    }
    let download_window = services_view
        .is_some()
        .then(|| DownloadWindow::install(mtm, download_source(&config_path, &cli)));
    if let Some(window) = &download_window {
        runtime.observe(Box::new(window.observer()));
    }
    runtime.observe(Box::new(LogObserver));
    runtime.observe(Box::new(status_cell.observer()));
    let login_item: std::sync::Arc<dyn LoginItem> =
        std::sync::Arc::new(MacOsLoginItem::of_this_process());
    runtime.observe(Box::new(LoginOfferObserver::new(LoginOffer::new(
        std::sync::Arc::clone(&login_item),
        config_path.clone(),
    ))));
    runtime.observe(Box::new(permissions_window.observer()));
    if runtime.start(config).is_err() {
        // the log observer logged the reason with the state Failed
        std::process::exit(2);
    }
    let reloader = runtime
        .reloader()
        .expect("the runtime runs after its start");
    let mut settings = settings_window(&config_path, &cli, &reloader, &login_item);
    if download_window.is_some() {
        settings = settings.with_models(std::rc::Rc::new(SavedModels {
            config_path: config_path.clone(),
            cli: cli.clone(),
        }));
    }
    signals::reload_on_hang_up(config_path, Box::new(move |cfg| cli.overlay(cfg)), reloader)
        .unwrap_or_else(|e| {
            log::error!("{e:#}");
            std::process::exit(2);
        });
    // Quit in the menu stops the runtime and ends the process; run() never returns.
    let mut actions = MacosMenuActions::new(runtime, Box::new(settings), logs_folder())
        .with_permissions_window(Box::new(permissions::bring_to_front));
    if let Some(view) = services_view {
        actions = actions
            .with_services(view)
            .with_download_window(Box::new(modeldownload::bring_to_front));
    }
    let actions = std::rc::Rc::new(actions);
    status_item.set_actions(Box::new(std::rc::Rc::clone(&actions)));
    if let Some(window) = &download_window {
        let actions = std::rc::Rc::clone(&actions);
        window.set_quit(Box::new(move || actions.quit()));
    }
    permissions_window.set_quit(Box::new(move || actions.quit()));

    app.run();
}

/// The settings window over the configuration file; Save applies through `reloader` with
/// this run's command line over the saved settings.
fn settings_window(
    config_path: &std::path::Path,
    cli: &Cli,
    reloader: &voicepuncher::runtime::Reloader,
    login_item: &std::sync::Arc<dyn LoginItem>,
) -> SettingsWindow {
    let (config_path, cli, reloader) = (config_path.to_path_buf(), cli.clone(), reloader.clone());
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let opener = move || {
        let cli = cli.clone();
        let parts = SessionParts {
            overlay: Box::new(move |cfg| cli.overlay(cfg)),
            applier: Box::new(RuntimeApplier::new(reloader.clone())),
            devices: Box::new(CpalInputDevices),
        };
        SettingsSession::open(&config_path, &home, parts)
    };
    SettingsWindow::new(
        Box::new(opener),
        logs_folder(),
        std::sync::Arc::clone(login_item),
    )
}

/// The saved configuration with this run's command line over it; the defaults when it
/// cannot be read.
fn saved_config(config_path: &std::path::Path, cli: &Cli) -> Config {
    let mut config = voicepuncher::config::load(config_path).unwrap_or_else(|e| {
        log::warn!("models: config not read, its defaults taken: {e:#}");
        Config::default()
    });
    cli.overlay(&mut config);
    config
}

/// The Download window's download: the files the saved configuration needs, into its
/// models folder.
fn download_source(config_path: &std::path::Path, cli: &Cli) -> modeldownload::DownloadSource {
    let (config_path, cli) = (config_path.to_path_buf(), cli.clone());
    Box::new(move || {
        let config = saved_config(&config_path, &cli);
        let models = CatalogModels::current();
        let folder = models.folder(&config);
        let download = models.download(
            folder.clone(),
            needs_of(&config),
            std::sync::Arc::new(SystemClock),
        );
        (download, folder)
    })
}

/// The Models row of the settings window over the saved configuration.
struct SavedModels {
    config_path: std::path::PathBuf,
    cli: Cli,
}

impl ModelsPanel for SavedModels {
    fn count(&self) -> (usize, usize) {
        let config = saved_config(&self.config_path, &self.cli);
        let store = CatalogModels::current().store(&config);
        let needed = model_catalog::needed(needs_of(&config));
        let present = needed.iter().filter(|file| store.present(file)).count();
        (present, needed.len())
    }

    fn folder(&self) -> std::path::PathBuf {
        CatalogModels::current().folder(&saved_config(&self.config_path, &self.cli))
    }

    fn open_download(&self) {
        modeldownload::bring_to_front();
    }
}

/// The folder the services' logs live in: `~/.voicepuncher/logs`.
fn logs_folder() -> std::path::PathBuf {
    logs_dir(&dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")))
}

/// `--download-models`: the models folder of the command line, else of the configuration
/// file when there is one; every catalog file but the text refinement model with `--no-llm`.
fn download_models(cli: &Cli, config_path: &std::path::Path) -> i32 {
    let mut config = if config_path.exists() {
        voicepuncher::config::load(config_path).unwrap_or_else(|e| {
            eprintln!("config error: {e:#}");
            std::process::exit(2);
        })
    } else {
        Config::default()
    };
    if let Some(dir) = &cli.models_dir {
        config.services.models_dir = dir.clone();
    }
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let needs = ModelNeeds { voice_control: true, refinement: !cli.no_llm };
    voicepuncher::runtime::models::download_headless(
        &CatalogModels::current(),
        config.services.resolved_models_dir(&home),
        needs,
        std::sync::Arc::new(SystemClock),
        &mut std::io::stderr(),
    )
}

/// Transcribes one WAV file and prints the text; archives it only when a folder was named.
fn run_file_mode(input_path: &std::path::Path, config: &Config, archives: bool) {
    // the recording is read before any service is asked, so a bad file ends with 1
    let audio = FileSystemAudioRecorder::load(input_path).unwrap_or_else(|e| {
        log::error!("file mode: {e:#}");
        std::process::exit(1);
    });
    let transcriber = connect_transcriber(config);
    let refiner = connect_refiner(config);
    let mut pipeline = Pipeline::new(
        Box::new(FileSystemAudioRecorder::new(input_path.to_path_buf())),
        Box::new(transcriber),
        refiner,
        Box::new(StdioTextPaster::new()),
        Box::new(MacosSoundPlayer::new("", "", "")),
        Box::new(NoopVisualIndicator),
        PipelineConfig {
            min_press_ms: config.hotkey.min_press_ms,
            recording_mode: config.hotkey.recording_mode,
            chunking: wiring::chunking_cfg(&config.transcription),
        },
    );
    if !config.refinement.enabled {
        pipeline = pipeline.without_text_refinement_stage();
    }
    if archives {
        pipeline = pipeline.with_archive(Box::new(wiring::connect_archive(config)));
    }
    pipeline.run_once(audio).unwrap_or_else(|e| {
        log::error!("file mode failed: {e:#}");
        std::process::exit(1);
    });
}

fn connect_transcriber(config: &Config) -> OpenAiTranscriber {
    wiring::connect_transcriber(config).unwrap_or_else(|e| {
        log::error!("{e:#}");
        std::process::exit(2);
    })
}

fn connect_refiner(config: &Config) -> Box<dyn TextRefiner> {
    wiring::connect_refiner(config).unwrap_or_else(|e| {
        log::error!("{e:#}");
        std::process::exit(2);
    })
}

/// File mode is a command line tool: its log goes to the error output only.
fn init_stderr_logger(level: log::LevelFilter) {
    env_logger::Builder::new()
        .filter_level(level)
        .format(log_line)
        .init();
}

/// Hotkey mode: the log file in the logs folder, plus the terminal when there is one; the
/// error output alone when the file cannot be opened.
fn init_file_logger(level: log::LevelFilter) {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let path = logs_dir(&home).join(LOG_FILE_NAME);
    let file = match RotatingFileWriter::open(&path, MAX_LOG_BYTES, KEPT_BACKUPS) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("log file {} cannot be opened: {e}", path.display());
            return init_stderr_logger(level);
        }
    };
    let tee = TeeWriter::new(file, terminal_stderr());
    env_logger::Builder::new()
        .filter_level(level)
        .format(log_line)
        .target(env_logger::Target::Pipe(Box::new(tee)))
        .init();
}

/// One log line with the thread that wrote it, so a session traces across the pipeline,
/// command spotting and transcription threads: `[time LEVEL thread target] message`.
fn log_line(buf: &mut env_logger::fmt::Formatter, record: &log::Record) -> std::io::Result<()> {
    use std::io::Write;
    let thread = std::thread::current();
    writeln!(
        buf,
        "[{} {:<5} {} {}] {}",
        buf.timestamp(),
        record.level(),
        thread.name().unwrap_or("-"),
        record.target(),
        record.args()
    )
}
