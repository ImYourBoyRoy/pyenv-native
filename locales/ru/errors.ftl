# ./locales/ru/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: не удалось определить домашний каталог для PYENV_ROOT
error-invalid-directory = pyenv: не удалось сменить рабочий каталог на `{ $path }`
error-invalid-version = pyenv: недопустимая версия `{ $version }` проигнорирована в `{ $path }`
error-no-local-version = pyenv: для этого каталога не настроена локальная версия
error-version-not-installed =
    pyenv: версия `{ $version }` не установлена (задана { $origin })
    подсказка: выполните `pyenv install { $version }` для установки или `pyenv versions`, чтобы увидеть установленные версии
error-unknown-config-key = pyenv: неизвестный ключ конфигурации `{ $key }`
error-invalid-config-value = pyenv: недопустимое значение `{ $value }` для ключа конфигурации `{ $key }`
error-version-already-installed = pyenv: версия `{ $version }` уже установлена
error-unknown-version = pyenv: нет известных версий, соответствующих `{ $version }`
error-unsupported-install-target = pyenv: сервер установки не поддерживает `{ $version }` на этой платформе
error-missing-install-version = pyenv: для установки нужен хотя бы один аргумент версии
error-missing-python-build = pyenv: не найден сервер python-build; задайте install.python_build_path или добавьте python-build в PATH
error-checksum-mismatch = pyenv: несовпадение контрольной суммы для `{ $url }` ({ $algorithm }): ожидалось { $expected }, получено { $actual }
error-missing-checksum = pyenv: не удалось получить контрольную сумму издателя для `{ $source }`
error-io = { $message }
error-self-update-portable = pyenv: самообновление поддерживает только переносимые установки, запущенные из `{ $expected }`; текущий файл — `{ $current }`
