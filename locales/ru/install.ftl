# ./locales/ru/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = Загрузка pyenv-native { $version }
install-extracting = Распаковка пакета выпуска
install-installing = Установка в { $path }
install-done = pyenv-native установлен. Откройте новый терминал и выполните `pyenv doctor`.
install-failed = Установка не удалась
install-lang-help = Язык интерфейса для сообщений установщика
install-summary-title = Сводка установки pyenv-native
install-network-summary-title = Сводка сетевой установки pyenv-native
install-summary-blurb = Будет создана или обновлена переносимая установка pyenv-native в выбранном корне.
install-summary-blurb-detail = Устанавливает pyenv, дружественный к агентам сервер pyenv-mcp и GUI-компаньон при наличии, пишет журнал установки и выполняет базовые проверки.
install-network-blurb = Будет скачан опубликованный пакет pyenv-native, проверена контрольная сумма SHA-256 и выполнена установка в выбранный переносимый корень.
install-profile-yes = Профиль оболочки будет обновлён, чтобы будущие сеансы находили pyenv-native автоматически.
install-profile-yes-pwsh = Профиль PowerShell будет обновлён, чтобы будущие сеансы находили pyenv-native автоматически.
install-profile-no = Изменения профиля оболочки не будут внесены.
install-profile-no-pwsh = Изменения профиля PowerShell не будут внесены.
install-continue = Продолжить установку? [y/N]:
install-need-yes = Для интерактивной установки требуется подтверждение. Повторите запуск с --yes для неинтерактивного режима.
install-need-yes-pwsh = Для интерактивной установки требуется подтверждение. Повторите запуск с -Yes для неинтерактивного режима.
install-cancelled = Установка отменена.
install-installed-to = pyenv-native установлен в { $path }
install-installed-command = Установленная команда: { $path }
install-installed-mcp = Установленный MCP-сервер: { $path }
install-mcp-helper = Помощник настройки MCP: { $command }
install-installed-gui = Установленный GUI: { $path }
install-log-file = Файл журнала: { $path }
