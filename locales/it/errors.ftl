# ./locales/it/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: impossibile determinare la directory home per PYENV_ROOT
error-invalid-directory = enchi: impossibile cambiare la directory di lavoro in `{ $path }`
error-invalid-version = enchi: versione non valida `{ $version }` ignorata in `{ $path }`
error-no-local-version = enchi: nessuna versione locale configurata per questa directory
error-version-not-installed =
    enchi: la versione `{ $version }` non è installata (impostata da { $origin })
    suggerimento: eseguire `enchi install { $version }` per installarla, oppure `enchi versions` per vedere le versioni installate
error-unknown-config-key = enchi: chiave di configurazione sconosciuta `{ $key }`
error-invalid-config-value = enchi: valore non valido `{ $value }` per la chiave di configurazione `{ $key }`
error-version-already-installed = enchi: la versione `{ $version }` è già installata
error-unknown-version = enchi: nessuna versione nota corrisponde a `{ $version }`
error-unsupported-install-target = enchi: il backend di installazione non supporta `{ $version }` su questa piattaforma
error-missing-install-version = enchi: l’operazione di installazione richiede almeno un argomento di versione
error-missing-python-build = enchi: impossibile trovare il backend python-build; impostare install.python_build_path o aggiungere python-build a PATH
error-checksum-mismatch = enchi: checksum non corrispondente per `{ $url }` ({ $algorithm }): previsto { $expected }, ottenuto { $actual }
error-missing-checksum = enchi: impossibile ottenere un checksum dell’editore per `{ $source }`
error-io = { $message }
error-self-update-portable = enchi: l’auto-aggiornamento supporta solo installazioni portatili avviate da `{ $expected }`; l’eseguibile attuale è `{ $current }`
error-self-update-managed = enchi: questa installazione è gestita da un gestore di pacchetti ({ $source }); aggiornala con: { $command }
