# ./locales/en-US/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = Downloading { $version }
install-extracting = Extracting release bundle
install-installing = Installing into { $path }
install-done = Enchi installed. Open a new terminal, then run `enchi doctor`.
install-failed = Installation failed
install-lang-help = Interface language for installer messages
install-summary-title = Enchi install summary
install-network-summary-title = Enchi network install summary
install-summary-blurb = This will create or update a portable Enchi installation under the selected root.
install-summary-blurb-detail = It installs enchi plus the agent-friendly enchi-mcp server and the GUI companion when available, writes an install log, and runs basic sanity checks.
install-network-blurb = This will download a published Enchi bundle, verify its SHA-256 checksum, and install it into the selected portable root.
install-profile-yes = Your shell profile will be updated so future sessions can find Enchi automatically.
install-profile-yes-pwsh = Your PowerShell profile will be updated so future sessions can find Enchi automatically.
install-profile-no = No shell profile changes will be made.
install-profile-no-pwsh = No PowerShell profile changes will be made.
install-continue = Continue with install? [y/N]:
install-need-yes = Confirmation is required for interactive installs. Re-run with --yes for non-interactive use.
install-need-yes-pwsh = Confirmation is required for interactive installs. Re-run with -Yes for non-interactive use.
install-cancelled = Install cancelled.
install-installed-to = Installed Enchi to { $path }
install-installed-command = Installed command: { $path }
install-installed-mcp = Installed MCP server: { $path }
install-mcp-helper = MCP config helper: { $command }
install-installed-gui = Installed GUI: { $path }
install-log-file = Log file: { $path }
