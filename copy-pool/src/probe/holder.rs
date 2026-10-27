//! Which program listens on a port, as `lsof` tells it.

use std::process::Command;

use super::ForeignProgram;

/// `lsof` ships with macOS here; the path keeps a changed `$PATH` from picking another.
const LSOF: &str = "/usr/sbin/lsof";

/// The program listening on `port` of loopback, named as far as `lsof` tells it.
pub fn holder_of(port: u16) -> ForeignProgram {
    let output = Command::new(LSOF)
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-Fpc"])
        .output();
    let listing = output.map(|output| String::from_utf8_lossy(&output.stdout).into_owned());
    parse(port, &listing.unwrap_or_default())
}

/// The first process of an `lsof -Fpc` listing: `p<pid>` and `c<command>` lines.
fn parse(port: u16, listing: &str) -> ForeignProgram {
    let mut holder = ForeignProgram { port, name: None, pid: None };
    for line in listing.lines() {
        match line.split_at_checked(1) {
            Some(("p", pid)) if holder.pid.is_none() => holder.pid = pid.parse().ok(),
            Some(("c", name)) if holder.name.is_none() => holder.name = Some(name.to_owned()),
            _ => {}
        }
    }
    holder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_the_name_and_pid_of_the_first_process() {
        let holder = parse(8081, "p28344\ncapp\nf11\np1\nclaunchd\n");
        assert_eq!(
            holder,
            ForeignProgram { port: 8081, name: Some("app".into()), pid: Some(28_344) }
        );
    }

    #[test]
    fn should_name_no_program_when_lsof_tells_nothing() {
        assert_eq!(
            parse(8081, ""),
            ForeignProgram { port: 8081, name: None, pid: None }
        );
    }
}
