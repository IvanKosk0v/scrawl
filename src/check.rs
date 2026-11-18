//! `check-config`: validate a config file and render the result for a human.

use crate::config::Config;
use crate::error::Error;

/// Validate the config text `src` read from `path`.
///
/// Returns the success line for stdout, or an error report for stderr that names the file, the
/// 1-based line and the offending source line when the line is known.
pub fn check(path: &str, src: &str) -> Result<String, String> {
    match Config::parse(src) {
        Ok(cfg) => {
            let bindings =
                cfg.binding.len() + cfg.profile.iter().map(|p| p.binding.len()).sum::<usize>();
            Ok(format!(
                "{path}: OK ({}, {})\n",
                count(bindings, "binding", "bindings"),
                count(cfg.profile.len(), "profile", "profiles"),
            ))
        }
        Err(Error::Config {
            line: Some(l),
            message,
        }) => {
            // toml messages may span lines; keep the report to one line per fact.
            let message = message.replace('\n', "; ");
            let text = src
                .lines()
                .nth(l.saturating_sub(1))
                .unwrap_or("")
                .trim_end();
            Err(format!(
                "error: {path} line {l}: {message}\n  {l} | {text}\n"
            ))
        }
        Err(Error::Config {
            line: None,
            message,
        }) => Err(format!("error: {path}: {message}\n")),
        Err(other) => Err(format!("error: {path}: {other}\n")),
    }
}

/// `"1 binding"` / `"2 bindings"`.
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_counts_global_and_profile_bindings() {
        let src = "trigger = \"right\"\n[[binding]]\nstroke = \"L\"\naction = { key = \"a\" }\n\
                   [[profile]]\ndevice = \"x\"\n[[profile.binding]]\nstroke = \"R\"\naction = { key = \"b\" }\n";
        assert_eq!(
            check("c.toml", src),
            Ok("c.toml: OK (2 bindings, 1 profile)\n".into())
        );
    }

    #[test]
    fn error_quotes_the_offending_line() {
        let err = check("c.toml", "trigger = \"right\"\nmin_strok_px = 10   \n").unwrap_err();
        let mut lines = err.lines();
        assert!(lines
            .next()
            .unwrap()
            .starts_with("error: c.toml line 2: unknown field"));
        assert_eq!(lines.next(), Some("  2 | min_strok_px = 10"));
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn error_on_last_line_without_newline() {
        let err = check("c.toml", "trigger = \"right\"\nescape = \"left\"").unwrap_err();
        assert!(err.ends_with("  2 | escape = \"left\"\n"), "{err}");
    }

    #[test]
    fn error_without_line_names_the_file() {
        let err = check("c.toml", "").unwrap_err();
        assert!(err.starts_with("error: c.toml"), "{err}");
        assert!(err.contains("trigger"), "{err}");
    }
}
