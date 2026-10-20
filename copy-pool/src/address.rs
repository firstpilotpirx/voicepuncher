//! Where the copies of a service listen, found from the service's address alone.

use std::fmt;

/// The copies of a service listen this far above the service's own port.
const COPY_PORT_OFFSET: u32 = 10_000;
/// The most copies one service runs; the service scripts keep to it.
pub const MAX_COPIES: u32 = 4;
const LAST_PORT: u32 = u16::MAX as u32;
const HTTP_PORT: u16 = 80;

/// A service's address, as `scheme://host:port/path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceAddress {
    scheme: String,
    host: String,
    port: u16,
    path: String,
}

/// An address that names no host or a port that is no number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressError(String);

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid service address: {}", self.0)
    }
}

impl std::error::Error for AddressError {}

impl ServiceAddress {
    pub fn parse(url: &str) -> Result<Self, AddressError> {
        let invalid = || AddressError(url.to_owned());
        let (scheme, rest) = url.split_once("://").ok_or_else(invalid)?;
        let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) if !port.ends_with(']') => {
                (host, port.parse().map_err(|_| invalid())?)
            }
            _ => (authority, HTTP_PORT),
        };
        if host.is_empty() {
            return Err(invalid());
        }
        Ok(Self {
            scheme: scheme.to_owned(),
            host: host.to_owned(),
            port,
            path: path.to_owned(),
        })
    }

    /// The service's own port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The host to connect to, without the brackets of an IPv6 address.
    pub fn host(&self) -> &str {
        self.host.trim_start_matches('[').trim_end_matches(']')
    }

    /// The ports where copies of the service may listen: the four above its port plus
    /// ten thousand, or above its port minus ten thousand when those would pass the last port.
    pub fn copy_ports(&self) -> Vec<u16> {
        let port = u32::from(self.port);
        let first = if port + COPY_PORT_OFFSET + MAX_COPIES <= LAST_PORT {
            port + COPY_PORT_OFFSET
        } else {
            port.saturating_sub(COPY_PORT_OFFSET)
        };
        (1..=MAX_COPIES)
            .filter_map(|copy| u16::try_from(first + copy).ok())
            .collect()
    }

    /// The address of the copy on `port`: the same scheme, host and path.
    pub fn with_port(&self, port: u16) -> String {
        format!("{}://{}:{port}{}", self.scheme, self.host, self.path)
    }
}

impl fmt::Display for ServiceAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.with_port(self.port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_scheme_host_port_and_path() {
        let address = ServiceAddress::parse("http://127.0.0.1:8081/v1").unwrap();
        assert_eq!(address.port(), 8081);
        assert_eq!(address.host(), "127.0.0.1");
        assert_eq!(address.with_port(18082), "http://127.0.0.1:18082/v1");
    }

    #[test]
    fn should_keep_the_brackets_of_an_ipv6_host_in_a_copy_address() {
        let address = ServiceAddress::parse("http://[::1]:8085/v1/").unwrap();
        assert_eq!(address.host(), "::1");
        assert_eq!(address.with_port(18086), "http://[::1]:18086/v1/");
    }

    #[test]
    fn should_take_the_http_port_when_the_address_names_none() {
        let address = ServiceAddress::parse("http://localhost").unwrap();
        assert_eq!(address.port(), 80);
        assert_eq!(address.with_port(10081), "http://localhost:10081");
    }

    #[test]
    fn should_reject_an_address_without_host_or_with_a_bad_port() {
        assert!(ServiceAddress::parse("127.0.0.1:8081").is_err());
        assert!(ServiceAddress::parse("http://:8081/v1").is_err());
        assert!(ServiceAddress::parse("http://127.0.0.1:port/v1").is_err());
    }

    #[test]
    fn should_place_the_copies_ten_thousand_above_the_service_port() {
        let address = ServiceAddress::parse("http://127.0.0.1:8081/v1").unwrap();
        assert_eq!(address.copy_ports(), (18082..=18085).collect::<Vec<u16>>());
    }

    #[test]
    fn should_place_the_copies_lower_when_above_would_pass_the_last_port() {
        let address = ServiceAddress::parse("http://127.0.0.1:55532/v1").unwrap();
        assert_eq!(address.copy_ports(), (45533..=45536).collect::<Vec<u16>>());
    }

    #[test]
    fn should_keep_the_copies_of_dictation_apart_from_those_of_commands() {
        let ports = |url| ServiceAddress::parse(url).unwrap().copy_ports();
        let dictation = ports("http://127.0.0.1:8081/v1");
        let commands = ports("http://127.0.0.1:8085/v1");
        assert!(dictation.iter().all(|port| !commands.contains(port)));
    }
}
