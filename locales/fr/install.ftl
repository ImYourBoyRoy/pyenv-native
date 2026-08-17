# ./locales/fr/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = Téléchargement de pyenv-native { $version }
install-extracting = Extraction du paquet de version
install-installing = Installation dans { $path }
install-done = pyenv-native installé. Ouvrez un nouveau terminal, puis exécutez `pyenv doctor`.
install-failed = Échec de l’installation
install-lang-help = Langue de l’interface pour les messages de l’installateur
install-summary-title = Résumé d’installation pyenv-native
install-network-summary-title = Résumé d’installation réseau pyenv-native
install-summary-blurb = Ceci créera ou mettra à jour une installation portable de pyenv-native sous la racine sélectionnée.
install-summary-blurb-detail = Installe pyenv, le serveur pyenv-mcp adapté aux agents et le compagnon GUI lorsqu’il est disponible, écrit un journal d’installation et exécute des contrôles de base.
install-network-blurb = Ceci téléchargera un paquet pyenv-native publié, vérifiera sa somme SHA-256 et l’installera dans la racine portable sélectionnée.
install-profile-yes = Votre profil de shell sera mis à jour pour que les sessions futures trouvent pyenv-native automatiquement.
install-profile-yes-pwsh = Votre profil PowerShell sera mis à jour pour que les sessions futures trouvent pyenv-native automatiquement.
install-profile-no = Aucun changement de profil de shell ne sera effectué.
install-profile-no-pwsh = Aucun changement de profil PowerShell ne sera effectué.
install-continue = Continuer l’installation ? [y/N]:
install-need-yes = Une confirmation est requise pour les installations interactives. Relancez avec --yes pour un usage non interactif.
install-need-yes-pwsh = Une confirmation est requise pour les installations interactives. Relancez avec -Yes pour un usage non interactif.
install-cancelled = Installation annulée.
install-installed-to = pyenv-native installé dans { $path }
install-installed-command = Commande installée : { $path }
install-installed-mcp = Serveur MCP installé : { $path }
install-mcp-helper = Assistant de configuration MCP : { $command }
install-installed-gui = GUI installée : { $path }
install-log-file = Fichier journal : { $path }
