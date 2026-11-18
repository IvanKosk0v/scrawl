//! Optional feedback after a recognized stroke (`feedback = "notify"`).

use crate::config::Feedback;
use crate::error::Error;

/// The `notify-send` command line announcing `stroke` (or that nothing is bound to it).
pub fn notify_argv(stroke: &str, matched: bool) -> Vec<String> {
    let body = if matched {
        stroke.to_owned()
    } else {
        format!("{stroke}: no match")
    };
    [
        "notify-send",
        "--app-name=strokebind",
        "--expire-time=1500",
        "strokebind",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([body])
    .collect()
}

/// Give feedback for `stroke` according to `mode`, starting commands through `spawn`.
pub fn give(
    mode: Feedback,
    stroke: &str,
    matched: bool,
    spawn: impl FnOnce(&[String]) -> Result<(), Error>,
) -> Result<(), Error> {
    match mode {
        Feedback::Off => Ok(()),
        Feedback::Notify => spawn(&notify_argv(stroke, matched)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake runner: records each argv it is asked to start.
    fn record(
        mode: Feedback,
        stroke: &str,
        matched: bool,
    ) -> (Result<(), Error>, Vec<Vec<String>>) {
        let mut spawned = Vec::new();
        let res = give(mode, stroke, matched, |argv| {
            spawned.push(argv.to_vec());
            Ok(())
        });
        (res, spawned)
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn notify_spawns_notify_send_with_the_stroke() {
        let (res, spawned) = record(Feedback::Notify, "L-U", true);
        assert!(res.is_ok());
        assert_eq!(
            spawned,
            vec![strings(&[
                "notify-send",
                "--app-name=strokebind",
                "--expire-time=1500",
                "strokebind",
                "L-U",
            ])]
        );
    }

    #[test]
    fn notify_reports_no_match() {
        let (_, spawned) = record(Feedback::Notify, "DR", false);
        assert_eq!(spawned.len(), 1);
        assert_eq!(spawned[0].last().map(String::as_str), Some("DR: no match"));
    }

    #[test]
    fn off_never_spawns() {
        for matched in [true, false] {
            let (res, spawned) = record(Feedback::Off, "L", matched);
            assert!(res.is_ok());
            assert!(spawned.is_empty());
        }
    }

    #[test]
    fn spawn_error_is_returned() {
        let res = give(Feedback::Notify, "L", true, |_| Err(Error::EmptyCommand));
        assert!(matches!(res, Err(Error::EmptyCommand)));
    }
}
