//! Process and port helpers of the scenario services: signals, health, free ports, process lookup and waiting.

use super::{
    anyhow, ensure, Command, Duration, Instant, Path, PathBuf, Result, ServiceAddress, SocketAddr,
    TcpListener, TcpStream, HEALTH_CHECK_WAIT, SPEECH_RECOGNITION,
};

pub(super) fn signal(pid: u32, signal: &str) -> Result<()> {
    let status = Command::new("kill")
        .args([signal, &pid.to_string()])
        .status()?;
    ensure!(status.success(), "kill {signal} {pid} failed");
    Ok(())
}

/// The service on `port` answers its health check.
pub fn healthy(port: u16) -> bool {
    ureq::get(&format!("http://127.0.0.1:{port}/health"))
        .timeout(HEALTH_CHECK_WAIT)
        .call()
        .is_ok()
}

pub(super) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("e2e-test lives in the workspace")
        .to_path_buf()
}

pub(super) fn services_dir(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("HOME is set"))
        .join(".voicepuncher")
        .join(name)
}

pub(super) fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

/// A free port for a speech recognition service whose copy ports are free too: the service
/// scripts and the clients take anything listening on them for a copy of the service.
pub(super) fn free_speech_recognition_port() -> Result<u16> {
    loop {
        let port = free_port()?;
        let copy_ports = ServiceAddress::parse(&format!("http://127.0.0.1:{port}/v1"))
            .map_err(|e| anyhow!("{e}"))?
            .copy_ports();
        if !copy_ports.into_iter().any(answers) {
            return Ok(port);
        }
    }
}

pub(super) fn answers(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(500),
    )
    .is_ok()
}

pub(super) fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A process of the speech recognition service on `port`: the service itself or what keeps it
/// running, found by its command line.
pub(super) fn any_process(port: u16) -> Option<u32> {
    pgrep(&format!("{SPEECH_RECOGNITION}.*--port {port}( |$)"))
}

/// The speech recognition service process itself on `port`.
pub(super) fn service_process(port: u16) -> Option<u32> {
    pgrep(&format!("^{SPEECH_RECOGNITION} .*--port {port}( |$)"))
}

pub(super) fn pgrep(pattern: &str) -> Option<u32> {
    let output = Command::new("pgrep").args(["-f", pattern]).output().ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .find(|pid| *pid != std::process::id())
}

pub(super) fn wait_until(limit: Duration, condition: impl Fn() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < limit {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    condition()
}
