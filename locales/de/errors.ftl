# ./locales/de/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: Home-Verzeichnis für PYENV_ROOT kann nicht ermittelt werden
error-invalid-directory = enchi: Arbeitsverzeichnis kann nicht zu `{ $path }` gewechselt werden
error-invalid-version = enchi: ungültige Version `{ $version }` in `{ $path }` ignoriert
error-no-local-version = enchi: keine lokale Version für dieses Verzeichnis konfiguriert
error-version-not-installed =
    enchi: Version `{ $version }` ist nicht installiert (gesetzt von { $origin })
    Hinweis: mit `enchi install { $version }` installieren oder mit `enchi versions` installierte Versionen anzeigen
error-unknown-config-key = enchi: unbekannter Konfigurationsschlüssel `{ $key }`
error-invalid-config-value = enchi: ungültiger Wert `{ $value }` für Konfigurationsschlüssel `{ $key }`
error-version-already-installed = enchi: Version `{ $version }` ist bereits installiert
error-unknown-version = enchi: keine bekannten Versionen entsprechen `{ $version }`
error-unsupported-install-target = enchi: die Installationskomponente unterstützt `{ $version }` auf dieser Plattform nicht
error-missing-install-version = enchi: Installationsvorgang erfordert mindestens ein Versionsargument
error-missing-python-build = enchi: python-build-Komponente nicht gefunden; install.python_build_path setzen oder python-build zum PATH hinzufügen
error-checksum-mismatch = enchi: Prüfsummenabweichung für `{ $url }` ({ $algorithm }): erwartet { $expected }, erhalten { $actual }
error-missing-checksum = enchi: Herausgeber-Prüfsumme für `{ $source }` konnte nicht ermittelt werden
error-io = { $message }
error-self-update-portable = enchi: Selbstupdate unterstützt nur portable Installationen, die von `{ $expected }` gestartet wurden; aktuelle Datei ist `{ $current }`
error-self-update-managed = enchi: Diese Installation wird von einem Paketmanager ({ $source }) verwaltet; aktualisiere sie mit: { $command }
