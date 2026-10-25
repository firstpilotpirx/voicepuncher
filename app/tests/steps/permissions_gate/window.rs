//! The Permissions window as its rows, badges, lines and buttons show it, over a gate whose
//! statuses are the app-runtime steps' System Settings; its buttons run the window's steps,
//! which record the request, the page and the bundle they would show instead of showing them.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::adapters::primary::permissions::macos::mapping::{
    badge, button, settings_steps, RowButton, RowModel, SettingsStep, Shown, WindowModel,
    REFRESH_MS,
};
use voicepuncher::adapters::secondary::permissions::macos::MacOsPermissionProbe;
use voicepuncher::core::application::{PermissionProbe, PermissionsGate};
use voicepuncher::core::domain::{Permission, PermissionStatus};

use crate::steps::app_runtime::run;
use crate::IntegrationWorld;

/// The permissions the window asked the system for.
type Requests = Arc<Mutex<Vec<Permission>>>;

/// What a scenario of the Permissions window shares between its steps; the macOS is the
/// one a scenario names, else 0.
#[derive(Debug, Default)]
pub struct WindowView {
    responsible_app: Option<String>,
    from_bundle: bool,
    macos_major: u64,
    pending: Vec<Permission>,
    opened: Vec<Permission>,
    requests: Requests,
    /// What Open System Settings did: the pages it opened and the bundle it showed.
    pages: Vec<String>,
    bundle_shown: bool,
    closed: bool,
}

/// The scenario's System Settings, as the window's gate asks it.
struct WindowProbe {
    settings: Arc<PermissionsGate>,
    responsible_app: Option<String>,
    requests: Requests,
}

impl PermissionProbe for WindowProbe {
    fn status(&self, permission: Permission) -> PermissionStatus {
        self.settings.report().status(permission)
    }

    fn request(&self, permission: Permission) {
        self.requests.lock().unwrap().push(permission);
    }

    fn settings_url(&self, permission: Permission) -> String {
        MacOsPermissionProbe::new().settings_url(permission)
    }

    fn responsible_app(&self) -> Option<String> {
        self.responsible_app.clone()
    }
}

fn view(world: &mut IntegrationWorld) -> &mut WindowView {
    world
        .extra
        .ui
        .permissions_window
        .get_or_insert_with(WindowView::default)
}

/// The window's gate over the scenario's System Settings.
fn gate(world: &mut IntegrationWorld) -> PermissionsGate {
    let settings = run(world).permissions().gate();
    let view = view(world);
    PermissionsGate::new(Box::new(WindowProbe {
        settings,
        responsible_app: view.responsible_app.clone(),
        requests: Arc::clone(&view.requests),
    }))
}

/// The window as it shows after its latest check.
fn model(world: &mut IntegrationWorld) -> WindowModel {
    let report = gate(world).report();
    let view = view(world);
    WindowModel::new(&report, &view.pending, &[]).with_lines(&view.opened, view.macos_major)
}

fn row(world: &mut IntegrationWorld, permission: Permission) -> Result<RowModel> {
    model(world)
        .rows
        .into_iter()
        .find(|row| row.permission == permission)
        .with_context(|| format!("no {permission} row"))
}

fn named(name: &str) -> Result<Permission> {
    Permission::ALL
        .into_iter()
        .find(|p| p.name() == name)
        .with_context(|| format!("no permission is named {name}"))
}

fn set(world: &mut IntegrationWorld, permission: Permission, status: PermissionStatus) {
    run(world).permissions().set(permission, status);
}

/// Open System Settings in `permission`'s row: the steps the window runs for it.
fn open_settings(world: &mut IntegrationWorld, permission: Permission) {
    let gate = gate(world);
    let view = view(world);
    view.opened.push(permission);
    for step in settings_steps(permission, view.from_bundle) {
        match step {
            SettingsStep::Request => gate.request(permission),
            SettingsStep::OpenPane => view.pages.push(gate.settings_url(permission)),
            SettingsStep::RevealBundle => view.bundle_shown = true,
        }
    }
}

#[given("Microphone is granted")]
fn microphone_granted(world: &mut IntegrationWorld) {
    set(world, Permission::Microphone, PermissionStatus::Granted);
}

