use std::collections::HashMap;
use std::io::Write;

use voicepuncher_test_steps::loopback::{serve_each, skip_request};

pub(crate) use voicepuncher::adapters::secondary::localservices::catalog::{
    needed, Machine, Needs,
};
use voicepuncher::adapters::secondary::localservices::process::HttpProbe;
use voicepuncher::config::Config;
use voicepuncher::core::domain::ServiceKind;

/// A whisper-server stand-in on a free port: healthy, and named on its start page.
fn whisper_on_free_port() -> u16 {
    serve_each(|mut stream| {
        skip_request(&mut stream);
        let body = "Whisper.cpp Server";
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    })
}

#[test]
fn should_take_a_running_whisper_server_on_the_service_port_as_the_service() {
    // Given
    let port = whisper_on_free_port();
    let mut cfg = Config::default();
    cfg.transcription.base_url = format!("http://127.0.0.1:{port}/v1");
    let machine = Machine {
        home: tempfile::tempdir().unwrap().keep(),
        env: HashMap::new(),
        path: Vec::new(),
        app_dir: None,
        cores: 4,
    };
    let needs = Needs { voice_control: false };
    // When
    let specs = needed(&cfg, needs, &machine, &HttpProbe::new());
    // Then
    let dictation: Vec<(ServiceKind, u16)> =
        specs.iter().map(|spec| (spec.kind, spec.port)).collect();
    assert_eq!(dictation, vec![(ServiceKind::Dictation, port)]);
}
