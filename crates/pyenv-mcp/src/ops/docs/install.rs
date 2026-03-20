// ./crates/pyenv-mcp/src/ops/docs/install.rs
//! Install and uninstall instruction builders for pyenv-native and its MCP registration.

use std::path::Path;

use crate::model::{BootstrapInstallCommands, InstallInstructions, PlatformInstallCommands};

use super::super::context::build_client_config;

fn render_windows_install_root(install_root: Option<&Path>) -> String {
    install_root
        .map(|path| path.display().to_string().replace('/', "\\"))
        .unwrap_or_else(|| "$HOME\\.pyenv".to_string())
}

fn render_posix_install_root(install_root: Option<&Path>) -> String {
    install_root
        .map(|path| path.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|| "~/.pyenv".to_string())
}

pub(crate) fn build_install_instructions(
    github_repo: &str,
    install_root: Option<&Path>,
    server_name: &str,
    mcp_command: &Path,
    pyenv_root: &Path,
) -> InstallInstructions {
    let install_root_windows = render_windows_install_root(install_root);
    let install_root_posix = render_posix_install_root(install_root);

    InstallInstructions {
        github_repo: github_repo.to_string(),
        default_install_roots: PlatformInstallCommands {
            windows_powershell: "$HOME\\.pyenv".to_string(),
            linux_or_macos: "~/.pyenv".to_string(),
        },
        latest_release: PlatformInstallCommands {
            windows_powershell: format!(
                "$installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/{github_repo}/main/install.ps1 -OutFile $installer; & $installer -InstallRoot \"{install_root_windows}\""
            ),
            linux_or_macos: format!(
                "curl -fsSL https://raw.githubusercontent.com/{github_repo}/main/install.sh | sh -s -- --install-root {install_root_posix}"
            ),
        },
        pinned_release_example: PlatformInstallCommands {
            windows_powershell: format!(
                "$tag = 'vX.Y.Z'; $installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest \"https://raw.githubusercontent.com/{github_repo}/$tag/install.ps1\" -OutFile $installer; & $installer -Tag $tag -InstallRoot \"{install_root_windows}\" -Force"
            ),
            linux_or_macos: format!(
                "tag='vX.Y.Z'; curl -fsSL \"https://raw.githubusercontent.com/{github_repo}/${{tag}}/install.sh\" | sh -s -- --tag \"$tag\" --install-root {install_root_posix}"
            ),
        },
        pip_bootstrap: BootstrapInstallCommands {
            pipx: format!(
                "pipx install pyenv-native && pyenv-native install --github-repo {github_repo} --install-root {install_root_posix}"
            ),
            pip: format!(
                "python -m pip install pyenv-native && pyenv-native install --github-repo {github_repo} --install-root {install_root_posix}"
            ),
        },
        uninstall: PlatformInstallCommands {
            windows_powershell: format!(
                "$uninstaller = Join-Path $env:TEMP 'pyenv-native-uninstall.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/{github_repo}/main/uninstall.ps1 -OutFile $uninstaller; & $uninstaller -InstallRoot \"{install_root_windows}\" -RemoveRoot"
            ),
            linux_or_macos: format!(
                "curl -fsSL https://raw.githubusercontent.com/{github_repo}/main/uninstall.sh | sh -s -- --install-root {install_root_posix} --remove-root"
            ),
        },
        mcp_client_config: build_client_config(mcp_command, pyenv_root, server_name),
    }
}
