//! Ports bound without listening for the life of the test process, which the fake services'
//! addresses take: connections to them are refused, so a client looks for copies, and no test
//! running beside one binds it as a copy of its own.

use std::sync::atomic::{AtomicUsize, Ordering};

/// A port of `range` bound without listening for the life of the test process.
pub(crate) fn reserve_port(range: std::ops::Range<u16>) -> u16 {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let span = range.len();
    let start = std::process::id() as usize * 7_919;
    loop {
        let index = (start + NEXT.fetch_add(1, Ordering::SeqCst)) % span;
        let port = range.start + u16::try_from(index).unwrap();
        if bind_without_listening(port) {
            return port;
        }
    }
}

/// Binds `port` on loopback without listening; whether it was free.
pub(crate) fn bind_without_listening(port: u16) -> bool {
    // SAFETY: a plain socket call with constant arguments.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    assert!(fd >= 0, "socket: {}", std::io::Error::last_os_error());
    // SAFETY: sockaddr_in is plain data, valid when zeroed.
    let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    let len = libc::socklen_t::try_from(std::mem::size_of::<libc::sockaddr_in>()).unwrap();
    addr.sin_len = u8::try_from(len).unwrap();
    addr.sin_family = libc::sa_family_t::try_from(libc::AF_INET).unwrap();
    addr.sin_port = port.to_be();
    addr.sin_addr.s_addr = u32::from(std::net::Ipv4Addr::LOCALHOST).to_be();
    // SAFETY: `addr` is a sockaddr_in of length `len`, alive for the call; a bound descriptor
    // is never closed, so its port stays reserved until the process exits.
    let bound = unsafe { libc::bind(fd, (&raw const addr).cast(), len) } == 0;
    if !bound {
        // SAFETY: `fd` is this function's own open descriptor, closed once.
        unsafe { libc::close(fd) };
    }
    bound
}
