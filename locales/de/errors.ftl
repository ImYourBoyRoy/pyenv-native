# ./locales/de/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: Home-Verzeichnis für PYENV_ROOT kann nicht ermittelt werden
error-invalid-directory = pyenv: Arbeitsverzeichnis kann nicht zu `{ $path }` gewechselt werden
error-invalid-version = pyenv: ungültige Version `{ $version }` in `{ $path }` ignoriert
error-no-local-version = pyenv: keine lokale Version für dieses Verzeichnis konfiguriert
error-version-not-installed =
    pyenv: Version `{ $version }` ist nicht installiert (gesetzt von { $origin })
    Hinweis: mit `pyenv install { $version }` installieren oder mit `pyenv versions` installierte Versionen anzeigen
error-unknown-config-key = pyenv: unbekannter Konfigurationsschlüssel `{ $key }`
error-invalid-config-value = pyenv: ungültiger Wert `{ $value }` für Konfigurationsschlüssel `{ $key }`
error-version-already-installed = pyenv: Version `{ $version }` ist bereits installiert
error-unknown-version = pyenv: keine bekannten Versionen entsprechen `{ $version }`
error-unsupported-install-target = pyenv: die Installationskomponente unterstützt `{ $version }` auf dieser Plattform nicht
error-missing-install-version = pyenv: Installationsvorgang erfordert mindestens ein Versionsargument
error-missing-python-build = pyenv: python-build-Komponente nicht gefunden; install.python_build_path setzen oder python-build zum PATH hinzufügen
error-checksum-mismatch = pyenv: Prüfsummenabweichung für `{ $url }` ({ $algorithm }): erwartet { $expected }, erhalten { $actual }
error-missing-checksum = pyenv: Herausgeber-Prüfsumme für `{ $source }` konnte nicht ermittelt werden
error-io = { $message }
error-self-update-portable = pyenv: Selbstupdate unterstützt nur portable Installationen, die von `{ $expected }` gestartet wurden; aktuelle Datei ist `{ $current }`
