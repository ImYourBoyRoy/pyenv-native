# ./locales/es/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = Descargando pyenv-native { $version }
install-extracting = Extrayendo el paquete de la versión
install-installing = Instalando en { $path }
install-done = pyenv-native instalado. Abra un terminal nuevo y ejecute `pyenv doctor`.
install-failed = Falló la instalación
install-lang-help = Idioma de la interfaz para los mensajes del instalador
install-summary-title = Resumen de instalación de pyenv-native
install-network-summary-title = Resumen de instalación en red de pyenv-native
install-summary-blurb = Esto creará o actualizará una instalación portable de pyenv-native en la raíz seleccionada.
install-summary-blurb-detail = Instala pyenv, el servidor pyenv-mcp orientado a agentes y el compañero GUI cuando esté disponible, escribe un registro de instalación y ejecuta comprobaciones básicas.
install-network-blurb = Esto descargará un paquete publicado de pyenv-native, verificará su suma SHA-256 y lo instalará en la raíz portable seleccionada.
install-profile-yes = Se actualizará el perfil del shell para que las sesiones futuras encuentren pyenv-native automáticamente.
install-profile-yes-pwsh = Se actualizará el perfil de PowerShell para que las sesiones futuras encuentren pyenv-native automáticamente.
install-profile-no = No se harán cambios en el perfil del shell.
install-profile-no-pwsh = No se harán cambios en el perfil de PowerShell.
install-continue = ¿Continuar con la instalación? [y/N]:
install-need-yes = Las instalaciones interactivas requieren confirmación. Vuelva a ejecutar con --yes para uso no interactivo.
install-need-yes-pwsh = Las instalaciones interactivas requieren confirmación. Vuelva a ejecutar con -Yes para uso no interactivo.
install-cancelled = Instalación cancelada.
install-installed-to = pyenv-native instalado en { $path }
install-installed-command = Comando instalado: { $path }
install-installed-mcp = Servidor MCP instalado: { $path }
install-mcp-helper = Asistente de configuración MCP: { $command }
install-installed-gui = GUI instalada: { $path }
install-log-file = Archivo de registro: { $path }
