# ./locales/pt-BR/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Gerenciador de runtimes e ambientes Python
cli-global-about = Definir ou mostrar a versão global do Python
cli-local-about = Definir ou mostrar a versão do Python do diretório local
cli-shell-about = Definir ou mostrar a versão do Python específica do shell
cli-latest-about = Mostrar a versão mais recente instalada ou conhecida que corresponda ao prefixo
cli-version-about = Mostrar a versão atual do Python e sua origem
cli-version-name-about = Mostrar a versão atual do Python
cli-version-origin-about = Explicar como a versão atual do Python é definida
cli-prefix-about = Exibir os caminhos onde as versões indicadas do Python estão instaladas
cli-install-about = Instalar versões do Python a partir de provedores nativos
cli-available-about = Listar versões instaláveis do Python a partir de provedores nativos
cli-versions-about = Listar todas as versões do Python disponíveis para o enchi
cli-upgrade-about = Mover os runtimes instalados para o patch mais recente (e seus venvs)
cli-uninstall-about = Desinstalar uma versão específica do Python
cli-venv-about = Criar, inspecionar e atribuir ambientes virtuais gerenciados
cli-pip-about = Listar, verificar, instalar e atualizar pacotes de um runtime ou venv
cli-init-about = Configurar o ambiente do shell para o enchi
cli-gui-about = Abrir o painel gráfico do Enchi
cli-rehash-about = Regenerar os shims do enchi (instala executáveis em todas as versões)
cli-shims-about = Listar os shims existentes do enchi
cli-prompt-about = Mostrar uma string sucinta de prompt para o ambiente atual
cli-exec-about = Executar um programa com a versão selecionada do Python
cli-completions-about = Mostrar o script de completar comandos
cli-doctor-about = Diagnosticar PATH, shims e pré-requisitos de instalação
cli-config-about = Obter, definir ou mostrar a configuração do Enchi
cli-self-update-about = Atualizar o Enchi a partir do GitHub Releases
cli-preflight-about = Inteligência de plataforma e preflight de prontidão para instalação
cli-environment-about = Alias de preflight (fatos de SO/toolchain para agentes e usuários)
cli-status-about = Mostrar o status abrangente do ambiente (versões, origens, venvs)
cli-root-about = Exibir o diretório raiz onde versões e shims são mantidos
cli-which-about = Exibir o caminho completo de um executável
cli-whence-about = Listar todas as versões do Python que contêm o executável indicado
cli-version-file-about = Detectar o arquivo que define a versão atual do enchi
cli-version-file-read-about = Ler o conteúdo de um arquivo .python-version
cli-self-install-about = Instalar ou reparar o Enchi a partir de um pacote de release
cli-self-uninstall-about = Desinstalar o Enchi do sistema
cli-agent-skills-about = Instalar a skill do Enchi para assistentes de programação
cli-help-about = Exibir a ajuda de um comando
cli-commands-about = Listar todos os comandos do enchi disponíveis
cli-hooks-about = Listar os ganchos executáveis de um comando dado
cli-venv-list-about = Listar ambientes virtuais gerenciados
cli-venv-info-about = Mostrar detalhes de um ambiente virtual gerenciado
cli-venv-create-about = Criar um ambiente virtual gerenciado sob um runtime específico
cli-venv-delete-about = Remover um ambiente virtual gerenciado
cli-venv-rename-about = Renomear um ambiente virtual gerenciado
cli-venv-use-about = Atribuir um ambiente virtual gerenciado ao diretório atual ou globalmente
cli-venv-upgrade-about = Atualizar um ambiente virtual gerenciado para um novo runtime base
cli-pip-list-about = Listar os pacotes pip instalados em um ambiente de destino
cli-pip-outdated-about = Listar os pacotes pip desatualizados em um ambiente de destino
cli-pip-check-about = Verificar requisitos de pacotes quebrados em um ambiente de destino
cli-pip-precheck-about = Pré-verificar estaticamente um arquivo de requisitos ou URL HTTPS antes de instalar
cli-pip-analyze-about = Analisar fontes Python em busca de importações de terceiros ausentes no destino
cli-pip-install-about = Instalar pacotes a partir de um arquivo requirements.txt ou URL HTTPS
cli-pip-update-about = Atualizar pacotes pip em um ambiente de destino
cli-config-path-about = Mostrar o caminho do arquivo de configuração
cli-config-show-about = Mostrar toda a configuração atual
cli-config-get-about = Mostrar o valor de uma chave de configuração específica
cli-config-set-about = Atualizar uma chave de configuração
cli-help-selection = SELEÇÃO
cli-help-provisioning = INSTALAÇÃO
cli-help-environment = AMBIENTE
cli-help-interface = INTERFACE
cli-help-diagnostics = DIAGNÓSTICO E CONFIGURAÇÃO
cli-help-maintenance = MANUTENÇÃO
cli-help-support = SUPORTE
cli-help-usage = Uso: enchi <command> [<args>]
cli-help-useful = Alguns comandos úteis do enchi:
cli-help-concepts =
    CONCEITOS PRINCIPAIS:
      Shims: executáveis leves (como `python` ou `pip`) que interceptam seus comandos e os encaminham à versão correta do Python. Execute `enchi rehash` após instalar novos pacotes pip.
      Versões: ambientes Python instalados com `enchi install`, em `$(enchi root)/versions`.
      Ambientes gerenciados: ambientes virtuais nomeados em `$(enchi root)/venvs/<runtime>/<name>`. Prefira `enchi venv create` e `enchi venv use`.
      Descoberta: `enchi install --list 3.13` ou `enchi available 3.13`.
      Seleção: PYENV_VERSION, depois `.python-version`, depois o arquivo de versão global.
    
    Execute `enchi help <command>` para ajuda detalhada. Docs: https://github.com/imyourboyroy/pyenv-native
