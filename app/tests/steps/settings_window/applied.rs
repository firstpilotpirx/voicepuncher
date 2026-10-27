//! Then steps: what the window shows and applies.

use super::{
    app_runtime, ensure, files, settings, status_text, then, when, ApplyStatus, Choices, Config,
    Context, HotKey, IntegrationWorld, Result, Value, FIRST_HOTKEY, MICROPHONES, SECOND_HOTKEY,
    SECOND_START_SOUND, SILENT_LOCAL_SERVICE,
};

#[then("the hotkey control shows Right Command")]
pub(super) fn shows_right_command(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("hotkey", "key")?;
    ensure!(shown == HotKey::RightCommand.human_name(), "shows {shown}");
    Ok(())
}

#[then("the minimum press control shows the default length")]
pub(super) fn shows_default_press(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("hotkey", "min_press_ms")?;
    let default = Config::default().hotkey.min_press_ms.to_string();
    ensure!(shown == default, "shows {shown}, the default is {default}");
    Ok(())
}

#[then("every option has exactly one control")]
pub(super) fn every_option_has_a_control(_world: &mut IntegrationWorld) -> Result<()> {
    files::every_key_has_one_field()
}

#[when("the controls of the settings window are listed")]
pub(super) fn list_controls(_world: &mut IntegrationWorld) {
    // the Then step reads the field table
}

#[then("none of them edits whether the login question was asked")]
pub(super) fn no_control_for_the_login_question(_world: &mut IntegrationWorld) -> Result<()> {
    files::no_field_edits_the_app_s_records()
}

#[then("the microphone choice offers the system default and both devices by name")]
pub(super) fn microphone_choice(world: &mut IntegrationWorld) -> Result<()> {
    let offered = settings(world).session()?.options(Choices::Microphones);
    let titles: Vec<&str> = offered.iter().map(|c| c.title.as_str()).collect();
    ensure!(
        titles == ["System default", MICROPHONES[0], MICROPHONES[1]],
        "{titles:?}"
    );
    Ok(())
}

#[then("the configuration file holds the second hotkey and the second start sound")]
pub(super) fn file_holds_second(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&settings(world).path())?;
    ensure!(saved.hotkey.key == SECOND_HOTKEY, "{:?}", saved.hotkey.key);
    ensure!(saved.sounds.start_sound == SECOND_START_SOUND);
    Ok(())
}

#[then("the switch \"Confirm commands with Whisper\" is on")]
pub(super) fn confirmation_switch_on(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("voice_control", "confirm_commands")?;
    ensure!(shown == "On", "shows {shown}");
    Ok(())
}

#[then("the configuration file has commands carried out without the Whisper confirmation")]
pub(super) fn file_holds_unconfirmed(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&settings(world).path())?;
    ensure!(!saved.voice_control.confirm_commands);
    Ok(())
}

#[then("the settings say the settings are applied")]
pub(super) fn says_applied(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let status = run.wait_for_status(|s| *s == ApplyStatus::Applied)?;
    ensure!(
        status_text(&status) == "Applied",
        "{}",
        status_text(&status)
    );
    Ok(())
}

#[then("once the dictation is pasted the settings say the settings are applied")]
pub(super) fn says_applied_after_the_paste(world: &mut IntegrationWorld) -> Result<()> {
    app_runtime::restarted_after_the_dictation(app_runtime::run(world))?;
    says_applied(world)?;
    let pastes = *settings(world).pastes_when_applied.lock().unwrap();
    ensure!(
        pastes == Some(1),
        "the settings were said to be applied after {pastes:?} pastes"
    );
    Ok(())
}

#[then("the next dictation starts with the second hotkey and plays the second start sound")]
pub(super) fn next_dictation_second(world: &mut IntegrationWorld) -> Result<()> {
    let app = app_runtime::run(world);
    let sound = app
        .newest_start_sound()
        .context("no pipeline was assembled")?;
    ensure!(sound == SECOND_START_SOUND, "plays {sound}");
    ensure!(
        app.hotkey_records(SECOND_HOTKEY),
        "the second hotkey started no recording"
    );
    app.finish_dictation(SECOND_HOTKEY)
}

#[then("the app was not relaunched")]
pub(super) fn not_relaunched(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(
        app_runtime::run(world).same_process(),
        "the process changed"
    );
    Ok(())
}

#[then("the configuration file holds the new address")]
pub(super) fn file_holds_new_address(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&settings(world).path())?;
    ensure!(saved.transcription.base_url == SILENT_LOCAL_SERVICE);
    Ok(())
}

#[then("the settings say the settings were not applied and name the speech recognition service")]
pub(super) fn says_not_applied(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let status = run.wait_for_status(|s| matches!(s, ApplyStatus::Rejected(_)))?;
    let said = status_text(&status);
    ensure!(
        said.contains("not applied") && said.contains("speech recognition"),
        "{said}"
    );
    Ok(())
}

#[then("the app keeps dictating with the first hotkey")]
pub(super) fn keeps_first_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let app = app_runtime::run(world);
    ensure!(
        app.hotkey_records(FIRST_HOTKEY),
        "the first hotkey no longer records"
    );
    app.finish_dictation(FIRST_HOTKEY)
}

#[then("a configuration file exists that sets the second hotkey")]
pub(super) fn file_created(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&settings(world).path())?;
    ensure!(saved.hotkey.key == SECOND_HOTKEY, "{:?}", saved.hotkey.key);
    Ok(())
}

#[then("every other option of that file keeps its default value")]
pub(super) fn others_default(world: &mut IntegrationWorld) -> Result<()> {
    let mut saved = voicepuncher::config::load(&settings(world).path())?;
    saved.hotkey.key = Config::default().hotkey.key;
    ensure!(Value::try_from(&saved)? == Value::try_from(Config::default())?);
    Ok(())
}

#[then("the settings say the settings apply after the current dictation")]
pub(super) fn says_waits(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let status = run.wait_for_status(|s| *s == ApplyStatus::WaitsForDictation)?;
    let said = status_text(&status);
    ensure!(said == "Applying after the current dictation…", "{said}");
    ensure!(
        !run.statuses.lock().unwrap().contains(&ApplyStatus::Applied),
        "applied during the recording"
    );
    Ok(())
}

#[then("the minimum press field is marked")]
pub(super) fn press_marked(world: &mut IntegrationWorld) -> Result<()> {
    let marked = settings(world).marked()?;
    ensure!(marked == "hotkey.min_press_ms", "{marked} is marked");
    Ok(())
}

#[then("the speech recognition address field is marked")]
pub(super) fn address_marked(world: &mut IntegrationWorld) -> Result<()> {
    let marked = settings(world).marked()?;
    ensure!(marked == "transcription.base_url", "{marked} is marked");
    Ok(())
}

#[then("the settings name the allowed range")]
pub(super) fn names_range(world: &mut IntegrationWorld) -> Result<()> {
    let refusal = settings(world).refusal.clone().context("not refused")?;
    ensure!(
        refusal.message.contains("between 50 and 1000"),
        "{}",
        refusal.message
    );
    Ok(())
}

#[then("the settings say the address must be on this machine")]
pub(super) fn says_loopback(world: &mut IntegrationWorld) -> Result<()> {
    let refusal = settings(world).refusal.clone().context("not refused")?;
    ensure!(
        refusal.message.contains("on this Mac"),
        "{}",
        refusal.message
    );
    Ok(())
}
