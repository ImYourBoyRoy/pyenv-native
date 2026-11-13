// ./crates/pyenv-core/src/agent_skills.rs
//! `pyenv agent-skills`: install the pyenv-native skill for coding assistants.
//!
//! The skill text is compiled into the binary, so this works from any installed `pyenv` with no
//! checkout, Rust toolchain, or network. Agents that read a skills directory (Cursor, Kiro,
//! Copilot) get the files written there. Agents that install from a repository (Gemini CLI,
//! Antigravity, Claude Code) are pointed at the public repository, and are driven directly when
//! their own CLI is on `PATH`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::command::CommandReport;
use crate::installer::home_dir;

/// The public repository the URL-based agents install from.
pub const REPO_URL: &str = "https://github.com/imyourboyroy/pyenv-native";

/// Claude Code plugin and marketplace names, from `.claude-plugin/{plugin,marketplace}.json`
/// (a test keeps them equal).
const CLAUDE_PLUGIN: &str = "pyenv-native";
const CLAUDE_MARKETPLACE: &str = "pyenv-native-skills";

/// Every file of the shipped skills: the path under an agent's skills directory, and its text.
/// A test fails when `skills/` gains a file that is not listed here.
const SKILL_FILES: &[(&str, &str)] = &[(
    "pyenv-native/SKILL.md",
    include_str!("../../../skills/pyenv-native/SKILL.md"),
)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Agent {
    Cursor,
    Claude,
    Gemini,
    Antigravity,
    Copilot,
    Kiro,
    Windsurf,
    Opencode,
}

impl Agent {
    /// What `--agent all` covers, in the order they are reported.
    pub const ALL: [Agent; 8] = [
        Agent::Cursor,
        Agent::Copilot,
        Agent::Kiro,
        Agent::Gemini,
        Agent::Antigravity,
        Agent::Claude,
        Agent::Windsurf,
        Agent::Opencode,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Agent::Cursor => "cursor",
            Agent::Claude => "claude",
            Agent::Gemini => "gemini",
            Agent::Antigravity => "antigravity",
            Agent::Copilot => "copilot",
            Agent::Kiro => "kiro",
            Agent::Windsurf => "windsurf",
            Agent::Opencode => "opencode",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Agent::Cursor => "Cursor",
            Agent::Claude => "Claude Code",
            Agent::Gemini => "Gemini CLI",
            Agent::Antigravity => "Antigravity",
            Agent::Copilot => "Copilot",
            Agent::Kiro => "Kiro",
            Agent::Windsurf => "Windsurf",
            Agent::Opencode => "OpenCode",
        }
    }
}

/// Accepted values of `--agent`.
pub const AGENT_NAMES: [&str; 9] = [
    "all",
    "cursor",
    "claude",
    "gemini",
    "antigravity",
    "copilot",
    "kiro",
    "windsurf",
    "opencode",
];

