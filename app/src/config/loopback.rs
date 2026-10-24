use std::net::IpAddr;

use anyhow::{bail, Context, Result};
use url::{Host, Url};

const LOCALHOST: &str = "localhost";
const HTTP_SCHEMES: &[&str] = &["http", "https"];
const WEBSOCKET_SCHEMES: &[&str] = &["ws", "wss"];

/// Rejects a service address that is not on this machine, so audio and text never
/// leave it. `setting` names the config key the address came from.
pub fn ensure_loopback(setting: &str, base_url: &str) -> Result<()> {
    ensure_loopback_with_schemes(setting, base_url, HTTP_SCHEMES)
}

/// As [`ensure_loopback`], for a service reached over WebSocket.
pub fn ensure_loopback_websocket(setting: &str, url: &str) -> Result<()> {
    ensure_loopback_with_schemes(setting, url, WEBSOCKET_SCHEMES)
}

fn ensure_loopback_with_schemes(setting: &str, base_url: &str, schemes: &[&str]) -> Result<()> {
    let url = Url::parse(base_url)
        .with_context(|| format!("{setting} is not a valid URL: {base_url}"))?;
    if !schemes.contains(&url.scheme()) {
        bail!(
            "{setting} must use {}, got: {base_url}",
            schemes.join(" or ")
        );
    }
    let is_loopback = match url.host() {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case(LOCALHOST),
        Some(Host::Ipv4(ip)) => IpAddr::V4(ip).is_loopback(),
        Some(Host::Ipv6(ip)) => IpAddr::V6(ip).is_loopback(),
        None => false,
    };
    if !is_loopback {
        bail!(
            "{setting} {base_url} is not on this machine — only a local service \
             (localhost, 127.0.0.0/8 or [::1]) is allowed, so audio and text never leave it"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTING: &str = "transcription.base_url";

    #[test]
    fn should_accept_loopback_hosts() {
        for url in [
            "http://localhost:8080/v1",
            "http://127.0.0.1:8080/v1",
            "http://127.1.2.3/v1",
            "http://[::1]:8080/v1",
        ] {
            assert!(
                ensure_loopback(SETTING, url).is_ok(),
                "expected {url} to be accepted"
            );
        }
    }

    #[test]
    fn should_reject_hosts_on_other_machines_naming_the_setting() {
        for url in ["http://192.168.1.10:8080/v1", "https://api.openai.com/v1"] {
            let message = format!("{:#}", ensure_loopback(SETTING, url).unwrap_err());
            assert!(message.contains("not on this machine"), "{url}: {message}");
            assert!(message.contains(SETTING), "{url}: {message}");
        }
    }

    #[test]
    fn should_reject_malformed_url() {
        assert!(ensure_loopback(SETTING, "not a url").is_err());
    }

    #[test]
    fn should_reject_non_http_scheme() {
        assert!(ensure_loopback(SETTING, "ftp://127.0.0.1/v1").is_err());
    }

    #[test]
    fn should_accept_a_local_websocket_service() {
        assert!(ensure_loopback_websocket("voice_control.url", "ws://127.0.0.1:8082").is_ok());
    }

    #[test]
    fn should_reject_an_http_url_for_a_websocket_service() {
        let message = format!(
            "{:#}",
            ensure_loopback_websocket("voice_control.url", "http://127.0.0.1:8082").unwrap_err()
        );
        assert!(message.contains("must use ws"), "{message}");
    }
}
