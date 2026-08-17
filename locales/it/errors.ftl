# ./locales/it/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: impossibile determinare la directory home per PYENV_ROOT
error-invalid-directory = pyenv: impossibile cambiare la directory di lavoro in `{ $path }`
error-invalid-version = pyenv: versione non valida `{ $version }` ignorata in `{ $path }`
error-no-local-version = pyenv: nessuna versione locale configurata per questa directory
error-version-not-installed =
    pyenv: la versione `{ $version }` non è installata (impostata da { $origin })
    suggerimento: eseguire `pyenv install { $version }` per installarla, oppure `pyenv versions` per vedere le versioni installate
error-unknown-config-key = pyenv: chiave di configurazione sconosciuta `{ $key }`
error-invalid-config-value = pyenv: valore non valido `{ $value }` per la chiave di configurazione `{ $key }`
error-version-already-installed = pyenv: la versione `{ $version }` è già installata
error-unknown-version = pyenv: nessuna versione nota corrisponde a `{ $version }`
error-unsupported-install-target = pyenv: il backend di installazione non supporta `{ $version }` su questa piattaforma
error-missing-install-version = pyenv: l’operazione di installazione richiede almeno un argomento di versione
error-missing-python-build = pyenv: impossibile trovare il backend python-build; impostare install.python_build_path o aggiungere python-build a PATH
error-checksum-mismatch = pyenv: checksum non corrispondente per `{ $url }` ({ $algorithm }): previsto { $expected }, ottenuto { $actual }
error-missing-checksum = pyenv: impossibile ottenere un checksum dell’editore per `{ $source }`
error-io = { $message }
error-self-update-portable = pyenv: l’auto-aggiornamento supporta solo installazioni portatili avviate da `{ $expected }`; l’eseguibile attuale è `{ $current }`
