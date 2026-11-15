# ./locales/en-US/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Python Runtime & Environment Manager
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
cli-versions-about = List all Python versions available to enchi
cli-upgrade-about = Move installed runtimes to the newest patch release (and their venvs)
cli-uninstall-about = Uninstall a specific Python version
cli-venv-about = Create, inspect, and assign managed virtual environments
cli-pip-about = List, check, install, and update packages for a runtime or venv
cli-init-about = Configure the shell environment for enchi
cli-gui-about = Launch the Enchi GUI dashboard
cli-rehash-about = Rehash enchi shims (installs executables across all versions)
cli-shims-about = List existing enchi shims
cli-prompt-about = Print a concise prompt string for the current environment
cli-exec-about = Run an executable with the selected Python version
cli-completions-about = Print command completion script
cli-doctor-about = Diagnose PATH, shims, and install prerequisites
cli-config-about = Get, set, or show Enchi configuration
cli-self-update-about = Update Enchi from GitHub Releases
cli-preflight-about = Platform intelligence and install-readiness preflight
cli-environment-about = Alias for preflight (OS/toolchain facts for agents and users)
cli-status-about = Show the comprehensive environment status (versions, origins, venvs)
cli-root-about = Display the root directory where versions and shims are kept
cli-which-about = Display the full path to an executable
cli-whence-about = List all Python versions that contain the given executable
cli-version-file-about = Detect the file that sets the current enchi version
cli-version-file-read-about = Read the contents of a .python-version file
cli-self-install-about = Install or repair Enchi from a release bundle
cli-self-uninstall-about = Uninstall Enchi from your system
cli-agent-skills-about = Install the Enchi skill for coding assistants
cli-help-about = Display help for a command
cli-commands-about = List all available enchi commands
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
cli-help-usage = Usage: enchi <command> [<args>]
cli-help-useful = Some useful enchi commands are:
cli-help-concepts =
    CORE CONCEPTS:
      Shims: Lightweight executables (like `python` or `pip`) that intercept your commands and route them to the correct Python version. Run `enchi rehash` after installing new pip packages.
      Versions: Python environments installed via `enchi install`, under `$(enchi root)/versions`.
      Managed envs: Named virtual environments under `$(enchi root)/venvs/<runtime>/<name>`. Prefer `enchi venv create` and `enchi venv use`.
      Discovery: `enchi install --list 3.13` or `enchi available 3.13`.
      Selection: PYENV_VERSION, then `.python-version`, then the global version file.

    Run `enchi help <command>` for detailed help. Docs: https://github.com/imyourboyroy/pyenv-native
