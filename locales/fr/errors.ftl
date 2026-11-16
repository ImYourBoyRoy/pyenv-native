# ./locales/fr/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv : impossible de déterminer le répertoire personnel pour ENCHI_ROOT
error-invalid-directory = enchi : impossible de changer le répertoire de travail vers `{ $path }`
error-invalid-version = enchi : version non valide `{ $version }` ignorée dans `{ $path }`
error-no-local-version = enchi : aucune version locale configurée pour ce répertoire
error-version-not-installed =
    enchi : la version `{ $version }` n’est pas installée (définie par { $origin })
    astuce : exécutez `enchi install { $version }` pour l’installer, ou `enchi versions` pour voir les versions installées
error-unknown-config-key = enchi : clé de configuration inconnue `{ $key }`
error-invalid-config-value = enchi : valeur non valide `{ $value }` pour la clé de configuration `{ $key }`
error-version-already-installed = enchi : la version `{ $version }` est déjà installée
error-unknown-version = enchi : aucune version connue ne correspond à `{ $version }`
error-unsupported-install-target = enchi : le backend d’installation ne prend pas en charge `{ $version }` sur cette plateforme
error-missing-install-version = enchi : l’installation exige au moins un argument de version
error-missing-python-build = enchi : impossible de localiser le backend python-build ; définissez install.python_build_path ou ajoutez python-build à PATH
error-checksum-mismatch = enchi : somme de contrôle incorrecte pour `{ $url }` ({ $algorithm }) : attendu { $expected }, obtenu { $actual }
error-missing-checksum = enchi : impossible d’obtenir une somme de contrôle de l’éditeur pour `{ $source }`
error-io = { $message }
error-self-update-portable = enchi : la mise à jour automatique ne prend en charge que les installations portables lancées depuis `{ $expected }` ; l’exécutable actuel est `{ $current }`
error-self-update-managed = enchi : cette installation est gérée par un gestionnaire de paquets ({ $source }) ; mettez-la à jour avec : { $command }
