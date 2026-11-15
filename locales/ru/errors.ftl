# ./locales/ru/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: не удалось определить домашний каталог для PYENV_ROOT
error-invalid-directory = enchi: не удалось сменить рабочий каталог на `{ $path }`
error-invalid-version = enchi: недопустимая версия `{ $version }` проигнорирована в `{ $path }`
error-no-local-version = enchi: для этого каталога не настроена локальная версия
error-version-not-installed =
    enchi: версия `{ $version }` не установлена (задана { $origin })
    подсказка: выполните `enchi install { $version }` для установки или `enchi versions`, чтобы увидеть установленные версии
error-unknown-config-key = enchi: неизвестный ключ конфигурации `{ $key }`
error-invalid-config-value = enchi: недопустимое значение `{ $value }` для ключа конфигурации `{ $key }`
error-version-already-installed = enchi: версия `{ $version }` уже установлена
error-unknown-version = enchi: нет известных версий, соответствующих `{ $version }`
error-unsupported-install-target = enchi: сервер установки не поддерживает `{ $version }` на этой платформе
error-missing-install-version = enchi: для установки нужен хотя бы один аргумент версии
error-missing-python-build = enchi: не найден сервер python-build; задайте install.python_build_path или добавьте python-build в PATH
error-checksum-mismatch = enchi: несовпадение контрольной суммы для `{ $url }` ({ $algorithm }): ожидалось { $expected }, получено { $actual }
error-missing-checksum = enchi: не удалось получить контрольную сумму издателя для `{ $source }`
error-io = { $message }
error-self-update-portable = enchi: самообновление поддерживает только переносимые установки, запущенные из `{ $expected }`; текущий файл — `{ $current }`
error-self-update-managed = enchi: эта установка управляется менеджером пакетов ({ $source }); обновите её командой: { $command }
