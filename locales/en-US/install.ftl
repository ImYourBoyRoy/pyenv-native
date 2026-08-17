# ./locales/en-US/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = Downloading { $version }
install-extracting = Extracting release bundle
install-installing = Installing into { $path }
install-done = pyenv-native installed. Open a new terminal, then run `pyenv doctor`.
install-failed = Installation failed
install-lang-help = Interface language for installer messages
install-summary-title = pyenv-native install summary
install-network-summary-title = pyenv-native network install summary
install-summary-blurb = This will create or update a portable pyenv-native installation under the selected root.
install-summary-blurb-detail = It installs pyenv plus the agent-friendly pyenv-mcp server and the GUI companion when available, writes an install log, and runs basic sanity checks.
install-network-blurb = This will download a published pyenv-native bundle, verify its SHA-256 checksum, and install it into the selected portable root.
install-profile-yes = Your shell profile will be updated so future sessions can find pyenv-native automatically.
install-profile-yes-pwsh = Your PowerShell profile will be updated so future sessions can find pyenv-native automatically.
install-profile-no = No shell profile changes will be made.
install-profile-no-pwsh = No PowerShell profile changes will be made.
install-continue = Continue with install? [y/N]:
install-need-yes = Confirmation is required for interactive installs. Re-run with --yes for non-interactive use.
install-need-yes-pwsh = Confirmation is required for interactive installs. Re-run with -Yes for non-interactive use.
install-cancelled = Install cancelled.
install-installed-to = Installed pyenv-native to { $path }
install-installed-command = Installed command: { $path }
install-installed-mcp = Installed MCP server: { $path }
install-mcp-helper = MCP config helper: { $command }
install-installed-gui = Installed GUI: { $path }
install-log-file = Log file: { $path }
