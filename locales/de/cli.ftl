# ./locales/de/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Manager für Python-Runtimes und -Umgebungen
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
cli-versions-about = Alle für enchi verfügbaren Python-Versionen auflisten
cli-upgrade-about = Installierte Runtimes auf den neuesten Patch-Stand bringen (samt venvs)
cli-uninstall-about = Eine bestimmte Python-Version deinstallieren
cli-venv-about = Verwaltete virtuelle Umgebungen erstellen, prüfen und zuweisen
cli-pip-about = Pakete für eine Laufzeit oder venv auflisten, prüfen, installieren und aktualisieren
cli-init-about = Die Shell-Umgebung für enchi konfigurieren
cli-gui-about = Die grafische Enchi-Übersicht starten
cli-rehash-about = enchi-Shims neu hashen (installiert ausführbare Dateien über alle Versionen)
cli-shims-about = Vorhandene enchi-Shims auflisten
cli-prompt-about = Eine kurze Eingabeaufforderung für die aktuelle Umgebung ausgeben
cli-exec-about = Eine ausführbare Datei mit der ausgewählten Python-Version ausführen
cli-completions-about = Befehlsergänzungsskript ausgeben
cli-doctor-about = PATH, Shims und Installationsvoraussetzungen diagnostizieren
cli-config-about = Enchi-Konfiguration abrufen, setzen oder anzeigen
cli-self-update-about = Enchi über GitHub Releases aktualisieren
cli-preflight-about = Plattforminformationen und Installationsbereitschafts-preflight
cli-environment-about = Alias für preflight (OS-/Toolchain-Fakten für Agenten und Benutzer)
cli-status-about = Den umfassenden Umgebungsstatus anzeigen (Versionen, Herkunft, venvs)
cli-root-about = Das Stammverzeichnis anzeigen, in dem Versionen und Shims liegen
cli-which-about = Den vollständigen Pfad zu einer ausführbaren Datei anzeigen
cli-whence-about = Alle Python-Versionen auflisten, die die angegebene ausführbare Datei enthalten
cli-version-file-about = Die Datei erkennen, die die aktuelle enchi-Version festlegt
cli-version-file-read-about = Den Inhalt einer .python-version-Datei lesen
cli-self-install-about = Enchi aus einem Release-Bundle installieren oder reparieren
cli-self-uninstall-about = Enchi vom System deinstallieren
cli-agent-skills-about = Den Enchi-Skill für Coding-Assistenten installieren
cli-help-about = Hilfe für einen Befehl anzeigen
cli-commands-about = Alle verfügbaren enchi-Befehle auflisten
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
cli-help-usage = Aufruf: enchi <command> [<args>]
cli-help-useful = Nützliche enchi-Befehle:
cli-help-concepts =
    KERNKONZEPTE:
      Shims: Schlanke ausführbare Dateien (wie `python` oder `pip`), die Befehle abfangen und an die richtige Python-Version weiterleiten. Nach der Installation neuer pip-Pakete `enchi rehash` ausführen.
      Versionen: Über `enchi install` installierte Python-Runtimes unter `$(enchi root)/versions`.
      Verwaltete Umgebungen: Benannte virtuelle Umgebungen unter `$(enchi root)/venvs/<runtime>/<name>`. `enchi venv create` und `enchi venv use` bevorzugen.
      Auffinden: `enchi install --list 3.13` oder `enchi available 3.13`.
      Auswahl: PYENV_VERSION, dann `.python-version`, dann die globale Versionsdatei.
    
    `enchi help <command>` zeigt ausführliche Hilfe. Doku: https://github.com/imyourboyroy/pyenv-native
