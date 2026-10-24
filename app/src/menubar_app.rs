//! The menu bar app of hotkey mode: the status item and the windows around the runtime,
//! alive until Quit ends the process.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

use voicepuncher::adapters::primary::cli::Cli;
use voicepuncher::adapters::primary::menubar::macos::{
    actions::{MacosMenuActions, MenuActions},
    applier::RuntimeApplier,
    indicator::StatusItemCell,
    login_offer::LoginOfferObserver,
    status_item::MacosStatusItem,
    system_quit::SystemQuit,
};
use voicepuncher::adapters::primary::modeldownload::macos::{
    self as modeldownload, DownloadSource, DownloadWindow,
};
use voicepuncher::adapters::primary::permissions::macos::{self as permissions, PermissionsWindow};
use voicepuncher::adapters::primary::ptt::eventtap::EventTapHotkeyListener;
use voicepuncher::adapters::primary::settings::macos::{
    login::LoginOffer,
    models_row::ModelsPanel,
    session::{SessionParts, SettingsSession},
    window::SettingsWindow,
};
use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::adapters::secondary::logging::file::logs_dir;
use voicepuncher::adapters::secondary::loginitem::macos::{launch, MacOsLoginItem};
use voicepuncher::adapters::secondary::permissions::macos::MacOsPermissionProbe;
use voicepuncher::adapters::secondary::recording::cpal::devices::CpalInputDevices;
use voicepuncher::adapters::secondary::visualfeedback::macos::{
    MacosPointerSign, MacosVisualIndicator,
};
use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::{
    PointerSignIndicator, FAILURE_NOTICE_LIFETIME,
};
use voicepuncher::config::Config;
use voicepuncher::core::application::{LoginItem, ModelStore, PermissionsGate};
use voicepuncher::core::domain::model_catalog;
use voicepuncher::runtime::models::needs_of;
use voicepuncher::runtime::services::SystemClock;
use voicepuncher::runtime::wiring::WiringAssembler;
use voicepuncher::runtime::{
    signals, CatalogModels, LogObserver, ManagedServices, ModelsGate, PermissionsWatch, Runtime,
    ServicesView, SharedVisualIndicator,
};

/// The menu bar app: the runtime under the status item until Quit, which ends the process.
pub(crate) fn run(cli: Cli, config: Config, config_path: PathBuf) -> anyhow::Result<ExitCode> {
    let services = start_services(&config)?;
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| anyhow::anyhow!("main must run on the main thread"))?;
    let app = NSApplication::sharedApplication(mtm);
    launch::log_launch_origin(mtm);
    let download = services
        .is_some()
        .then(|| download_source(&config_path, &cli));
    let shell = Shell::install(mtm, download);
    let (mut runtime, services_view) = shell.assemble_runtime(mtm, services)?;
    let login_item: Arc<dyn LoginItem> = Arc::new(MacOsLoginItem::of_this_process());
    shell.observe(&runtime, &login_item, &config_path);
    if runtime.start(config).is_err() {
        // the log observer logged the reason with the state Failed
        return Ok(ExitCode::from(2));
    }
    let settings = settings_window(&runtime, &shell, cli, config_path, &login_item)?;
    let _system_quit = shell.install_menu(mtm, runtime, settings, services_view);
    app.run();
    Ok(ExitCode::SUCCESS)
}

/// The services the app manages, started; when it leaves them to the scripts, a missing
/// service ends the launch at once, before the app waits for any permission.
fn start_services(config: &Config) -> anyhow::Result<Option<ManagedServices>> {
    let services = config
        .services
        .manage
        .then(ManagedServices::start)
        .transpose()?;
    if services.is_none() && config.voice_control.enabled {
        SpottingClient::connect(&config.voice_control.url, std::time::Instant::now())?;
    }
    Ok(services)
}

/// The status item and the windows of the menu bar app, alive until the process ends.
struct Shell {
    status_item: MacosStatusItem,
    status_cell: StatusItemCell,
    gate: Arc<PermissionsGate>,
    permissions_window: PermissionsWindow,
    download_window: Option<DownloadWindow>,
}

impl Shell {
    /// Installs the status item, the Permissions window and, given a `download`, the
    /// Download window.
    fn install(mtm: MainThreadMarker, download: Option<DownloadSource>) -> Self {
        let status_item = MacosStatusItem::install(mtm);
        let status_cell =
            StatusItemCell::new(Box::new(status_item.icon()), FAILURE_NOTICE_LIFETIME);
        status_item.connect(status_cell.clone());
        let gate = Arc::new(PermissionsGate::new(Box::new(MacOsPermissionProbe)));
        let permissions_window = PermissionsWindow::install(mtm, Arc::clone(&gate));
        let download_window = download.map(|source| DownloadWindow::install(mtm, source));
        Self { status_item, status_cell, gate, permissions_window, download_window }
    }

