//! Confirmations of the command spotting service against stand-in copies of the speech
//! recognition service for commands: copies held busy, or decoding one request at a time
//! for longer than a confirmation may wait, through `UtteranceTranscriber`.
//!
//! The stand-in's address is kept in the world's `transcription_url`, the confirmation's
//! outcome in `stdout`, its log lines in `stderr` and its time in `elapsed`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher_command_spotting::confirm::UtteranceTranscriber;
use voicepuncher_copy_pool::{CopyPool, ServiceAddress};

use crate::steps::local_services::{log_mark, logged_since};
use crate::steps::off_thread;
use crate::E2eWorld;

const PRESS_ENTER: &str = r#"{"text":" press enter","segments":[]}"#;
/// The deadline of every confirmation in these scenarios.
const DEADLINE: Duration = Duration::from_secs(3);
/// How long a stalled copy decodes: past the deadline.
const STALL: Duration = Duration::from_secs(5);
/// How far past its deadline a confirmation may end.
const PAST_DEADLINE: Duration = Duration::from_secs(1);
/// The base ports the two-copies scenarios take, 8 apart: below the ports the system hands
/// out and the copy windows of the command spotting tests, and clear of the project's services.
const WINDOW_BASES: std::ops::Range<u16> = 35_000..37_400;

/// The ports of the copies that took a confirmation.
static ANSWERED: Mutex<Vec<u16>> = Mutex::new(Vec::new());

/// Serves the requests on `listener` one at a time: a health check at once, a confirmation
/// after `decoding`; a copy decoding answers nothing else meanwhile, as a stalled one.
fn stand_in_copy(listener: TcpListener, decoding: Duration) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            answer(stream, decoding);
        }
    });
}

fn answer(mut stream: TcpStream, decoding: Duration) {
    if voicepuncher_test_steps::loopback::answered_as_whisper(&mut stream) {
        return;
    }
    let Ok(port) = stream.local_addr().map(|a| a.port()) else {
        return;
    };
    let mut reader = BufReader::new(&mut stream);
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
            break;
        }
        lines.push(line.to_ascii_lowercase());
    }
    let length: usize = lines
        .iter()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    // A probe of the copy pool connects and sends nothing
    let Some(request) = lines.first() else {
        return;
    };
    let body = if request.starts_with("get /health") {
        "ok"
    } else {
        let _ = reader.read_exact(&mut vec![0u8; length]);
        ANSWERED.lock().unwrap().push(port);
        std::thread::sleep(decoding);
        PRESS_ENTER
    };
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(reply.as_bytes());
}

/// One stand-in copy at its own address; its address.
fn one_copy(decoding: Duration) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/v1", listener.local_addr()?);
    stand_in_copy(listener, decoding);
    Ok(url)
}

/// Holds the only copy at `url` busy for `busy`.
fn hold_busy(url: &str, busy: Duration) -> Result<()> {
    let lease = CopyPool::shared(url)?.lease(DEADLINE)?;
    std::thread::spawn(move || {
        std::thread::sleep(busy);
        lease.finished();
    });
    Ok(())
}

#[given("every copy of the speech recognition service for commands is busy for longer than a confirmation's deadline")]
fn busy_past_deadline(world: &mut E2eWorld) -> Result<()> {
    let url = one_copy(Duration::ZERO)?;
    hold_busy(&url, DEADLINE * 2)?;
    world.transcription_url = Some(url);
    Ok(())
}

#[given("every copy of the speech recognition service for commands is busy for one more second")]
fn busy_one_second(world: &mut E2eWorld) -> Result<()> {
    let url = one_copy(Duration::ZERO)?;
    hold_busy(&url, Duration::from_secs(1))?;
    world.transcription_url = Some(url);
    Ok(())
}

#[given("every copy of the speech recognition service for commands stops answering")]
fn every_copy_stalls(world: &mut E2eWorld) -> Result<()> {
    world.transcription_url = Some(one_copy(STALL)?);
    Ok(())
}

