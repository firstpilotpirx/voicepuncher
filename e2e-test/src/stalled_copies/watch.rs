use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// How the command spotting service writes a confirmation it cut short.
const CUT_SHORT: &str = "confirmation cut short after ";

/// The command spotting service's log from a point on: what it writes after `start`.
pub struct Watch {
    log: PathBuf,
    from: u64,
}

impl Watch {
    /// Watches the log of the command spotting service at `url` (`ws://host:port`), in the
    /// service scripts' log directory under `home`, from its current end.
    pub fn start(url: &str, home: &Path) -> Self {
        let port = url.rsplit(':').next().unwrap_or_default();
        let log = home
            .join(".voicepuncher/logs")
            .join(format!("voicepuncher-command-spotting-{port}.log"));
        let from = std::fs::metadata(&log).map_or(0, |meta| meta.len());
        Self { log, from }
    }

    /// The confirmations cut short since `start`, as `cut_short` reports them; none when the
    /// log cannot be read.
    pub fn cut_short(&self) -> Vec<String> {
        let mut since = String::new();
        let read = std::fs::File::open(&self.log).and_then(|mut file| {
            file.seek(SeekFrom::Start(self.from))?;
            file.read_to_string(&mut since)
        });
        read.map_or_else(|_| Vec::new(), |_| cut_short(&since))
    }
}

/// "copy <port>: cut short after <n> ms (<what followed>)" for each confirmation `log` says
/// was cut short, in order.
pub fn cut_short(log: &str) -> Vec<String> {
    log.lines()
        .filter_map(|line| {
            let rest = &line[line.find(CUT_SHORT)? + CUT_SHORT.len()..];
            let (ms, rest) = rest.split_once("ms on copy ")?;
            let (url, next) = rest.split_once(": ")?;
            let port = url.trim_end_matches("/v1").rsplit(':').next()?;
            Some(format!("copy {port}: cut short after {ms} ms ({next})"))
        })
        .collect()
}

/// The cut-short confirmations of a start that gave no cue, one per line, for its failure.
pub fn report(cut_short: &[String]) -> String {
    if cut_short.is_empty() {
        return "no confirmation was cut short".into();
    }
    format!("confirmations cut short:\n  {}", cut_short.join("\n  "))
}
