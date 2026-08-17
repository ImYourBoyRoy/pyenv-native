# ./locales/en-US/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Native-first, cross-platform Python version manager
cli-global-about = Set or show the global Python version
cli-local-about = Set or show the local directory Python version
cli-shell-about = Set or show the shell-specific Python version
cli-latest-about = Print the latest installed or known version matching the prefix
cli-version-about = Show the current Python version and its origin
cli-version-name-about = Show the current Python version
cli-version-origin-about = Explain how the current Python version is set
cli-prefix-about = Display paths where the given Python versions are installed
cli-install-about = Install Python versions from native providers
cli-available-about = List installable Python versions from native providers
cli-versions-about = List all Python versions available to pyenv
cli-uninstall-about = Uninstall a specific Python version
cli-venv-about = Create, inspect, and assign managed virtual environments
cli-pip-about = List, check, install, and update packages for a runtime or venv
cli-init-about = Configure the shell environment for pyenv
cli-gui-about = Launch the Pyenv Native GUI dashboard
cli-rehash-about = Rehash pyenv shims (installs executables across all versions)
cli-shims-about = List existing pyenv shims
cli-prompt-about = Print a concise prompt string for the current environment
cli-exec-about = Run an executable with the selected Python version
cli-completions-about = Print command completion script
cli-doctor-about = Diagnose PATH, shims, and install prerequisites
cli-config-about = Get, set, or show pyenv-native configuration
cli-self-update-about = Update pyenv-native from GitHub Releases
cli-preflight-about = Platform intelligence and install-readiness preflight
cli-environment-about = Alias for preflight (OS/toolchain facts for agents and users)
cli-status-about = Show the comprehensive environment status (versions, origins, venvs)
cli-root-about = Display the root directory where versions and shims are kept
cli-which-about = Display the full path to an executable
cli-whence-about = List all Python versions that contain the given executable
cli-version-file-about = Detect the file that sets the current pyenv version
cli-version-file-read-about = Read the contents of a .python-version file
cli-self-uninstall-about = Uninstall pyenv-native from your system
cli-help-about = Display help for a command
cli-commands-about = List all available pyenv commands
cli-hooks-about = List executable hooks for a given command
cli-venv-list-about = List managed virtual environments
cli-venv-info-about = Show details for a managed virtual environment
cli-venv-create-about = Create a managed virtual environment under a specific runtime
cli-venv-delete-about = Remove a managed virtual environment
cli-venv-rename-about = Rename a managed virtual environment
cli-venv-use-about = Assign a managed virtual environment to the current directory or globally
cli-venv-upgrade-about = Upgrade a managed virtual environment to a new base runtime
cli-pip-list-about = List installed pip packages in a target environment
cli-pip-outdated-about = List outdated pip packages in a target environment
cli-pip-check-about = Check for broken package requirements in a target environment
cli-pip-precheck-about = Statically pre-check a requirements file or HTTPS URL before install
cli-pip-analyze-about = Scan Python sources for third-party imports missing from the target
cli-pip-install-about = Install packages from a requirements.txt file or HTTPS URL
cli-pip-update-about = Update pip packages inside a target environment
cli-config-path-about = Show the path to the config file
cli-config-show-about = Print all current configuration
cli-config-get-about = Print the value of a specific config key
cli-config-set-about = Update a config key
cli-help-selection = SELECTION
cli-help-provisioning = PROVISIONING
cli-help-environment = ENVIRONMENT
cli-help-interface = INTERFACE
cli-help-diagnostics = DIAGNOSTICS & CONFIG
cli-help-maintenance = MAINTENANCE
cli-help-support = SUPPORT
cli-help-usage = Usage: pyenv <command> [<args>]
cli-help-useful = Some useful pyenv commands are:
cli-help-concepts =
    CORE CONCEPTS:
      Shims: Lightweight executables (like `python` or `pip`) that intercept your commands and route them to the correct Python version. Run `pyenv rehash` after installing new pip packages.
      Versions: Python environments installed via `pyenv install`, under `~/.pyenv/versions`.
      Managed envs: Named virtual environments under `~/.pyenv/venvs/<runtime>/<name>`. Prefer `pyenv venv create` and `pyenv venv use`.
      Discovery: `pyenv install --list 3.13` or `pyenv available 3.13`.
      Selection: PYENV_VERSION, then `.python-version`, then the global version file.

    Run `pyenv help <command>` for detailed help. Docs: https://github.com/imyourboyroy/pyenv-native