    /// The runtime under this shell's overlay and permissions gate, with the services it
    /// manages and their view, if any.
    fn assemble_runtime(
        &self,
        mtm: MainThreadMarker,
        services: Option<ManagedServices>,
    ) -> anyhow::Result<(Runtime, Option<Arc<dyn ServicesView>>)> {
        let level_register = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let overlay = SharedVisualIndicator::new(Box::new(self.status_cell.indicator(
            PointerSignIndicator::new(
                MacosVisualIndicator::initialize(mtm, Arc::clone(&level_register)),
                MacosPointerSign::initialize(mtm),
            ),
        )));
        let assembler = WiringAssembler::new(overlay, level_register);
        // a missing permission makes the runtime wait for it, not the process exit
        let runtime = Runtime::new(Box::new(assembler), Box::new(EventTapHotkeyListener::new()))
            .with_permissions(PermissionsWatch::new(Arc::clone(&self.gate)));
        let Some(services) = services else {
            return Ok((runtime, None));
        };
        let view = services.view();
        signals::stop_on_termination(services.stopper())?;
        // the services start only once the models they load are on disk
        let runtime = runtime
            .with_models(Box::new(CatalogModels::current()))
            .with_services(Box::new(services));
        Ok((runtime, Some(view)))
    }

    fn observe(&self, runtime: &Runtime, login_item: &Arc<dyn LoginItem>, config_path: &Path) {
        if let Some(window) = &self.download_window {
            runtime.observe(Box::new(window.observer()));
        }
        runtime.observe(Box::new(LogObserver));
        runtime.observe(Box::new(self.status_cell.observer()));
        runtime.observe(Box::new(LoginOfferObserver::new(LoginOffer::new(
            Arc::clone(login_item),
            config_path.to_path_buf(),
        ))));
        runtime.observe(Box::new(self.permissions_window.observer()));
    }

    /// Hands the menu its actions; Quit, in the menu or from the system, stops the runtime
    /// and ends the process.
    fn install_menu(
        &self,
        mtm: MainThreadMarker,
        runtime: Runtime,
        settings: SettingsWindow,
        services_view: Option<Arc<dyn ServicesView>>,
    ) -> Retained<SystemQuit> {
        let mut actions = MacosMenuActions::new(runtime, Box::new(settings), logs_folder())
            .with_permissions_window(Box::new(permissions::bring_to_front));
        if let Some(view) = services_view {
            actions = actions
                .with_services(view)
                .with_download_window(Box::new(modeldownload::bring_to_front));
        }
        let actions = Rc::new(actions);
        self.status_item.set_actions(Box::new(Rc::clone(&actions)));
        if let Some(window) = &self.download_window {
            let actions = Rc::clone(&actions);
            window.set_quit(Box::new(move || actions.quit()));
        }
        let system_quit = SystemQuit::install(mtm, Rc::clone(&actions) as _);
        self.permissions_window
            .set_quit(Box::new(move || actions.quit()));
        system_quit
    }
}

/// The settings window over the configuration file; Save applies through the runtime with
/// this run's command line over the saved settings, as a hang-up signal does.
fn settings_window(
    runtime: &Runtime,
    shell: &Shell,
    cli: Cli,
    config_path: PathBuf,
    login_item: &Arc<dyn LoginItem>,
) -> anyhow::Result<SettingsWindow> {
    let reloader = runtime.reloader()?;
    let (path, cli_of_opener, applier) = (config_path.clone(), cli.clone(), reloader.clone());
    let home = crate::home();
    let opener = move || {
        let cli = cli_of_opener.clone();
        let parts = SessionParts {
            overlay: Box::new(move |cfg| cli.overlay(cfg)),
            applier: Box::new(RuntimeApplier::new(applier.clone())),
            devices: Box::new(CpalInputDevices),
        };
        SettingsSession::open(&path, &home, parts)
    };
    let mut settings = SettingsWindow::new(Box::new(opener), logs_folder(), Arc::clone(login_item));
    if shell.download_window.is_some() {
        let models = SavedModels { config_path: config_path.clone(), cli: cli.clone() };
        settings = settings.with_models(Rc::new(models));
    }
    let overrides = Box::new(move |cfg: &mut Config| cli.overlay(cfg));
    signals::reload_on_hang_up(config_path, overrides, reloader)?;
    Ok(settings)
}

/// The saved configuration with this run's command line over it; the defaults when it
/// cannot be read.
fn saved_config(config_path: &Path, cli: &Cli) -> Config {
    let mut config = voicepuncher::config::load(config_path).unwrap_or_else(|e| {
        log::warn!("models: config not read, its defaults taken: {e:#}");
        Config::default()
    });
    cli.overlay(&mut config);
    config
}

/// The Download window's download: the files the saved configuration needs, into its
/// models folder.
fn download_source(config_path: &Path, cli: &Cli) -> modeldownload::DownloadSource {
    let (config_path, cli) = (config_path.to_path_buf(), cli.clone());
    Box::new(move || {
        let config = saved_config(&config_path, &cli);
        let models = CatalogModels::current();
        let folder = models.folder(&config);
        let download = models.download(folder.clone(), needs_of(&config), Arc::new(SystemClock));
        (download, folder)
    })
}

/// The Models row of the settings window over the saved configuration.
struct SavedModels {
    config_path: PathBuf,
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

    fn folder(&self) -> PathBuf {
        CatalogModels::current().folder(&saved_config(&self.config_path, &self.cli))
    }

    fn open_download(&self) {
        modeldownload::bring_to_front();
    }
}

/// The folder the services' logs live in: `~/.voicepuncher/logs`.
fn logs_folder() -> PathBuf {
    logs_dir(&crate::home())
}
