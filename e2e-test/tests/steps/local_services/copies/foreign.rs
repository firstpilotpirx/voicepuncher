//! Steps of another program, no speech recognition service, on the service's own port.

use std::net::TcpListener;

use voicepuncher_test_steps::loopback::Http2Only;

use super::{ensure, given, services, then, Context, E2eWorld, Result};

/// Another program on the service's own port, which answers HTTP/2 only.
#[derive(Debug)]
pub struct ForeignListener(Http2Only);

impl ForeignListener {
    fn on(port: u16) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .with_context(|| format!("another program cannot take port {port}"))?;
        Ok(Self(Http2Only::on(listener)))
    }
}

#[given("another program that is no speech recognition service listens on the service's own port")]
pub(super) fn foreign_on_service_port(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    services.pool.foreign = Some(ForeignListener::on(services.stt_port)?);
    Ok(())
}

#[then("the client sends its requests to the three copies, and the other program receives no transcription request")]
pub(super) fn passed_over(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    super::discovery::found_the_three_copies(services)?;
    let foreign = services
        .pool
        .foreign
        .as_ref()
        .context("no other program listens")?;
    let requests = foreign.0.transcriptions();
    ensure!(
        requests == 0,
        "the other program got {requests} transcription requests"
    );
    Ok(())
}
