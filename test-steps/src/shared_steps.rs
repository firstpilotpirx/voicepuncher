//! The Gherkin steps both harnesses use.
//!
//! Their meaning is [`SharedSteps`], one method per step,
//! and [`shared_steps!`](crate::shared_steps) writes the step functions with their texts once. Each
//! harness implements the trait in its own terms (doubles in-process, the real app and services
//! in E2E); a step a level cannot perform returns an error naming the level.

use anyhow::Result;

/// The steps of the slice and component harness that the E2E harness also runs.
#[expect(
    async_fn_in_trait,
    reason = "cucumber runs the steps on its local executor: no Send bound is needed"
)]
pub trait SharedSteps {
    /// Given voice control is turned on: the configuration turns voice control on.
    async fn voice_control_is_turned_on(&mut self) -> Result<()>;
    /// Given listening is on: the app runs and listens for voice commands.
    async fn listening_is_on(&mut self) -> Result<()>;
    /// Given the overlay is hidden: a check that no sign shows yet.
    async fn the_overlay_is_hidden(&mut self) -> Result<()>;
    /// Given the overlay shows that voice commands do not work.
    async fn commands_unavailable_shown_given(&mut self) -> Result<()>;
    /// Given the command spotting service went away during it and is back.
    async fn command_spotting_went_away_and_is_back(&mut self) -> Result<()>;
    /// When the app starts.
    async fn the_app_starts(&mut self) -> Result<()>;
    /// When the command spotting service stays away for longer than 5 seconds.
    async fn command_spotting_stays_away(&mut self) -> Result<()>;
    /// Then the overlay shows that voice commands do not work.
    async fn commands_unavailable_shown(&mut self) -> Result<()>;
    /// Then no listening sound played after the app started.
    async fn no_listening_sound_after_start(&mut self) -> Result<()>;
    /// Then no permission is checked.
    async fn no_permission_is_checked(&mut self) -> Result<()>;
    /// Then only the other sentence is pasted, its first words included.
    async fn only_the_other_sentence_is_pasted(&mut self) -> Result<()>;
    /// Then the Return key is pressed after it: after the paste.
    async fn return_pressed_after_the_paste(&mut self) -> Result<()>;
    /// Then the Return key is pressed in the focused app.
    async fn return_pressed(&mut self) -> Result<()>;
    /// Then the sentence is pasted into the focused app.
    async fn the_sentence_is_pasted(&mut self) -> Result<()>;
}

/// Writes the step functions of [`SharedSteps`] for `$world` in the harness that calls it.
///
/// Each step function carries its text and calls the world's method.
#[macro_export]
macro_rules! shared_steps {
    ($world:ident) => {
        $crate::shared_steps!(@step $world, given, "voice control is turned on", voice_control_is_turned_on);
        $crate::shared_steps!(@step $world, given, "listening is on", listening_is_on);
        $crate::shared_steps!(@step $world, given, "the overlay is hidden", the_overlay_is_hidden);
        $crate::shared_steps!(@step $world, given, "the overlay shows that voice commands do not work", commands_unavailable_shown_given);
        $crate::shared_steps!(@step $world, given, "the command spotting service went away during it and is back", command_spotting_went_away_and_is_back);
        $crate::shared_steps!(@step $world, when, "the app starts", the_app_starts);
        $crate::shared_steps!(@step $world, when, "the command spotting service stays away for longer than 5 seconds", command_spotting_stays_away);
        $crate::shared_steps!(@step $world, then, "the overlay shows that voice commands do not work", commands_unavailable_shown);
        $crate::shared_steps!(@step $world, then, "no listening sound played after the app started", no_listening_sound_after_start);
        $crate::shared_steps!(@step $world, then, "no permission is checked", no_permission_is_checked);
        $crate::shared_steps!(@step $world, then, "only the other sentence is pasted, its first words included", only_the_other_sentence_is_pasted);
        $crate::shared_steps!(@step $world, then, "the Return key is pressed after it", return_pressed_after_the_paste);
        $crate::shared_steps!(@step $world, then, "the Return key is pressed in the focused app", return_pressed);
        $crate::shared_steps!(@step $world, then, "the sentence is pasted into the focused app", the_sentence_is_pasted);
    };
    (@step $world:ident, $keyword:ident, $text:literal, $method:ident) => {
        #[::cucumber::$keyword($text)]
        #[allow(clippy::future_not_send, reason = "cucumber runs the steps on its local executor")]
        async fn $method(world: &mut $world) -> ::anyhow::Result<()> {
            <$world as $crate::SharedSteps>::$method(world).await
        }
    };
}
