//! The HTTP agents of the app: one for the local services, one for model downloads.

use std::time::Duration;

/// A model host that does not connect within this is taken as no connection.
const DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// A download that sends nothing for this long is cut; a slow one is not.
const DOWNLOAD_READ_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_REDIRECTS: u32 = 8;

/// An agent for a local service that opens a new connection for every request.
///
/// The service closes an idle connection after a few seconds, and after a pause of minutes
/// the HTTP client fails a request on such a connection instead of reconnecting. A new
/// loopback connection costs about 0.1 ms.
pub fn fresh_connections(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(timeout)
        .max_idle_connections(0)
        .max_idle_connections_per_host(0)
        .build()
}

/// An agent for model downloads: no overall time limit, so a slow link is no failure;
/// redirects to the hosts' CDNs are followed.
pub fn downloads() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(DOWNLOAD_CONNECT_TIMEOUT)
        .timeout_read(DOWNLOAD_READ_TIMEOUT)
        .redirects(DOWNLOAD_REDIRECTS)
        .user_agent(concat!("voicepuncher/", env!("CARGO_PKG_VERSION")))
        .build()
}
