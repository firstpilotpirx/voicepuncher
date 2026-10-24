//! Routes the requests of a client to the copies of a local speech recognition service:
//! the copies are found from the service's address, a request takes a free copy or waits
//! its turn, and a copy answers one request at a time.

mod address;
mod pool;
mod probe;

pub use address::{
    AddressError, ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET, MAX_COPIES,
    SPEECH_RECOGNITION_COPY_OFFSET,
};
pub use pool::{CopyPool, CopyState, Finish, Lease, Place, PoolError, Ran};
pub use probe::{Probe, TcpProbe};
