use super::*;
use std::fs;
use std::path::PathBuf;

fn lock_dir_with(holders: &[(u32, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("holders")).unwrap();
    for (pid, content) in holders {
        fs::write(dir.path().join("holders").join(pid.to_string()), content).unwrap();
    }
    dir
}

#[test]
fn should_list_a_live_holder_with_its_lock_checkout_command_and_mode() {
    let dir = lock_dir_with(&[(
        4242,
        "pid=4242\npgid=4242\ndir=/w/x\ncmd=just _check\nmode=shared\n",
    )]);
    let holders = live_holders(&[dir.path().to_path_buf()], |_| true);
    let lock = dir
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(
        holders,
        vec![LockHolder {
            lock,
            pid: 4242,
            dir: "/w/x".into(),
            command: "just _check".into(),
            mode: "shared".into()
        }]
    );
}

#[test]
fn should_skip_a_holder_whose_process_is_gone_and_a_missing_directory() {
    let dir = lock_dir_with(&[
        (1, "pid=1\ndir=/a\ncmd=a\nmode=shared\n"),
        (2, "pid=2\ndir=/b\ncmd=b\nmode=exclusive\n"),
    ]);
    let holders = live_holders(
        &[
            dir.path().to_path_buf(),
            PathBuf::from("/nonexistent/lock-dir"),
        ],
        |pid| pid == 2,
    );
    assert_eq!(holders.len(), 1);
    assert_eq!(holders[0].dir, "/b");
}

#[test]
fn should_keep_the_five_busiest_processes_without_the_header() {
    let ps = "%CPU   PID COMM\n 98.1   10 rustc\n 77.0   11 rustc\n 40.2   12 cargo\n 12.0   13 whisper-server\n  5.5   14 just\n  1.0   15 zsh\n";
    let rows = busiest_processes(ps);
    assert_eq!(rows.len(), 5);
    assert_eq!(
        (rows[0].as_str(), rows[4].as_str()),
        ("98.1   10 rustc", "5.5   14 just")
    );
}
