//! Connections a stand-in service ends with a reset, as a busy or crashed service does.

use std::net::TcpStream;
use std::os::fd::AsRawFd;

/// Closes the connection with a reset instead of an orderly close.
pub fn reset(stream: TcpStream) {
    let linger = libc::linger { l_onoff: 1, l_linger: 0 };
    // SAFETY: `stream` owns a valid socket; `linger` is a valid SO_LINGER value of its size.
    let rc = unsafe {
        libc::setsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_LINGER,
            (&raw const linger).cast(),
            libc::socklen_t::try_from(std::mem::size_of::<libc::linger>()).unwrap(),
        )
    };
    assert_eq!(rc, 0, "set SO_LINGER");
    drop(stream);
}
