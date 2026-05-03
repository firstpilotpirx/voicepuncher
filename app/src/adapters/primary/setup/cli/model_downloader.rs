use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

pub struct ModelDownloader;

impl ModelDownloader {
    pub fn download(url: &str, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create directory {}", parent.display()))?;
        }

        let tmp = dest.with_extension("download");

        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(4 * 60 * 60))
            .build();

        let response = agent
            .get(url)
            .call()
            .map_err(|e| anyhow!("HTTP request failed: {e}"))?;

        let total: u64 = response
            .header("Content-Length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let filename = dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "model".to_string());

        let pb = if total > 0 {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "{msg}\n[{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                    )
                    .unwrap()
                    .progress_chars("=> "),
            );
            pb
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} {msg}  {bytes} ({bytes_per_sec})")
                    .unwrap(),
            );
            pb
        };
        pb.set_message(filename.clone());

        let mut reader = response.into_reader();
        let mut file = fs::File::create(&tmp)
            .with_context(|| format!("create temp file {}", tmp.display()))?;
        let mut buf = [0u8; 65_536];

        loop {
            let n = reader.read(&mut buf).context("read from download stream")?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n]).context("write to temp file")?;
            pb.inc(n as u64);
        }

        pb.finish_with_message(format!("{filename} — done"));

        fs::rename(&tmp, dest)
            .with_context(|| format!("rename {} to {}", tmp.display(), dest.display()))?;

        Ok(())
    }
}
