# ./locales/de/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = pyenv-native { $version } wird heruntergeladen
install-extracting = Release-Paket wird entpackt
install-installing = Installation nach { $path }
install-done = pyenv-native installiert. Ein neues Terminal öffnen, dann `pyenv doctor` ausführen.
install-failed = Installation fehlgeschlagen
install-lang-help = Oberflächensprache für Installer-Meldungen
install-summary-title = pyenv-native-Installationszusammenfassung
install-network-summary-title = pyenv-native-Netzwerkinstallationszusammenfassung
install-summary-blurb = Damit wird eine portable pyenv-native-Installation unter dem gewählten Stamm erstellt oder aktualisiert.
install-summary-blurb-detail = Es installiert pyenv sowie den agentenfreundlichen pyenv-mcp-Server und den GUI-Begleiter, falls verfügbar, schreibt ein Installationsprotokoll und führt grundlegende Prüfungen aus.
install-network-blurb = Damit wird ein veröffentlichtes pyenv-native-Paket heruntergeladen, die SHA-256-Prüfsumme geprüft und in den gewählten portablen Stamm installiert.
install-profile-yes = Ihr Shell-Profil wird aktualisiert, damit künftige Sitzungen pyenv-native automatisch finden.
install-profile-yes-pwsh = Ihr PowerShell-Profil wird aktualisiert, damit künftige Sitzungen pyenv-native automatisch finden.
install-profile-no = Am Shell-Profil werden keine Änderungen vorgenommen.
install-profile-no-pwsh = Am PowerShell-Profil werden keine Änderungen vorgenommen.
install-continue = Mit der Installation fortfahren? [y/N]:
install-need-yes = Interaktive Installationen erfordern eine Bestätigung. Für nicht interaktive Nutzung mit --yes erneut ausführen.
install-need-yes-pwsh = Interaktive Installationen erfordern eine Bestätigung. Für nicht interaktive Nutzung mit -Yes erneut ausführen.
install-cancelled = Installation abgebrochen.
install-installed-to = pyenv-native nach { $path } installiert
install-installed-command = Installierter Befehl: { $path }
install-installed-mcp = Installierter MCP-Server: { $path }
install-mcp-helper = MCP-Konfigurationshelfer: { $command }
install-installed-gui = Installierte GUI: { $path }
install-log-file = Protokolldatei: { $path }
