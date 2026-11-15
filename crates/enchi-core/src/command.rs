// ./crates/enchi-core/src/command.rs
//! Common command result types shared across the native pyenv core.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReport {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub exit_code: i32,
}

impl CommandReport {
    pub fn success(stdout: Vec<String>) -> Self {
        Self {
            stdout,
            stderr: Vec::new(),
            exit_code: 0,
        }
    }

    pub fn empty_success() -> Self {
        Self::success(Vec::new())
    }

    pub fn success_one(line: impl Into<String>) -> Self {
        Self::success(vec![line.into()])
    }

    pub fn failure(stderr: Vec<String>, exit_code: i32) -> Self {
        Self {
            stdout: Vec::new(),
            stderr,
            exit_code,
        }
    }

    /// Re-express this report as one JSON document on stdout, for `--json`. Nothing goes to stderr
    /// and the exit code is unchanged, so scripts can rely on either.
    ///
    /// Shape: `{ schema, command, ok, exit_code, dry_run, data?, output: [...], errors: [...] }`.
    /// `data` carries typed results where a command has them; `output` always holds the text the
    /// command would have printed, so no information is lost.
    pub fn into_json(
        self,
        command: &str,
        dry_run: bool,
        data: Option<serde_json::Value>,
    ) -> CommandReport {
        let mut document = serde_json::json!({
            "schema": RESULT_SCHEMA,
            "command": command,
            "ok": self.exit_code == 0,
            "exit_code": self.exit_code,
            "dry_run": dry_run,
            "output": self.stdout,
            "errors": self.stderr,
        });
        if let (Some(data), Some(map)) = (data, document.as_object_mut()) {
            map.insert("data".to_string(), data);
        }
        let rendered = serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_string());
        CommandReport {
            stdout: rendered.lines().map(ToOwned::to_owned).collect(),
            stderr: Vec::new(),
            exit_code: self.exit_code,
        }
    }
}

/// Schema tag of the machine-readable result envelope produced by [`CommandReport::into_json`].
pub const RESULT_SCHEMA: &str = "enchi.result/1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_envelope_keeps_output_errors_and_exit_code() {
        let report = CommandReport {
            stdout: vec!["a".to_string()],
            stderr: vec!["boom".to_string()],
            exit_code: 3,
        }
        .into_json("uninstall", true, Some(serde_json::json!({"n": 1})));
        assert_eq!(report.exit_code, 3);
        assert!(report.stderr.is_empty());
        let value: serde_json::Value = serde_json::from_str(&report.stdout.join("\n")).unwrap();
        assert_eq!(value["schema"], RESULT_SCHEMA);
        assert_eq!(value["command"], "uninstall");
        assert_eq!(value["ok"], false);
        assert_eq!(value["dry_run"], true);
        assert_eq!(value["output"][0], "a");
        assert_eq!(value["errors"][0], "boom");
        assert_eq!(value["data"]["n"], 1);
    }

    #[test]
    fn json_envelope_omits_data_when_absent() {
        let report = CommandReport::success_one("hi").into_json("root", false, None);
        let value: serde_json::Value = serde_json::from_str(&report.stdout.join("\n")).unwrap();
        assert_eq!(value["ok"], true);
        assert!(value.get("data").is_none());
    }
}
