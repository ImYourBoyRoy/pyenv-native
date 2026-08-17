# ./locales/de/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Nativer, plattformübergreifender Python-Versionsmanager
cli-global-about = Globale Python-Version festlegen oder anzeigen
cli-local-about = Lokale Verzeichnis-Python-Version festlegen oder anzeigen
cli-shell-about = Shell-spezifische Python-Version festlegen oder anzeigen
cli-latest-about = Neueste installierte oder bekannte Version ausgeben, die dem Präfix entspricht
cli-version-about = Aktuelle Python-Version und ihre Herkunft anzeigen
cli-version-name-about = Aktuelle Python-Version anzeigen
cli-version-origin-about = Erklären, wie die aktuelle Python-Version festgelegt ist
cli-prefix-about = Pfade anzeigen, in denen die angegebenen Python-Versionen installiert sind
cli-install-about = Python-Versionen von nativen Anbietern installieren
cli-available-about = Installierbare Python-Versionen von nativen Anbietern auflisten
cli-versions-about = Alle für pyenv verfügbaren Python-Versionen auflisten
cli-uninstall-about = Eine bestimmte Python-Version deinstallieren
cli-venv-about = Verwaltete virtuelle Umgebungen erstellen, prüfen und zuweisen
cli-pip-about = Pakete für eine Laufzeit oder venv auflisten, prüfen, installieren und aktualisieren
cli-init-about = Die Shell-Umgebung für pyenv konfigurieren
cli-gui-about = Die grafische Pyenv Native-Übersicht starten
cli-rehash-about = pyenv-Shims neu hashen (installiert ausführbare Dateien über alle Versionen)
cli-shims-about = Vorhandene pyenv-Shims auflisten
cli-prompt-about = Eine kurze Eingabeaufforderung für die aktuelle Umgebung ausgeben
cli-exec-about = Eine ausführbare Datei mit der ausgewählten Python-Version ausführen
cli-completions-about = Befehlsergänzungsskript ausgeben
cli-doctor-about = PATH, Shims und Installationsvoraussetzungen diagnostizieren
cli-config-about = pyenv-native-Konfiguration abrufen, setzen oder anzeigen
cli-self-update-about = pyenv-native über GitHub Releases aktualisieren
cli-preflight-about = Plattforminformationen und Installationsbereitschafts-preflight
cli-environment-about = Alias für preflight (OS-/Toolchain-Fakten für Agenten und Benutzer)
cli-status-about = Den umfassenden Umgebungsstatus anzeigen (Versionen, Herkunft, venvs)
cli-root-about = Das Stammverzeichnis anzeigen, in dem Versionen und Shims liegen
cli-which-about = Den vollständigen Pfad zu einer ausführbaren Datei anzeigen
cli-whence-about = Alle Python-Versionen auflisten, die die angegebene ausführbare Datei enthalten
cli-version-file-about = Die Datei erkennen, die die aktuelle pyenv-Version festlegt
cli-version-file-read-about = Den Inhalt einer .python-version-Datei lesen
cli-self-uninstall-about = pyenv-native vom System deinstallieren
cli-help-about = Hilfe für einen Befehl anzeigen
cli-commands-about = Alle verfügbaren pyenv-Befehle auflisten
cli-hooks-about = Ausführbare Hooks für einen gegebenen Befehl auflisten
cli-venv-list-about = Verwaltete virtuelle Umgebungen auflisten
cli-venv-info-about = Details einer verwalteten virtuellen Umgebung anzeigen
cli-venv-create-about = Eine verwaltete virtuelle Umgebung unter einer bestimmten Laufzeit erstellen
cli-venv-delete-about = Eine verwaltete virtuelle Umgebung entfernen
cli-venv-rename-about = Eine verwaltete virtuelle Umgebung umbenennen
cli-venv-use-about = Eine verwaltete virtuelle Umgebung dem aktuellen Verzeichnis oder global zuweisen
cli-venv-upgrade-about = Eine verwaltete virtuelle Umgebung auf eine neue Basis-Laufzeit aktualisieren
cli-pip-list-about = Installierte pip-Pakete in einer Zielumgebung auflisten
cli-pip-outdated-about = Veraltete pip-Pakete in einer Zielumgebung auflisten
cli-pip-check-about = Defekte Paketanforderungen in einer Zielumgebung prüfen
cli-pip-precheck-about = Eine Anforderungsdatei oder HTTPS-URL vor der Installation statisch vorprüfen
cli-pip-analyze-about = Python-Quellen nach Drittanbieter-Imports durchsuchen, die im Ziel fehlen
cli-pip-install-about = Pakete aus einer requirements.txt-Datei oder HTTPS-URL installieren
cli-pip-update-about = pip-Pakete in einer Zielumgebung aktualisieren
cli-config-path-about = Den Pfad zur Konfigurationsdatei anzeigen
cli-config-show-about = Die gesamte aktuelle Konfiguration ausgeben
cli-config-get-about = Den Wert eines bestimmten Konfigurationsschlüssels ausgeben
cli-config-set-about = Einen Konfigurationsschlüssel aktualisieren
cli-help-selection = AUSWAHL
cli-help-provisioning = BEREITSTELLUNG
cli-help-environment = UMGEBUNG
cli-help-interface = OBERFLÄCHE
cli-help-diagnostics = DIAGNOSE & KONFIG
cli-help-maintenance = WARTUNG
cli-help-support = UNTERSTÜTZUNG
cli-help-usage = Aufruf: pyenv <command> [<args>]
cli-help-useful = Nützliche pyenv-Befehle:
cli-help-concepts =
    KERNKONZEPTE:
      Shims: leichte Executables (`python` oder `pip`), die Befehle abfangen und an die aktuelle Version weiterleiten. Nach pip-Paketen `pyenv rehash` ausführen.
      Versions: mit `pyenv install` installierte Umgebungen unter `~/.pyenv/versions`.
      Managed envs: `~/.pyenv/venvs/<runtime>/<name>`. Bevorzugen Sie `pyenv venv create` und `pyenv venv use`.
      Discovery: `pyenv install --list 3.13` oder `pyenv available 3.13`.
      Selection: PYENV_VERSION, dann `.python-version`, dann die globale Versionsdatei.
    
    `pyenv help <command>` für Details. Docs: https://github.com/imyourboyroy/pyenv-native
