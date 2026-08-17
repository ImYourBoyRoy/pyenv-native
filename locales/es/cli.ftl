# ./locales/es/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Gestor nativo y multiplataforma de versiones de Python
cli-global-about = Establecer o mostrar la versión global de Python
cli-local-about = Establecer o mostrar la versión de Python del directorio local
cli-shell-about = Establecer o mostrar la versión de Python específica del shell
cli-latest-about = Mostrar la última versión instalada o conocida que coincida con el prefijo
cli-version-about = Mostrar la versión actual de Python y su origen
cli-version-name-about = Mostrar la versión actual de Python
cli-version-origin-about = Explicar cómo se establece la versión actual de Python
cli-prefix-about = Mostrar las rutas donde están instaladas las versiones de Python indicadas
cli-install-about = Instalar versiones de Python desde proveedores nativos
cli-available-about = Listar las versiones de Python instalables desde proveedores nativos
cli-versions-about = Listar todas las versiones de Python disponibles para pyenv
cli-uninstall-about = Desinstalar una versión concreta de Python
cli-venv-about = Crear, inspeccionar y asignar entornos virtuales gestionados
cli-pip-about = Listar, comprobar, instalar y actualizar paquetes de un runtime o venv
cli-init-about = Configurar el entorno del shell para pyenv
cli-gui-about = Abrir el panel gráfico de Pyenv Native
cli-rehash-about = Regenerar los shims de pyenv (instala ejecutables en todas las versiones)
cli-shims-about = Listar los shims de pyenv existentes
cli-prompt-about = Mostrar una cadena breve de prompt para el entorno actual
cli-exec-about = Ejecutar un programa con la versión de Python seleccionada
cli-completions-about = Mostrar el script de completar comandos
cli-doctor-about = Diagnosticar PATH, shims y requisitos de instalación
cli-config-about = Obtener, establecer o mostrar la configuración de pyenv-native
cli-self-update-about = Actualizar pyenv-native desde GitHub Releases
cli-preflight-about = Inteligencia de plataforma y preflight de preparación para instalar
cli-environment-about = Alias de preflight (datos de SO/cadena de herramientas para agentes y usuarios)
cli-status-about = Mostrar el estado completo del entorno (versiones, orígenes, venvs)
cli-root-about = Mostrar el directorio raíz donde se guardan las versiones y los shims
cli-which-about = Mostrar la ruta completa de un ejecutable
cli-whence-about = Listar todas las versiones de Python que contienen el ejecutable indicado
cli-version-file-about = Detectar el archivo que establece la versión actual de pyenv
cli-version-file-read-about = Leer el contenido de un archivo .python-version
cli-self-uninstall-about = Desinstalar pyenv-native del sistema
cli-help-about = Mostrar la ayuda de un comando
cli-commands-about = Listar todos los comandos pyenv disponibles
cli-hooks-about = Listar los ganchos ejecutables de un comando dado
cli-venv-list-about = Listar los entornos virtuales gestionados
cli-venv-info-about = Mostrar detalles de un entorno virtual gestionado
cli-venv-create-about = Crear un entorno virtual gestionado bajo un runtime concreto
cli-venv-delete-about = Eliminar un entorno virtual gestionado
cli-venv-rename-about = Renombrar un entorno virtual gestionado
cli-venv-use-about = Asignar un entorno virtual gestionado al directorio actual o de forma global
cli-venv-upgrade-about = Actualizar un entorno virtual gestionado a un nuevo runtime base
cli-pip-list-about = Listar los paquetes pip instalados en un entorno de destino
cli-pip-outdated-about = Listar los paquetes pip desactualizados en un entorno de destino
cli-pip-check-about = Comprobar requisitos de paquetes rotos en un entorno de destino
cli-pip-precheck-about = Comprobar estáticamente un archivo de requisitos o una URL HTTPS antes de instalar
cli-pip-analyze-about = Analizar fuentes Python en busca de importaciones de terceros que falten en el destino
cli-pip-install-about = Instalar paquetes desde un archivo requirements.txt o una URL HTTPS
cli-pip-update-about = Actualizar paquetes pip dentro de un entorno de destino
cli-config-path-about = Mostrar la ruta del archivo de configuración
cli-config-show-about = Mostrar toda la configuración actual
cli-config-get-about = Mostrar el valor de una clave de configuración concreta
cli-config-set-about = Actualizar una clave de configuración
cli-help-selection = SELECCIÓN
cli-help-provisioning = INSTALACIÓN
cli-help-environment = ENTORNO
cli-help-interface = INTERFAZ
cli-help-diagnostics = DIAGNÓSTICO Y CONFIGURACIÓN
cli-help-maintenance = MANTENIMIENTO
cli-help-support = SOPORTE
cli-help-usage = Uso: pyenv <command> [<args>]
cli-help-useful = Algunos comandos útiles de pyenv:
cli-help-concepts =
    CONCEPTOS:
      Shims: ejecutables ligeros (`python` o `pip`) que interceptan comandos y los enrutan a la versión actual. Ejecute `pyenv rehash` tras instalar paquetes pip.
      Versions: entornos instalados con `pyenv install`, en `~/.pyenv/versions`.
      Managed envs: `~/.pyenv/venvs/<runtime>/<name>`. Prefiera `pyenv venv create` y `pyenv venv use`.
      Discovery: `pyenv install --list 3.13` o `pyenv available 3.13`.
      Selection: PYENV_VERSION, luego `.python-version`, luego el archivo global.
    
    Ejecute `pyenv help <command>`. Docs: https://github.com/imyourboyroy/pyenv-native
