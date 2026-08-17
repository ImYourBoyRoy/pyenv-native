# ./locales/it/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = Download di pyenv-native { $version }
install-extracting = Estrazione del pacchetto di versione
install-installing = Installazione in { $path }
install-done = pyenv-native installato. Aprire un nuovo terminale, poi eseguire `pyenv doctor`.
install-failed = Installazione non riuscita
install-lang-help = Lingua dell’interfaccia per i messaggi del programma di installazione
install-summary-title = Riepilogo installazione pyenv-native
install-network-summary-title = Riepilogo installazione di rete pyenv-native
install-summary-blurb = Questo creerà o aggiornerà un’installazione portatile di pyenv-native sotto la radice selezionata.
install-summary-blurb-detail = Installa pyenv, il server pyenv-mcp adatto agli agenti e il companion GUI quando disponibile, scrive un registro di installazione ed esegue controlli di base.
install-network-blurb = Questo scaricherà un pacchetto pyenv-native pubblicato, ne verificherà il checksum SHA-256 e lo installerà nella radice portatile selezionata.
install-profile-yes = Il profilo della shell verrà aggiornato così le sessioni future potranno trovare pyenv-native automaticamente.
install-profile-yes-pwsh = Il profilo PowerShell verrà aggiornato così le sessioni future potranno trovare pyenv-native automaticamente.
install-profile-no = Non verranno apportate modifiche al profilo della shell.
install-profile-no-pwsh = Non verranno apportate modifiche al profilo PowerShell.
install-continue = Continuare l’installazione? [y/N]:
install-need-yes = Le installazioni interattive richiedono conferma. Rieseguire con --yes per l’uso non interattivo.
install-need-yes-pwsh = Le installazioni interattive richiedono conferma. Rieseguire con -Yes per l’uso non interattivo.
install-cancelled = Installazione annullata.
install-installed-to = pyenv-native installato in { $path }
install-installed-command = Comando installato: { $path }
install-installed-mcp = Server MCP installato: { $path }
install-mcp-helper = Assistente di configurazione MCP: { $command }
install-installed-gui = GUI installata: { $path }
install-log-file = File di registro: { $path }
