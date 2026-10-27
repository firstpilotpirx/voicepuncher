//! The macOS tests that run on the main thread, where `AppKit` draws and the main queue
//! runs: the overlay, the menu bar status item, the settings and the history window. libtest runs each
//! test on a thread of its own, where none of them can be built, so this harness
//! (`harness = false`) answers cargo-nextest's listing and runs each test on the main
//! thread. Every module lists its tests; a test is listed as `<module>::<name>`.

mod history_macos;
mod menubar_macos;
mod settings_macos;
mod visualfeedback_macos;

use objc2::MainThreadMarker;

/// A test of a module: its name and the function that runs it.
pub type Test = (&'static str, fn());

/// Every module with its tests.
const MODULES: &[(&str, &[Test])] = &[
    ("history_macos", history_macos::TESTS),
    ("menubar_macos", menubar_macos::TESTS),
    ("settings_macos", settings_macos::TESTS),
    ("visualfeedback_macos", visualfeedback_macos::TESTS),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--list", "--format", "terse", "--ignored"] => {}
        ["--list", "--format", "terse"] => list(),
        ["--exact", name, ..] => run(name),
        _ => tests().for_each(|(_, test)| test()),
    }
}

/// Every test with its listed name, `<module>::<name>`.
fn tests() -> impl Iterator<Item = (String, fn())> {
    MODULES.iter().flat_map(|(module, tests)| {
        tests
            .iter()
            .map(move |(name, test)| (format!("{module}::{name}"), *test))
    })
}

fn list() {
    for (name, _) in tests() {
        println!("{name}: test");
    }
}

fn run(name: &str) {
    let (_, test) = tests()
        .find(|(listed, _)| listed == name)
        .unwrap_or_else(|| panic!("no test named {name}"));
    test();
}

/// The marker every test needs: the harness runs on the main thread.
pub fn main_thread() -> MainThreadMarker {
    MainThreadMarker::new().expect("the harness runs every test on the main thread")
}
