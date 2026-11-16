# ./locales/ru/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = Загрузка { $version }
install-extracting = Распаковка пакета выпуска
install-installing = Установка в { $path }
install-done = Enchi установлен. Откройте новый терминал и выполните `enchi doctor`.
install-failed = Установка не удалась
install-lang-help = Язык интерфейса для сообщений установщика
install-summary-title = Сводка установки Enchi
install-network-summary-title = Сводка сетевой установки Enchi
install-summary-blurb = Будет создана или обновлена переносимая установка Enchi в выбранном корневом каталоге.
install-summary-blurb-detail = Устанавливает enchi, удобный для агентов сервер enchi-mcp и приложение с графическим интерфейсом (при наличии), записывает журнал установки и выполняет базовые проверки.
install-network-blurb = Будет скачан опубликованный пакет Enchi, проверена контрольная сумма SHA-256 и выполнена установка в выбранный переносимый корневой каталог.
install-profile-yes = Профиль оболочки будет обновлён, чтобы будущие сеансы находили Enchi автоматически.
install-profile-yes-pwsh = Профиль PowerShell будет обновлён, чтобы будущие сеансы находили Enchi автоматически.
install-profile-no = Изменения профиля оболочки не будут внесены.
install-profile-no-pwsh = Изменения профиля PowerShell не будут внесены.
install-continue = Продолжить установку? [y/N]:
install-need-yes = Для интерактивной установки требуется подтверждение. Повторите запуск с --yes для неинтерактивного режима.
install-need-yes-pwsh = Для интерактивной установки требуется подтверждение. Повторите запуск с -Yes для неинтерактивного режима.
install-cancelled = Установка отменена.
install-installed-to = Enchi установлен в { $path }
install-installed-command = Установленная команда: { $path }
install-installed-mcp = Установленный MCP-сервер: { $path }
install-mcp-helper = Помощник настройки MCP: { $command }
install-installed-gui = Установленный GUI: { $path }
install-log-file = Файл журнала: { $path }