#[given("the speech recognition service for commands runs as two copies and the copy a confirmation reaches first stops answering")]
fn one_copy_stalls(world: &mut E2eWorld) -> Result<()> {
    let slot = u16::try_from(std::process::id() % 300)?;
    for n in 0..300 {
        let base = WINDOW_BASES.start + ((slot + n) % 300) * 8;
        let url = format!("http://127.0.0.1:{base}/v1");
        let ports = ServiceAddress::parse(&url)?.copy_ports();
        let (Ok(first), Ok(second)) = (
            TcpListener::bind(("127.0.0.1", ports[0])),
            TcpListener::bind(("127.0.0.1", ports[1])),
        ) else {
            continue;
        };
        // The pool of this process tries the copy its process id points at first
        let (stalled, live) = if std::process::id() % 2 == 0 {
            (first, second)
        } else {
            (second, first)
        };
        stand_in_copy(stalled, STALL);
        stand_in_copy(live, Duration::ZERO);
        world.transcription_url = Some(url);
        return Ok(());
    }
    bail!("no free window of copy ports")
}

#[when("the user says \"press enter\" and the command spotting service asks for its confirmation")]
async fn confirm(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let url = world
            .transcription_url
            .clone()
            .context("no stand-in copies")?;
        let transcriber = UtteranceTranscriber::new(&url)?;
        let mark = log_mark();
        let started = Instant::now();
        let outcome = transcriber.transcribe(&[0.0; 1_600], 16_000, DEADLINE);
        world.app.elapsed = Some(started.elapsed());
        world.app.stdout = match outcome {
            Ok(heard) => heard.text.trim().to_owned(),
            Err(e) => format!("failed: {e:#}"),
        };
        world.app.stderr = logged_since(mark, "").join("\n");
        Ok(())
    })
    .await
}

/// The port of the only copy at the stand-in's address.
fn single_port(world: &E2eWorld) -> Result<u16> {
    let url = world
        .transcription_url
        .as_deref()
        .context("no stand-in copies")?;
    Ok(ServiceAddress::parse(url)?.port())
}

#[then("the confirmation is logged as given up and ends without waiting much past its deadline, and no copy receives it")]
fn given_up_unsent(world: &mut E2eWorld) -> Result<()> {
    let took = world.app.elapsed.context("nothing was confirmed")?;
    let received = ANSWERED.lock().unwrap().contains(&single_port(world)?);
    let url = world.transcription_url.as_deref().unwrap_or_default();
    let logged = world
        .app
        .stderr
        .lines()
        .any(|l| l.contains("confirmation given up unsent") && l.contains(url));
    if !world.app.stdout.starts_with("failed")
        || !logged
        || took > DEADLINE + PAST_DEADLINE
        || received
    {
        bail!(
            "{:?} after {took:?}, received: {received}, log: {}",
            world.app.stdout,
            world.app.stderr
        );
    }
    Ok(())
}

#[then("the confirmation is sent to the copy once it frees, and \"press enter\" is reported")]
fn sent_once_free(world: &mut E2eWorld) -> Result<()> {
    let received = ANSWERED.lock().unwrap().contains(&single_port(world)?);
    if world.app.stdout != "press enter" || !received {
        bail!("{:?}, received by the copy: {received}", world.app.stdout);
    }
    Ok(())
}

#[then(
    "the other copy confirms it within the confirmation's deadline and \"press enter\" is reported"
)]
fn other_copy_confirms(world: &mut E2eWorld) -> Result<()> {
    let took = world.app.elapsed.context("nothing was confirmed")?;
    if world.app.stdout != "press enter"
        || took >= DEADLINE
        || !world.app.stderr.contains("another copy")
    {
        bail!(
            "{:?} after {took:?}, log: {}",
            world.app.stdout,
            world.app.stderr
        );
    }
    Ok(())
}

#[then("it ends as given up, not as speech heard without a command")]
fn ends_given_up(world: &mut E2eWorld) -> Result<()> {
    if !world.app.stdout.starts_with("failed") {
        bail!("the confirmation ended as {:?}", world.app.stdout);
    }
    Ok(())
}
