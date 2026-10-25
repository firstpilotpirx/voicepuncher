//! Steps of copies that cannot start: a taken port and a stale copy list.

use super::{
    any_process, copy_list, ensure, expected_copy_ports, given, off_thread, services, services_dir,
    then, wait_until, Command, Context, E2eWorld, Result, ScenarioServices, PROCESS_GONE_WITHIN,
    SEVERAL_COPIES, SPEECH_RECOGNITION,
};

/// Holds `port` bound without listening, so a server cannot bind it while nothing answers
/// there; it is released when dropped.
#[derive(Debug)]
pub struct TakenPort(std::process::Child);

impl TakenPort {
    fn hold(port: u32) -> Result<Self> {
        let mut child = Command::new("python3")
            .args([
                "-c",
                "import socket, sys, time\n\
                 s = socket.socket(); s.bind(('127.0.0.1', int(sys.argv[1])))\n\
                 print('bound', flush=True); time.sleep(600)",
                &port.to_string(),
            ])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .context("python3 runs")?;
        let mut line = String::new();
        std::io::BufRead::read_line(
            &mut std::io::BufReader::new(child.stdout.as_mut().context("its output")?),
            &mut line,
        )?;
        ensure!(line.trim() == "bound", "could not take port {port}");
        Ok(Self(child))
    }
}

impl Drop for TakenPort {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[given(
    "the speech recognition service is to run as several copies, and its second copy cannot start"
)]
pub(super) fn second_copy_cannot_start(world: &mut E2eWorld) -> Result<()> {
    let mut services = ScenarioServices::with_copies(Some(SEVERAL_COPIES))?;
    let second = expected_copy_ports(services.stt_port, SEVERAL_COPIES)[1];
    services.pool.taken = Some(TakenPort::hold(second)?);
    world.services.local_services = Some(services);
    Ok(())
}

#[given("a list of copies is left from an earlier start, and the service scripts started the speech recognition service as one copy")]
pub(super) async fn stale_copy_list(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::with_copies(Some(1))?;
        let left = expected_copy_ports(services.stt_port, SEVERAL_COPIES);
        let listed: Vec<String> = left.iter().map(u32::to_string).collect();
        std::fs::write(copy_list(services.stt_port), listed.join(" "))?;
        services.start_speech_recognition()?;
        let started = services.run.started.as_ref().expect("just started it");
        ensure!(
            started.status.success(),
            "the service scripts could not start the speech recognition service:\n{}",
            String::from_utf8_lossy(&started.stderr)
        );
        world.services.local_services = Some(services);
        Ok(())
    })
    .await
}

#[then("the start fails naming the service and the log of the copy that failed, and the copy that started is not left")]
pub(super) async fn copies_start_failed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let started = services.started()?;
        let report = String::from_utf8_lossy(&started.stderr).into_owned();
        ensure!(
            !started.status.success(),
            "the start succeeded although a copy could not start"
        );
        let stt_port = services.stt_port;
        let ports = expected_copy_ports(stt_port, SEVERAL_COPIES);
        let failed_log =
            services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{}.log", ports[1]));
        ensure!(
            report.contains(SPEECH_RECOGNITION) && report.contains(&*failed_log.to_string_lossy()),
            "the failure report does not name {SPEECH_RECOGNITION} and the log {}:\n{report}",
            failed_log.display()
        );
        let first = u16::try_from(ports[0])?;
        ensure!(
            wait_until(PROCESS_GONE_WITHIN, || any_process(first).is_none()),
            "the copy that started (port {first}) is left: {:?}",
            any_process(first)
        );
        ensure!(
            !copy_list(stt_port).exists(),
            "the list of copies is left after the failed start"
        );
        Ok(())
    })
    .await
}
