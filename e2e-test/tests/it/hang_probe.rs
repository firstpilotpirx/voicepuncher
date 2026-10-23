//! Test probe for the nextest time limit (fail-hung-tests-fast, D10): armed with
//! `VOICEPUNCHER_HANG_PROBE=1`, it never returns, so `just test` must stop it as `TIMEOUT`.
//! The recipes leave it out unless armed (`needs_hang_probe`).

mod needs_hang_probe {
    use voicepuncher_test_steps::required_env;

    #[test]
    fn should_hang_until_the_test_limit_stops_it_when_armed() {
        assert_eq!(
            required_env("VOICEPUNCHER_HANG_PROBE"),
            "1",
            "the probe is armed with 1"
        );
        eprintln!("hang probe armed: sleeping until the test limit stops this test");
        loop {
            std::thread::park();
        }
    }
}