/// Resolve an `--agent` value to the agents it names (`all` is every agent).
pub fn parse_agents(value: &str) -> Result<Vec<Agent>, String> {
    if value.eq_ignore_ascii_case("all") {
        return Ok(Agent::ALL.to_vec());
    }
    Agent::ALL
        .into_iter()
        .find(|agent| agent.name().eq_ignore_ascii_case(value))
        .map(|agent| vec![agent])
        .ok_or_else(|| {
            format!(
                "pyenv: unknown agent `{value}` (expected one of: {})",
                AGENT_NAMES.join(", ")
            )
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Your home directory, for every project.
    User,
    /// The current directory only.
    Project,
}

impl Scope {
    pub const NAMES: [&'static str; 2] = ["user", "project"];

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "user" => Ok(Scope::User),
            "project" => Ok(Scope::Project),
            _ => Err(format!(
                "pyenv: unknown scope `{value}` (expected user or project)"
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::Project => "project",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AgentSkillsOptions {
    pub agents: Vec<Agent>,
    pub scope: Scope,
    /// Repository the URL-based agents install from.
    pub repo_url: String,
    /// Base of user-scope skill directories.
    pub home: PathBuf,
    /// Base of project-scope skill directories.
    pub cwd: PathBuf,
}

impl AgentSkillsOptions {
    /// Every agent, user scope, the public repository, with home and the current directory
    /// taken from the environment.
    pub fn from_environment() -> Result<Self, String> {
        let home =
            home_dir().ok_or_else(|| "pyenv: cannot resolve the home directory".to_string())?;
        let cwd = std::env::current_dir()
            .map_err(|error| format!("pyenv: cannot read the current directory: {error}"))?;
        Ok(Self {
            agents: Agent::ALL.to_vec(),
            scope: Scope::User,
            repo_url: REPO_URL.to_string(),
            home,
            cwd,
        })
    }
}

/// What happened when an agent's own CLI was asked to do the install.
pub enum ToolRun {
    /// The program is not installed.
    Missing,
    Ran {
        success: bool,
        output: String,
    },
}

/// Runs an agent's CLI. Replaceable so tests never start the real tools.
pub type ToolRunner<'a> = &'a dyn Fn(&str, &[String]) -> ToolRun;

fn run_system_tool(program: &str, args: &[String]) -> ToolRun {
    match Command::new(program).args(args).output() {
        Ok(output) => ToolRun::Ran {
            success: output.status.success(),
            output: format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        },
        Err(_) => ToolRun::Missing,
    }
}

pub fn cmd_agent_skills(options: &AgentSkillsOptions) -> CommandReport {
    let mut lines = Vec::new();
    let mut sink = |line: &str| lines.push(line.to_string());
    match run_agent_skills(options, &run_system_tool, &mut sink) {
        Ok(()) => CommandReport::success(lines),
        Err(message) => {
            let mut stderr = lines;
            stderr.push(message);
            CommandReport::failure(stderr, 1)
        }
    }
}

pub fn run_agent_skills(
    options: &AgentSkillsOptions,
    runner: ToolRunner<'_>,
    sink: &mut dyn FnMut(&str),
) -> Result<(), String> {
    sink(&format!(
        "Installing the pyenv-native agent skill | Scope: {}",
        options.scope.name()
    ));
    for agent in &options.agents {
        sink("");
        sink(&format!("[{}]", agent.name()));
        install(*agent, options, runner, sink)?;
    }
    sink("");
    sink(&format!(
        "Done. Per-agent details: {}",
        guide_url(&options.repo_url, "README.md")
    ));
    Ok(())
}

fn install(
    agent: Agent,
    options: &AgentSkillsOptions,
    runner: ToolRunner<'_>,
    sink: &mut dyn FnMut(&str),
) -> Result<(), String> {
    match agent {
        Agent::Cursor | Agent::Kiro | Agent::Copilot => {
            let Some(dest) = skills_dir(agent, options.scope, &options.home, &options.cwd) else {
                sink(
                    "  Copilot: project scope only (.github/skills). Re-run with --scope project from a repo root.",
                );
                return Ok(());
            };
            sink(&format!("  {} -> {}", agent.title(), dest.display()));
            for name in write_skills(&dest)? {
                sink(&format!("    + {name}"));
            }
        }
        Agent::Gemini => {
            let mut args = vec![
                "skills".to_string(),
                "install".to_string(),
                git_url(&options.repo_url),
                "--path".to_string(),
                "skills".to_string(),
            ];
            if options.scope == Scope::Project {
                args.extend(["--scope".to_string(), "workspace".to_string()]);
            }
            run_or_show(agent.title(), "gemini", &args, runner, sink);
        }
        Agent::Antigravity => {
            let args = vec![
                "plugin".to_string(),
                "install".to_string(),
                options.repo_url.clone(),
            ];
            run_or_show(agent.title(), "agy", &args, runner, sink);
        }
        Agent::Claude => {
            sink("  Claude Code:");
            sink(&format!("    /plugin marketplace add {}", options.repo_url));
            sink(&format!(
                "    /plugin install {CLAUDE_PLUGIN}@{CLAUDE_MARKETPLACE}"
            ));
            sink(&format!(
                "    Docs: {}",
                guide_url(&options.repo_url, "claude-code.md")
            ));
        }
        Agent::Windsurf => {
            sink("  Windsurf: copy skill content to .windsurfrules or Global Rules");
            sink(&format!(
                "    Docs: {}",
                guide_url(&options.repo_url, "windsurf.md")
            ));
        }
        Agent::Opencode => {
            sink("  OpenCode: open the workspace with AGENTS.md + skills/ from the repository");
            sink(&format!("    Repository: {}", options.repo_url));
            sink(&format!(
                "    Docs: {}",
                guide_url(&options.repo_url, "opencode.md")
            ));
        }
    }
    Ok(())
}

/// Where an agent reads skills from, or `None` when that scope is unsupported (Copilot has no
/// user-level skills directory).
fn skills_dir(agent: Agent, scope: Scope, home: &Path, cwd: &Path) -> Option<PathBuf> {
    let (base, dir) = match (agent, scope) {
        (Agent::Cursor, Scope::User) => (home, ".cursor/skills"),
        (Agent::Cursor, Scope::Project) => (cwd, ".cursor/skills"),
        (Agent::Kiro, Scope::User) => (home, ".kiro/skills"),
        (Agent::Kiro, Scope::Project) => (cwd, ".kiro/skills"),
        (Agent::Copilot, Scope::Project) => (cwd, ".github/skills"),
        _ => return None,
    };
    Some(base.join(dir))
}

/// Write the embedded skills under `dest`, replacing a previous copy of each skill. Returns the
/// skill names.
fn write_skills(dest: &Path) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = Vec::new();
    for (relative, text) in SKILL_FILES {
        let skill = relative.split('/').next().unwrap_or(relative).to_string();
        if !names.contains(&skill) {
            let existing = dest.join(&skill);
            if existing.exists() {
                fs::remove_dir_all(&existing)
                    .map_err(|error| format!("pyenv: replacing {}: {error}", existing.display()))?;
            }
            names.push(skill);
        }
        let target = dest.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("pyenv: creating {}: {error}", parent.display()))?;
        }
        fs::write(&target, text)
            .map_err(|error| format!("pyenv: writing {}: {error}", target.display()))?;
    }
    Ok(names)
}

/// Link to a page of the agent-skills guides in the repository.
fn guide_url(repo_url: &str, page: &str) -> String {
    let base = repo_url.trim_end_matches('/').trim_end_matches(".git");
    format!("{base}/blob/main/docs/agent-skills/{page}")
}

fn git_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    if url.ends_with(".git") {
        url.to_string()
    } else {
        format!("{url}.git")
    }
}

/// Run `program args` when it is installed; otherwise print the command to run by hand.
fn run_or_show(
    label: &str,
    program: &str,
    args: &[String],
    runner: ToolRunner<'_>,
    sink: &mut dyn FnMut(&str),
) {
    match runner(program, args) {
        ToolRun::Ran { success, output } => {
            for line in output.lines().filter(|line| !line.trim().is_empty()) {
                sink(&format!("  {line}"));
            }
            if !success {
                sink(&format!(
                    "  {label}: `{program}` did not finish successfully"
                ));
            }
        }
        ToolRun::Missing => {
            sink(&format!("  {label}: '{program}' not in PATH — manual:"));
            sink(&format!("    {program} {}", args.join(" ")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn options(dir: &Path, agents: Vec<Agent>, scope: Scope) -> AgentSkillsOptions {
        AgentSkillsOptions {
            agents,
            scope,
            repo_url: REPO_URL.to_string(),
            home: dir.join("home"),
            cwd: dir.join("project"),
        }
    }

    fn collect(
        options: &AgentSkillsOptions,
        runner: ToolRunner<'_>,
    ) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        run_agent_skills(options, runner, &mut |line| lines.push(line.to_string()))?;
        Ok(lines)
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn agent_values_parse_case_insensitively_and_all_expands() {
        assert_eq!(parse_agents("ALL").unwrap().len(), Agent::ALL.len());
        assert_eq!(parse_agents("Cursor").unwrap(), vec![Agent::Cursor]);
        assert!(parse_agents("emacs").unwrap_err().contains("unknown agent"));
        for name in AGENT_NAMES {
            assert!(parse_agents(name).is_ok(), "{name}");
        }
        assert!(Scope::parse("workspace").is_err());
        assert_eq!(Scope::parse("Project").unwrap(), Scope::Project);
    }

    #[test]
    fn skill_directories_depend_on_agent_and_scope() {
        let home = Path::new("/home/u");
        let cwd = Path::new("/work/app");
        assert_eq!(
            skills_dir(Agent::Cursor, Scope::User, home, cwd),
            Some(home.join(".cursor/skills"))
        );
        assert_eq!(
            skills_dir(Agent::Kiro, Scope::Project, home, cwd),
            Some(cwd.join(".kiro/skills"))
        );
        assert_eq!(
            skills_dir(Agent::Copilot, Scope::Project, home, cwd),
            Some(cwd.join(".github/skills"))
        );
        assert_eq!(skills_dir(Agent::Copilot, Scope::User, home, cwd), None);
    }

    #[test]
    fn installing_writes_the_skill_and_replaces_a_stale_copy() {
        let dir = tempfile::tempdir().unwrap();
        let stale = dir.path().join("home/.cursor/skills/pyenv-native");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join("old.md"), "old").unwrap();

        let opts = options(dir.path(), vec![Agent::Cursor], Scope::User);
        let lines = collect(&opts, &|_, _| ToolRun::Missing).unwrap();

        let skill = fs::read_to_string(stale.join("SKILL.md")).unwrap();
        assert!(skill.contains("pyenv-native"), "{skill}");
        assert!(!stale.join("old.md").exists());
        assert!(lines.iter().any(|line| line == "    + pyenv-native"));
    }

    #[test]
    fn project_scope_writes_under_the_current_directory() {
        let dir = tempfile::tempdir().unwrap();
        let opts = options(
            dir.path(),
            vec![Agent::Copilot, Agent::Kiro],
            Scope::Project,
        );
        collect(&opts, &|_, _| ToolRun::Missing).unwrap();
        assert!(
            dir.path()
                .join("project/.github/skills/pyenv-native/SKILL.md")
                .is_file()
        );
        assert!(
            dir.path()
                .join("project/.kiro/skills/pyenv-native/SKILL.md")
                .is_file()
        );
        assert!(!dir.path().join("home").exists());
    }

    #[test]
    fn copilot_user_scope_explains_instead_of_writing() {
        let dir = tempfile::tempdir().unwrap();
        let opts = options(dir.path(), vec![Agent::Copilot], Scope::User);
        let lines = collect(&opts, &|_, _| ToolRun::Missing).unwrap();
        assert!(lines.iter().any(|line| line.contains("project scope only")));
        assert!(!dir.path().join("home").exists());
    }

    #[test]
    fn url_agents_use_the_repository_not_a_checkout() {
        let dir = tempfile::tempdir().unwrap();
        let calls = RefCell::new(Vec::new());
        let opts = options(
            dir.path(),
            vec![Agent::Gemini, Agent::Antigravity],
            Scope::Project,
        );
        let runner = |program: &str, args: &[String]| {
            calls
                .borrow_mut()
                .push(format!("{program} {}", args.join(" ")));
            ToolRun::Ran {
                success: true,
                output: "installed\n".to_string(),
            }
        };
        let lines = collect(&opts, &runner).unwrap();
        assert_eq!(
            *calls.borrow(),
            [
                format!("gemini skills install {REPO_URL}.git --path skills --scope workspace"),
                format!("agy plugin install {REPO_URL}"),
            ]
        );
        assert!(lines.iter().any(|line| line == "  installed"));
    }

    #[test]
    fn missing_agent_clis_print_the_manual_command() {
        let dir = tempfile::tempdir().unwrap();
        let opts = options(dir.path(), vec![Agent::Gemini], Scope::User);
        let lines = collect(&opts, &|_, _| ToolRun::Missing).unwrap();
        assert!(
            lines
                .iter()
                .any(|line| line.contains("'gemini' not in PATH"))
        );
        assert!(lines.iter().any(|line| {
            line == &format!("    gemini skills install {REPO_URL}.git --path skills")
        }));
    }

    #[test]
    fn a_failing_agent_cli_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let opts = options(dir.path(), vec![Agent::Antigravity], Scope::User);
        let lines = collect(&opts, &|_, _| ToolRun::Ran {
            success: false,
            output: "boom\n".to_string(),
        })
        .unwrap();
        assert!(lines.iter().any(|line| line == "  boom"));
        assert!(lines.iter().any(|line| line.contains("did not finish")));
    }

    #[test]
    fn guide_links_point_at_the_repository_docs() {
        assert_eq!(
            guide_url("https://github.com/o/r.git/", "cursor.md"),
            "https://github.com/o/r/blob/main/docs/agent-skills/cursor.md"
        );
    }

    #[test]
    fn git_urls_gain_one_dot_git_suffix() {
        assert_eq!(
            git_url("https://github.com/o/r"),
            "https://github.com/o/r.git"
        );
        assert_eq!(
            git_url("https://github.com/o/r.git"),
            "https://github.com/o/r.git"
        );
        assert_eq!(
            git_url("https://github.com/o/r/"),
            "https://github.com/o/r.git"
        );
    }

    #[test]
    fn every_repository_skill_file_is_embedded_and_current() {
        let skills = repo_root().join("skills");
        let mut on_disk = Vec::new();
        collect_files(&skills, &skills, &mut on_disk);
        on_disk.sort();
        let mut embedded: Vec<&str> = SKILL_FILES.iter().map(|(path, _)| *path).collect();
        embedded.sort();
        assert_eq!(
            on_disk, embedded,
            "skills/ and SKILL_FILES in agent_skills.rs list different files"
        );
        for (relative, text) in SKILL_FILES {
            let current = fs::read_to_string(skills.join(relative)).unwrap();
            assert_eq!(&current, text, "{relative} is stale in the binary");
        }
    }

    fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_files(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    #[test]
    fn claude_names_match_the_plugin_manifests() {
        let read = |file: &str| -> String {
            let text = fs::read_to_string(repo_root().join(".claude-plugin").join(file)).unwrap();
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            value["name"].as_str().unwrap().to_string()
        };
        assert_eq!(read("plugin.json"), CLAUDE_PLUGIN);
        assert_eq!(read("marketplace.json"), CLAUDE_MARKETPLACE);
    }
}
