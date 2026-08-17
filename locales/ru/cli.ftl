# ./locales/ru/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Нативный кроссплатформенный менеджер версий Python
cli-global-about = Задать или показать глобальную версию Python
cli-local-about = Задать или показать локальную версию Python каталога
cli-shell-about = Задать или показать версию Python для текущей оболочки
cli-latest-about = Вывести последнюю установленную или известную версию по префиксу
cli-version-about = Показать текущую версию Python и её источник
cli-version-name-about = Показать текущую версию Python
cli-version-origin-about = Пояснить, как задана текущая версия Python
cli-prefix-about = Показать пути, где установлены указанные версии Python
cli-install-about = Установить версии Python из нативных источников
cli-available-about = Список устанавливаемых версий Python из нативных источников
cli-versions-about = Список всех версий Python, доступных pyenv
cli-uninstall-about = Удалить указанную версию Python
cli-venv-about = Создавать, просматривать и назначать управляемые виртуальные окружения
cli-pip-about = Просматривать, проверять, устанавливать и обновлять пакеты для среды выполнения или venv
cli-init-about = Настроить окружение оболочки для pyenv
cli-gui-about = Запустить графическую панель Pyenv Native
cli-rehash-about = Пересоздать шимы pyenv (устанавливает исполняемые файлы для всех версий)
cli-shims-about = Список существующих шимов pyenv
cli-prompt-about = Вывести краткую строку приглашения для текущего окружения
cli-exec-about = Запустить исполняемый файл с выбранной версией Python
cli-completions-about = Вывести скрипт автодополнения команд
cli-doctor-about = Диагностика PATH, шимов и требований к установке
cli-config-about = Получить, задать или показать конфигурацию pyenv-native
cli-self-update-about = Обновить pyenv-native из GitHub Releases
cli-preflight-about = Сведения о платформе и preflight готовности к установке
cli-environment-about = Псевдоним preflight (сведения об ОС/инструментах для агентов и пользователей)
cli-status-about = Показать полный статус окружения (версии, источники, venv)
cli-root-about = Показать корневой каталог, где хранятся версии и shim
cli-which-about = Показать полный путь к исполняемому файлу
cli-whence-about = Перечислить все версии Python, содержащие указанный исполняемый файл
cli-version-file-about = Обнаружить файл, задающий текущую версию pyenv
cli-version-file-read-about = Прочитать содержимое файла .python-version
cli-self-uninstall-about = Удалить pyenv-native из системы
cli-help-about = Показать справку по команде
cli-commands-about = Перечислить все доступные команды pyenv
cli-hooks-about = Перечислить исполняемые хуки для указанной команды
cli-venv-list-about = Перечислить управляемые виртуальные окружения
cli-venv-info-about = Показать сведения об управляемом виртуальном окружении
cli-venv-create-about = Создать управляемое виртуальное окружение под указанной средой выполнения
cli-venv-delete-about = Удалить управляемое виртуальное окружение
cli-venv-rename-about = Переименовать управляемое виртуальное окружение
cli-venv-use-about = Назначить управляемое виртуальное окружение текущему каталогу или глобально
cli-venv-upgrade-about = Обновить управляемое виртуальное окружение до новой базовой среды выполнения
cli-pip-list-about = Перечислить установленные пакеты pip в целевом окружении
cli-pip-outdated-about = Перечислить устаревшие пакеты pip в целевом окружении
cli-pip-check-about = Проверить повреждённые требования пакетов в целевом окружении
cli-pip-precheck-about = Статически предварительно проверить файл требований или HTTPS URL перед установкой
cli-pip-analyze-about = Сканировать исходники Python на сторонние импорты, отсутствующие в цели
cli-pip-install-about = Установить пакеты из файла requirements.txt или HTTPS URL
cli-pip-update-about = Обновить пакеты pip в целевом окружении
cli-config-path-about = Показать путь к файлу конфигурации
cli-config-show-about = Вывести всю текущую конфигурацию
cli-config-get-about = Вывести значение конкретного ключа конфигурации
cli-config-set-about = Обновить ключ конфигурации
cli-help-selection = ВЫБОР
cli-help-provisioning = УСТАНОВКА
cli-help-environment = ОКРУЖЕНИЕ
cli-help-interface = ИНТЕРФЕЙС
cli-help-diagnostics = ДИАГНОСТИКА И КОНФИГ
cli-help-maintenance = ОБСЛУЖИВАНИЕ
cli-help-support = ПОДДЕРЖКА
cli-help-usage = Использование: pyenv <command> [<args>]
cli-help-useful = Полезные команды pyenv:
cli-help-concepts =
    ОСНОВНЫЕ ПОНЯТИЯ:
      Shims: лёгкие исполняемые файлы (`python` или `pip`), перехватывающие команды и направляющие их к текущей версии. После установки pip-пакетов выполните `pyenv rehash`.
      Versions: окружения, установленные через `pyenv install`, в `~/.pyenv/versions`.
      Managed envs: `~/.pyenv/venvs/<runtime>/<name>`. Предпочитайте `pyenv venv create` и `pyenv venv use`.
      Discovery: `pyenv install --list 3.13` или `pyenv available 3.13`.
      Selection: PYENV_VERSION, затем `.python-version`, затем глобальный файл версии.
    
    Подробности: `pyenv help <command>`. Документация: https://github.com/imyourboyroy/pyenv-native
