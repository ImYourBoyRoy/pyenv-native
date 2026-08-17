# ./locales/it/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Gestore nativo e multipiattaforma delle versioni di Python
cli-global-about = Impostare o mostrare la versione globale di Python
cli-local-about = Impostare o mostrare la versione di Python della directory locale
cli-shell-about = Impostare o mostrare la versione di Python specifica della shell
cli-latest-about = Mostrare l’ultima versione installata o nota che corrisponde al prefisso
cli-version-about = Mostrare la versione corrente di Python e la relativa origine
cli-version-name-about = Mostrare la versione corrente di Python
cli-version-origin-about = Spiegare come è impostata la versione corrente di Python
cli-prefix-about = Mostrare i percorsi in cui sono installate le versioni di Python indicate
cli-install-about = Installare versioni di Python da provider nativi
cli-available-about = Elencare le versioni di Python installabili da provider nativi
cli-versions-about = Elencare tutte le versioni di Python disponibili per pyenv
cli-uninstall-about = Disinstallare una versione specifica di Python
cli-venv-about = Creare, ispezionare e assegnare ambienti virtuali gestiti
cli-pip-about = Elencare, verificare, installare e aggiornare i pacchetti di un runtime o venv
cli-init-about = Configurare l’ambiente della shell per pyenv
cli-gui-about = Aprire il pannello grafico di Pyenv Native
cli-rehash-about = Rigenerare gli shim di pyenv (installa gli eseguibili in tutte le versioni)
cli-shims-about = Elencare gli shim di pyenv esistenti
cli-prompt-about = Mostrare una stringa di prompt concisa per l’ambiente corrente
cli-exec-about = Eseguire un programma con la versione di Python selezionata
cli-completions-about = Mostrare lo script di completamento dei comandi
cli-doctor-about = Diagnosticare PATH, shim e prerequisiti di installazione
cli-config-about = Leggere, impostare o mostrare la configurazione di pyenv-native
cli-self-update-about = Aggiornare pyenv-native da GitHub Releases
cli-preflight-about = Informazioni sulla piattaforma e preflight di prontezza all’installazione
cli-environment-about = Alias di preflight (fatti su SO/toolchain per agenti e utenti)
cli-status-about = Mostrare lo stato completo dell’ambiente (versioni, origini, venv)
cli-root-about = Mostrare la directory radice in cui sono conservate versioni e shim
cli-which-about = Mostrare il percorso completo di un eseguibile
cli-whence-about = Elencare tutte le versioni di Python che contengono l’eseguibile indicato
cli-version-file-about = Rilevare il file che imposta la versione pyenv corrente
cli-version-file-read-about = Leggere il contenuto di un file .python-version
cli-self-uninstall-about = Disinstallare pyenv-native dal sistema
cli-help-about = Mostrare la guida di un comando
cli-commands-about = Elencare tutti i comandi pyenv disponibili
cli-hooks-about = Elencare gli hook eseguibili di un comando dato
cli-venv-list-about = Elencare gli ambienti virtuali gestiti
cli-venv-info-about = Mostrare i dettagli di un ambiente virtuale gestito
cli-venv-create-about = Creare un ambiente virtuale gestito sotto un runtime specifico
cli-venv-delete-about = Rimuovere un ambiente virtuale gestito
cli-venv-rename-about = Rinominare un ambiente virtuale gestito
cli-venv-use-about = Assegnare un ambiente virtuale gestito alla directory corrente o globalmente
cli-venv-upgrade-about = Aggiornare un ambiente virtuale gestito a un nuovo runtime di base
cli-pip-list-about = Elencare i pacchetti pip installati in un ambiente di destinazione
cli-pip-outdated-about = Elencare i pacchetti pip obsoleti in un ambiente di destinazione
cli-pip-check-about = Controllare requisiti di pacchetti non validi in un ambiente di destinazione
cli-pip-precheck-about = Precontrollare in modo statico un file di requisiti o un URL HTTPS prima dell’installazione
cli-pip-analyze-about = Analizzare i sorgenti Python per import di terze parti mancanti nella destinazione
cli-pip-install-about = Installare pacchetti da un file requirements.txt o da un URL HTTPS
cli-pip-update-about = Aggiornare i pacchetti pip in un ambiente di destinazione
cli-config-path-about = Mostrare il percorso del file di configurazione
cli-config-show-about = Mostrare tutta la configurazione corrente
cli-config-get-about = Mostrare il valore di una chiave di configurazione specifica
cli-config-set-about = Aggiornare una chiave di configurazione
cli-help-selection = SELEZIONE
cli-help-provisioning = INSTALLAZIONE
cli-help-environment = AMBIENTE
cli-help-interface = INTERFACCIA
cli-help-diagnostics = DIAGNOSTICA E CONFIGURAZIONE
cli-help-maintenance = MANUTENZIONE
cli-help-support = SUPPORTO
cli-help-usage = Uso: pyenv <command> [<args>]
cli-help-useful = Alcuni comandi pyenv utili:
cli-help-concepts =
    CONCETTI:
      Shims: eseguibili leggeri (`python` o `pip`) che intercettano i comandi e li instradano alla versione corrente. Esegui `pyenv rehash` dopo aver installato pacchetti pip.
      Versions: ambienti installati con `pyenv install`, in `~/.pyenv/versions`.
      Managed envs: `~/.pyenv/venvs/<runtime>/<name>`. Preferisci `pyenv venv create` e `pyenv venv use`.
      Discovery: `pyenv install --list 3.13` o `pyenv available 3.13`.
      Selection: PYENV_VERSION, poi `.python-version`, poi il file globale.
    
    Esegui `pyenv help <command>`. Docs: https://github.com/imyourboyroy/pyenv-native
