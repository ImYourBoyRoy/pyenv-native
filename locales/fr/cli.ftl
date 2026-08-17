# ./locales/fr/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Gestionnaire natif et multiplateforme de versions Python
cli-global-about = Définir ou afficher la version Python globale
cli-local-about = Définir ou afficher la version Python du répertoire local
cli-shell-about = Définir ou afficher la version Python propre au shell
cli-latest-about = Afficher la dernière version installée ou connue correspondant au préfixe
cli-version-about = Afficher la version Python actuelle et son origine
cli-version-name-about = Afficher la version Python actuelle
cli-version-origin-about = Expliquer comment la version Python actuelle est définie
cli-prefix-about = Afficher les chemins où les versions Python indiquées sont installées
cli-install-about = Installer des versions Python depuis des fournisseurs natifs
cli-available-about = Lister les versions Python installables depuis des fournisseurs natifs
cli-versions-about = Lister toutes les versions Python disponibles pour pyenv
cli-uninstall-about = Désinstaller une version Python précise
cli-venv-about = Créer, inspecter et attribuer des environnements virtuels gérés
cli-pip-about = Lister, vérifier, installer et mettre à jour les paquets d’un runtime ou d’un venv
cli-init-about = Configurer l’environnement du shell pour pyenv
cli-gui-about = Ouvrir le tableau de bord graphique de Pyenv Native
cli-rehash-about = Régénérer les shims pyenv (installe les exécutables pour toutes les versions)
cli-shims-about = Lister les shims pyenv existants
cli-prompt-about = Afficher une chaîne d’invite concise pour l’environnement actuel
cli-exec-about = Exécuter un programme avec la version Python sélectionnée
cli-completions-about = Afficher le script de complétion des commandes
cli-doctor-about = Diagnostiquer PATH, les shims et les prérequis d’installation
cli-config-about = Lire, définir ou afficher la configuration de pyenv-native
cli-self-update-about = Mettre à jour pyenv-native depuis GitHub Releases
cli-preflight-about = Intelligence de plateforme et preflight de préparation à l’installation
cli-environment-about = Alias de preflight (faits OS/chaîne d’outils pour agents et utilisateurs)
cli-status-about = Afficher l’état complet de l’environnement (versions, origines, venvs)
cli-root-about = Afficher le répertoire racine où sont conservés versions et shims
cli-which-about = Afficher le chemin complet d’un exécutable
cli-whence-about = Lister toutes les versions Python qui contiennent l’exécutable indiqué
cli-version-file-about = Détecter le fichier qui définit la version pyenv actuelle
cli-version-file-read-about = Lire le contenu d’un fichier .python-version
cli-self-uninstall-about = Désinstaller pyenv-native du système
cli-help-about = Afficher l’aide d’une commande
cli-commands-about = Lister toutes les commandes pyenv disponibles
cli-hooks-about = Lister les crochets exécutables d’une commande donnée
cli-venv-list-about = Lister les environnements virtuels gérés
cli-venv-info-about = Afficher les détails d’un environnement virtuel géré
cli-venv-create-about = Créer un environnement virtuel géré sous un runtime précis
cli-venv-delete-about = Supprimer un environnement virtuel géré
cli-venv-rename-about = Renommer un environnement virtuel géré
cli-venv-use-about = Attribuer un environnement virtuel géré au répertoire actuel ou globalement
cli-venv-upgrade-about = Mettre à niveau un environnement virtuel géré vers un nouveau runtime de base
cli-pip-list-about = Lister les paquets pip installés dans un environnement cible
cli-pip-outdated-about = Lister les paquets pip obsolètes dans un environnement cible
cli-pip-check-about = Vérifier les exigences de paquets cassées dans un environnement cible
cli-pip-precheck-about = Précontrôler statiquement un fichier d’exigences ou une URL HTTPS avant l’installation
cli-pip-analyze-about = Analyser les sources Python pour les imports tiers manquants dans la cible
cli-pip-install-about = Installer des paquets depuis un fichier requirements.txt ou une URL HTTPS
cli-pip-update-about = Mettre à jour les paquets pip dans un environnement cible
cli-config-path-about = Afficher le chemin du fichier de configuration
cli-config-show-about = Afficher toute la configuration actuelle
cli-config-get-about = Afficher la valeur d’une clé de configuration précise
cli-config-set-about = Mettre à jour une clé de configuration
cli-help-selection = SÉLECTION
cli-help-provisioning = INSTALLATION
cli-help-environment = ENVIRONNEMENT
cli-help-interface = INTERFACE
cli-help-diagnostics = DIAGNOSTICS ET CONFIGURATION
cli-help-maintenance = MAINTENANCE
cli-help-support = ASSISTANCE
cli-help-usage = Utilisation : pyenv <command> [<args>]
cli-help-useful = Commandes pyenv utiles :
cli-help-concepts =
    CONCEPTS:
      Shims : exécutables légers (`python` ou `pip`) qui interceptent les commandes et les acheminent vers la version actuelle. Exécutez `pyenv rehash` après l’installation de paquets pip.
      Versions : environnements installés via `pyenv install`, sous `~/.pyenv/versions`.
      Managed envs : `~/.pyenv/venvs/<runtime>/<name>`. Préférez `pyenv venv create` et `pyenv venv use`.
      Discovery : `pyenv install --list 3.13` ou `pyenv available 3.13`.
      Selection : PYENV_VERSION, puis `.python-version`, puis le fichier global.
    
    Exécutez `pyenv help <command>`. Docs : https://github.com/imyourboyroy/pyenv-native
