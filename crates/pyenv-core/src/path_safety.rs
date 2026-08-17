// ./crates/pyenv-core/src/path_safety.rs
//! Lexical path containment helpers so version names, venv specs, and uninstall
//! arguments cannot walk out of PYENV_ROOT with `..` or absolute paths.

use std::path::{Component, Path};

/// Returns true when `value` is a relative path with no escaping `..` prefix,
/// no root/prefix components, and at least one real path component.
pub fn is_safe_relative_path(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return false;
    }

    let mut depth = 0usize;
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }

    depth >= 1
}

/// Returns true when `candidate` stays strictly inside `root` after lexical
/// normalization of `.` and `..` components. Neither path is canonicalized, so
/// this does not follow symlinks; it is the check used before destructive
/// deletes and venv resolution.
pub fn path_stays_under(root: &Path, candidate: &Path) -> bool {
    let root_stack = normalize_components(root);
    let candidate_stack = normalize_components(candidate);
    if root_stack.is_empty() || candidate_stack.len() <= root_stack.len() {
        return false;
    }
    candidate_stack.starts_with(&root_stack)
}

fn normalize_components(path: &Path) -> Vec<Component<'_>> {
    let mut stack = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = stack.pop();
            }
            other => stack.push(other),
        }
    }
    stack
}

#[cfg(test)]
mod tests {
    use super::{is_safe_relative_path, path_stays_under};
    use std::path::Path;

    #[test]
    fn safe_relative_path_accepts_version_and_env_specs() {
        assert!(is_safe_relative_path("3.14.7"));
        assert!(is_safe_relative_path("3.14.7t"));
        assert!(is_safe_relative_path("3.14.7/envs/api"));
        assert!(is_safe_relative_path("pypy3.11-7.3.20"));
    }

    #[test]
    fn safe_relative_path_rejects_escape_and_empty() {
        assert!(!is_safe_relative_path(""));
        assert!(!is_safe_relative_path("."));
        assert!(!is_safe_relative_path(".."));
        assert!(!is_safe_relative_path("../"));
        assert!(!is_safe_relative_path("../versions"));
        assert!(!is_safe_relative_path("foo/../../etc"));
        assert!(!is_safe_relative_path("/etc/passwd"));
    }

    #[test]
    fn path_stays_under_rejects_parent_join() {
        let root = Path::new("pyenv-root").join("versions");
        assert!(path_stays_under(&root, &root.join("3.14.7")));
        assert!(!path_stays_under(&root, &root.join("..")));
        assert!(!path_stays_under(&root, &root.join("../venvs")));
        assert!(!path_stays_under(&root, &root));
        assert!(!path_stays_under(
            &root,
            &root.join("3.14.7").join("..").join("..").join("outside")
        ));
    }
}
