use std::net::IpAddr;

use anyhow::{bail, Context, Result};
use url::{Host, Url};

const LOCALHOST: &str = "localhost";
const ALLOWED_SCHEMES: &[&str] = &["http", "https"];

/// Rejects a service address that is not on this machine, so audio and text never
/// leave it. `setting` names the config key the address came from.
pub fn ensure_loopback(setting: &str, base_url: &str) -> Result<()> {
    let url = Url::parse(base_url)
        .with_context(|| format!("{setting} is not a valid URL: {base_url}"))?;
    if !ALLOWED_SCHEMES.contains(&url.scheme()) {
        bail!("{setting} must use http or https, got: {base_url}");
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

    const SETTING: &str = "refinement.base_url";

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
}
