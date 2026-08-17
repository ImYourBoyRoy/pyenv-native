# ./locales/fr/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv : impossible de déterminer le répertoire personnel pour PYENV_ROOT
error-invalid-directory = pyenv : impossible de changer le répertoire de travail vers `{ $path }`
error-invalid-version = pyenv : version non valide `{ $version }` ignorée dans `{ $path }`
error-no-local-version = pyenv : aucune version locale configurée pour ce répertoire
error-version-not-installed =
    pyenv : la version `{ $version }` n’est pas installée (définie par { $origin })
    astuce : exécutez `pyenv install { $version }` pour l’installer, ou `pyenv versions` pour voir les versions installées
error-unknown-config-key = pyenv : clé de configuration inconnue `{ $key }`
error-invalid-config-value = pyenv : valeur non valide `{ $value }` pour la clé de configuration `{ $key }`
error-version-already-installed = pyenv : la version `{ $version }` est déjà installée
error-unknown-version = pyenv : aucune version connue ne correspond à `{ $version }`
error-unsupported-install-target = pyenv : le backend d’installation ne prend pas en charge `{ $version }` sur cette plateforme
error-missing-install-version = pyenv : l’installation exige au moins un argument de version
error-missing-python-build = pyenv : impossible de localiser le backend python-build ; définissez install.python_build_path ou ajoutez python-build à PATH
error-checksum-mismatch = pyenv : somme de contrôle incorrecte pour `{ $url }` ({ $algorithm }) : attendu { $expected }, obtenu { $actual }
error-missing-checksum = pyenv : impossible d’obtenir une somme de contrôle de l’éditeur pour `{ $source }`
error-io = { $message }
error-self-update-portable = pyenv : la mise à jour automatique ne prend en charge que les installations portables lancées depuis `{ $expected }` ; l’exécutable actuel est `{ $current }`