#[given("the app never asked for Input Monitoring")]
fn input_monitoring_never_asked(world: &mut IntegrationWorld) {
    set(
        world,
        Permission::InputMonitoring,
        PermissionStatus::NotDetermined,
    );
}

#[given(regex = r"^the app runs on macOS (\d+) and Accessibility is not granted$")]
fn runs_on_macos(world: &mut IntegrationWorld, major: u64) {
    view(world).macos_major = major;
    set(world, Permission::Accessibility, PermissionStatus::Denied);
}

#[given("the app runs as a bare binary started from iTerm2")]
fn runs_from_iterm(world: &mut IntegrationWorld) {
    view(world).responsible_app = Some("iTerm2".to_owned());
}

#[given("the app runs from its bundle")]
fn runs_from_bundle(world: &mut IntegrationWorld) {
    let view = view(world);
    view.responsible_app = None;
    view.from_bundle = true;
}

#[given("the Permissions window is open with Accessibility not granted")]
fn open_without_accessibility(world: &mut IntegrationWorld) {
    set(world, Permission::Accessibility, PermissionStatus::Denied);
}

#[given("all three permissions are granted and the window is open")]
fn open_with_every_permission(world: &mut IntegrationWorld) {
    for permission in Permission::ALL {
        set(world, permission, PermissionStatus::Granted);
    }
    run(world).launch();
}

#[given("the user pressed Open System Settings in the Accessibility row while Accessibility is not granted")]
fn pressed_settings_for_accessibility(world: &mut IntegrationWorld) {
    set(world, Permission::Accessibility, PermissionStatus::Denied);
    open_settings(world, Permission::Accessibility);
}

#[given("Input Monitoring is not granted and the app runs from its bundle")]
fn input_monitoring_denied_in_bundle(world: &mut IntegrationWorld) {
    set(world, Permission::InputMonitoring, PermissionStatus::Denied);
    runs_from_bundle(world);
}

#[when(regex = r"^the user presses Open System Settings in the (Microphone|Input Monitoring) row$")]
fn presses_open_settings(world: &mut IntegrationWorld, name: String) -> Result<()> {
    open_settings(world, named(&name)?);
    Ok(())
}

#[when("the user presses Request… in the Input Monitoring row")]
fn presses_request(world: &mut IntegrationWorld) {
    let gate = gate(world);
    view(world).pending.push(Permission::InputMonitoring);
    gate.request(Permission::InputMonitoring);
}

#[when("the Permissions window shows the Accessibility row")]
#[when("the user looks at the footer")]
#[when("the Permissions window checks again")]
fn window_checks(_world: &mut IntegrationWorld) {
    // every Then reads the window as its next check shows it
}

#[when("the user presses Continue")]
fn presses_continue(world: &mut IntegrationWorld) {
    let enabled = model(world).continue_enabled;
    view(world).closed = enabled;
}

#[then(
    regex = r"^System Settings opens on the (Microphone|Input Monitoring) page(?: of Privacy & Security)?$"
)]
fn settings_page_opened(world: &mut IntegrationWorld, name: String) -> Result<()> {
    let permission = named(&name)?;
    let shown = row(world, permission)?.shown;
    if button(shown) != Some(RowButton::OpenSystemSettings) {
        bail!("the {name} row offers {:?}", button(shown));
    }
    let page = MacOsPermissionProbe::new().settings_url(permission);
    let pages = view(world).pages.clone();
    if pages != [page] || !pages[0].contains("Privacy_") {
        bail!("System Settings opened {pages:?}");
    }
    Ok(())
}

#[then(regex = r"^the system (?:asks the user|is asked) for Input Monitoring(?: again)?$")]
fn input_monitoring_requested(world: &mut IntegrationWorld) -> Result<()> {
    let requests = view(world).requests.lock().unwrap().clone();
    if requests != [Permission::InputMonitoring] {
        bail!("the system was asked for {requests:?}");
    }
    Ok(())
}

