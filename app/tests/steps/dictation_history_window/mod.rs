//! Drives the history window's content over a real archive folder the real dictation archive
//! writes, with the clipboard noted and the local time a fixed two hours east of UTC.

mod archive;
mod givens;
mod thens;
mod whens;

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use tempfile::TempDir;

use voicepuncher::adapters::primary::history::{HistoryParts, HistoryView, Shown, ShownRow};
use voicepuncher::adapters::secondary::archiving::filesystem::FileSystemDictationHistory;

use crate::IntegrationWorld;

/// The local time's offset from UTC in these scenarios.
const UTC_OFFSET_SECS: i64 = 2 * 3600;

/// The archive folder of a scenario and the window over it.
#[derive(Debug)]
pub struct HistoryRun {
    dir: TempDir,
    archive_off: bool,
    view: Option<ViewCell>,
    copied: Arc<Mutex<Vec<String>>>,
}

/// The window's content; it holds closures, so it is shown to `Debug` by name only.
pub struct ViewCell(HistoryView);

impl std::fmt::Debug for ViewCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryView")
    }
}

impl HistoryRun {
    fn new() -> Result<Self> {
        // the warnings of skipped entries reach the shared log only once it is installed
        crate::steps::voice_control::pipeline_log();
        let dir = tempfile::tempdir().context("no temporary folder")?;
        Ok(Self { dir, archive_off: false, view: None, copied: Arc::default() })
    }

    fn open(&mut self) {
        let dir = (!self.archive_off).then(|| self.dir.path().to_path_buf());
        let copied = Arc::clone(&self.copied);
        let parts = HistoryParts {
            source: Box::new(move || Box::new(FileSystemDictationHistory::new(dir.clone()))),
            copier: Box::new(move |text| {
                copied.lock().unwrap().push(text.to_owned());
                Ok(())
            }),
            utc_offset: Box::new(|_| UTC_OFFSET_SECS),
        };
        self.view = Some(ViewCell(HistoryView::open(parts)));
    }

    fn view(&mut self) -> Result<&mut HistoryView> {
        Ok(&mut self
            .view
            .as_mut()
            .context("the history window is not open")?
            .0)
    }

    fn rows(&mut self) -> Result<Vec<ShownRow>> {
        match self.view()?.shown() {
            Shown::Rows(rows) => Ok(rows.clone()),
            Shown::Note(note) => anyhow::bail!("the window lists no row but says {note:?}"),
        }
    }
}

fn run(world: &mut IntegrationWorld) -> Result<&mut HistoryRun> {
    if world.app.history.is_none() {
        world.app.history = Some(HistoryRun::new()?);
    }
    world.app.history.as_mut().context("no history run")
}
