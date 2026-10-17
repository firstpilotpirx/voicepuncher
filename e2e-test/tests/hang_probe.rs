//! Test probe for the nextest time limit (fail-hung-tests-fast, D10): armed with
//! `VOICEPUNCHER_HANG_PROBE=1`, it never returns, so `just test` must stop it as `TIMEOUT`.

#[test]
fn should_hang_until_the_test_limit_stops_it_when_armed() {
    if std::env::var("VOICEPUNCHER_HANG_PROBE").as_deref() != Ok("1") {
        eprintln!("GATED: VOICEPUNCHER_HANG_PROBE not set, skipping");
        return;
    }
    eprintln!("hang probe armed: sleeping until the test limit stops this test");
    loop {
        std::thread::park();
    }
}