#[then("the Input Monitoring row shows Checking… until the status is known")]
fn checking_until_known(world: &mut IntegrationWorld) -> Result<()> {
    let asking = row(world, Permission::InputMonitoring)?;
    if badge(asking.shown).0 != "Checking…" || button(asking.shown).is_some() {
        bail!("while asking the row shows {:?}", asking.shown);
    }
    set(world, Permission::InputMonitoring, PermissionStatus::Denied);
    let answered = row(world, Permission::InputMonitoring)?;
    if answered.shown != Shown::Status(PermissionStatus::Denied) {
        bail!("once answered the row shows {:?}", answered.shown);
    }
    Ok(())
}

#[then("the Microphone row shows Granted and no button")]
fn microphone_granted_without_button(world: &mut IntegrationWorld) -> Result<()> {
    let shown = row(world, Permission::Microphone)?.shown;
    if badge(shown).0 != "Granted" || button(shown).is_some() {
        bail!(
            "the Microphone row shows {shown:?} with {:?}",
            button(shown)
        );
    }
    Ok(())
}

#[then(expr = "the row names the pane {string}")]
fn row_names_pane(world: &mut IntegrationWorld, pane: String) -> Result<()> {
    let line = row(world, Permission::Accessibility)?.line;
    if line != format!("Switch voicepuncher on in {pane}") {
        bail!("the Accessibility row reads {line:?}");
    }
    Ok(())
}

#[then("it says voicepuncher runs from iTerm2 and the permissions must be granted to iTerm2")]
fn names_iterm(world: &mut IntegrationWorld) -> Result<()> {
    let line = model(world).developer_line;
    let wanted = "voicepuncher runs from iTerm2 — grant the permissions to iTerm2.";
    if line.as_deref() != Some(wanted) {
        bail!("the window says {line:?}");
    }
    Ok(())
}

#[then("it names no other application to grant the permissions to")]
fn names_no_other_app(world: &mut IntegrationWorld) -> Result<()> {
    if let Some(line) = model(world).developer_line {
        bail!("the window says {line:?}");
    }
    Ok(())
}

#[then("within 1 s the Accessibility row shows Granted")]
fn accessibility_granted_soon(world: &mut IntegrationWorld) -> Result<()> {
    if REFRESH_MS > 1_000 {
        bail!("the window checks only every {REFRESH_MS} ms");
    }
    let shown = row(world, Permission::Accessibility)?.shown;
    if shown != Shown::Status(PermissionStatus::Granted) {
        bail!("the Accessibility row shows {shown:?}");
    }
    Ok(())
}

#[then("Continue is disabled")]
fn continue_disabled(world: &mut IntegrationWorld) -> Result<()> {
    if model(world).continue_enabled {
        bail!("Continue is enabled");
    }
    Ok(())
}

#[then("the window closes and the dictation pipeline is running")]
fn closes_with_pipeline(world: &mut IntegrationWorld) -> Result<()> {
    if !view(world).closed {
        bail!("the window stayed open");
    }
    run(world).assembled(1)
}

#[then("the Accessibility row says to remove voicepuncher with − and add it again with +")]
fn hint_to_add_again(world: &mut IntegrationWorld) -> Result<()> {
    let line = row(world, Permission::Accessibility)?.line;
    if !line.contains("Remove voicepuncher with −") || !line.contains("add it again with +") {
        bail!("the Accessibility row reads {line:?}");
    }
    Ok(())
}

#[then("the Accessibility row shows no hint")]
fn no_hint(world: &mut IntegrationWorld) -> Result<()> {
    let line = row(world, Permission::Accessibility)?.line;
    if line.contains('+') {
        bail!("the Accessibility row reads {line:?}");
    }
    Ok(())
}

#[then("Finder shows the voicepuncher bundle")]
fn bundle_shown(world: &mut IntegrationWorld) -> Result<()> {
    if !view(world).bundle_shown {
        bail!("Finder was not asked to show the bundle");
    }
    Ok(())
}

#[then("the Input Monitoring row says to add voicepuncher with + when it is not in the list")]
fn hint_to_add(world: &mut IntegrationWorld) -> Result<()> {
    let line = row(world, Permission::InputMonitoring)?.line;
    if line != "Not listed? Add voicepuncher with +." {
        bail!("the Input Monitoring row reads {line:?}");
    }
    Ok(())
}
