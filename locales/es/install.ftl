# ./locales/es/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = Descargando { $version }
install-extracting = Extrayendo el paquete de la versión
install-installing = Instalando en { $path }
install-done = Enchi instalado. Abre un terminal nuevo y ejecuta `enchi doctor`.
install-failed = Falló la instalación
install-lang-help = Idioma de la interfaz para los mensajes del instalador
install-summary-title = Resumen de instalación de Enchi
install-network-summary-title = Resumen de instalación en red de Enchi
install-summary-blurb = Esto creará o actualizará una instalación portable de Enchi en la raíz seleccionada.
install-summary-blurb-detail = Instala enchi, el servidor enchi-mcp orientado a agentes y la aplicación gráfica complementaria cuando esté disponible, escribe un registro de instalación y ejecuta comprobaciones básicas.
install-network-blurb = Esto descargará un paquete publicado de Enchi, verificará su suma SHA-256 y lo instalará en la raíz portable seleccionada.
install-profile-yes = Se actualizará el perfil del shell para que las sesiones futuras encuentren Enchi automáticamente.
install-profile-yes-pwsh = Se actualizará el perfil de PowerShell para que las sesiones futuras encuentren Enchi automáticamente.
install-profile-no = No se harán cambios en el perfil del shell.
install-profile-no-pwsh = No se harán cambios en el perfil de PowerShell.
install-continue = ¿Continuar con la instalación? [y/N]:
install-need-yes = Las instalaciones interactivas requieren confirmación. Vuelva a ejecutar con --yes para uso no interactivo.
install-need-yes-pwsh = Las instalaciones interactivas requieren confirmación. Vuelva a ejecutar con -Yes para uso no interactivo.
install-cancelled = Instalación cancelada.
install-installed-to = Enchi instalado en { $path }
install-installed-command = Comando instalado: { $path }
install-installed-mcp = Servidor MCP instalado: { $path }
install-mcp-helper = Asistente de configuración MCP: { $command }
install-installed-gui = GUI instalada: { $path }
install-log-file = Archivo de registro: { $path }
