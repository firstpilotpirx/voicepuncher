//! The copies of the speech recognition service that sent the app no answer in time.
//!
//! Read from the app's log: a file-mode scenario that fails after one reports the machine's
//! load beside the log, so the failure says what the copies were busy with.

const NO_ANSWER: &str = " sent no answer in ";
const COPY: &str = "copy ";

/// The ports of the copies the app's `log` says sent no answer in time, each once, in order.
pub fn unanswered_copies(log: &str) -> Vec<u16> {
    let mut ports = Vec::new();
    for line in log.lines().filter(|l| l.contains(NO_ANSWER)) {
        let Some(copy) = line.rfind(COPY) else {
            continue;
        };
        let port: String = line[copy + COPY.len()..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(port) = port.parse::<u16>() {
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
    }
    ports
}

/// The machine's load (`report`, taken now) behind the unanswered copies `log` names, or
/// nothing when every copy answered.
pub fn load_beside(log: &str, report: impl FnOnce() -> String) -> Option<String> {
    let ports = unanswered_copies(log);
    if ports.is_empty() {
        return None;
    }
    let ports: Vec<String> = ports.iter().map(u16::to_string).collect();
    Some(format!(
        "copies {} sent no answer in time; {}",
        ports.join(", "),
        report()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "\
[WARN] speech recognition service copy (request=3 attempt 1, after 6522ms, deadline 6500ms): copy 38082 sent no answer in 6500ms — retrying in 200ms
[INFO] chunk 1 | 20.0s transcribed
[WARN] speech recognition service copy (request=4 attempt 1, after 6510ms, deadline 6500ms): copy 38081 sent no answer in 6500ms — retrying in 200ms
[WARN] speech recognition service copy (request=4 attempt 2, after 13100ms, deadline 6500ms): copy 38082 sent no answer in 6500ms — giving up
";

    #[test]
    fn should_name_each_copy_that_sent_no_answer_once_in_order() {
        assert_eq!(unanswered_copies(LOG), [38_082, 38_081]);
    }

    #[test]
    fn should_name_no_copy_when_every_request_was_answered() {
        assert!(unanswered_copies("[INFO] chunk 1 | 20.0s transcribed\n").is_empty());
    }

    #[test]
    fn should_report_the_load_behind_the_unanswered_copies() {
        let report = load_beside(LOG, || "machine load: busy".to_string()).unwrap();
        assert_eq!(
            report,
            "copies 38082, 38081 sent no answer in time; machine load: busy"
        );
    }

    #[test]
    fn should_take_no_load_report_when_every_copy_answered() {
        assert_eq!(load_beside("", || panic!("no report needed")), None);
    }
}
