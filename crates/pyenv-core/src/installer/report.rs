// ./crates/pyenv-core/src/installer/report.rs
//! Progress reporting for install/update/uninstall: live console lines plus an install log file.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Sink for human-readable output lines (the CLI prints them live; the GUI collects them).
pub type LineSink<'a> = &'a mut dyn FnMut(&str);

/// Writes `[pyenv-native][LEVEL] message` lines to a sink and, once attached, a log file.
pub(crate) struct Reporter<'a> {
    sink: LineSink<'a>,
    log: Option<(PathBuf, File)>,
}

impl<'a> Reporter<'a> {
    pub(crate) fn new(sink: LineSink<'a>) -> Self {
        Self { sink, log: None }
    }

    /// Start (truncate) the log file at `path`. Later lines are mirrored into it.
    pub(crate) fn attach_log(&mut self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        self.log = Some((path.to_path_buf(), file));
        Ok(())
    }

    /// Emit an unprefixed line (summaries, hints).
    pub(crate) fn plain(&mut self, line: &str) {
        (self.sink)(line);
        if let Some((_, file)) = self.log.as_mut() {
            let _ = writeln!(file, "{line}");
        }
    }

    fn leveled(&mut self, level: &str, message: &str) {
        self.plain(&format!("[pyenv-native][{level}] {message}"));
    }

    pub(crate) fn info(&mut self, message: &str) {
        self.leveled("INFO", message);
    }

    pub(crate) fn warn(&mut self, message: &str) {
        self.leveled("WARN", message);
    }

    pub(crate) fn error(&mut self, message: &str) {
        self.leveled("ERROR", message);
    }
}

/// `YYYYMMDD-HHMMSS` in UTC, for log file names (no time-zone database needed).
pub(crate) fn utc_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    format_utc_stamp(seconds)
}

fn format_utc_stamp(epoch_seconds: u64) -> String {
    let days = (epoch_seconds / 86_400) as i64;
    let secs = epoch_seconds % 86_400;
    // Civil-from-days (Howard Hinnant), valid for the proleptic Gregorian calendar.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        secs / 3_600,
        (secs % 3_600) / 60,
        secs % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn formats_known_instants() {
        assert_eq!(format_utc_stamp(0), "19700101-000000");
        assert_eq!(format_utc_stamp(951_782_400), "20000229-000000");
        assert_eq!(format_utc_stamp(1_700_000_000), "20231114-221320");
    }

    #[test]
    fn mirrors_lines_into_the_log_file() {
        let temp = TempDir::new().expect("tempdir");
        let log = temp.path().join("logs").join("install.log");
        let mut seen = Vec::new();
        {
            let mut sink = |line: &str| seen.push(line.to_string());
            let mut reporter = Reporter::new(&mut sink);
            reporter.info("before log");
            reporter.attach_log(&log).expect("attach");
            reporter.warn("careful");
            reporter.plain("plain line");
        }
        assert_eq!(seen.len(), 3);
        let text = fs::read_to_string(&log).expect("log");
        assert_eq!(text, "[pyenv-native][WARN] careful\nplain line\n");
    }
}
