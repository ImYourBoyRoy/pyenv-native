# ./locales/es/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: no se puede determinar el directorio de inicio para PYENV_ROOT
error-invalid-directory = enchi: no se puede cambiar el directorio de trabajo a `{ $path }`
error-invalid-version = enchi: versión no válida `{ $version }` ignorada en `{ $path }`
error-no-local-version = enchi: no hay una versión local configurada para este directorio
error-version-not-installed =
    enchi: la versión `{ $version }` no está instalada (definida por { $origin })
    sugerencia: ejecute `enchi install { $version }` para instalarla, o `enchi versions` para ver las versiones instaladas
error-unknown-config-key = enchi: clave de configuración desconocida `{ $key }`
error-invalid-config-value = enchi: valor no válido `{ $value }` para la clave de configuración `{ $key }`
error-version-already-installed = enchi: la versión `{ $version }` ya está instalada
error-unknown-version = enchi: ninguna versión conocida coincide con `{ $version }`
error-unsupported-install-target = enchi: el backend de instalación no admite `{ $version }` en esta plataforma
error-missing-install-version = enchi: la instalación requiere al menos un argumento de versión
error-missing-python-build = enchi: no se encuentra el backend python-build; establezca install.python_build_path o añada python-build a PATH
error-checksum-mismatch = enchi: la suma de comprobación no coincide para `{ $url }` ({ $algorithm }): se esperaba { $expected }, se obtuvo { $actual }
error-missing-checksum = enchi: no se pudo obtener una suma de comprobación del editor para `{ $source }`
error-io = { $message }
error-self-update-portable = enchi: la autoactualización solo admite instalaciones portátiles lanzadas desde `{ $expected }`; el ejecutable actual es `{ $current }`
error-self-update-managed = enchi: esta instalación la gestiona un gestor de paquetes ({ $source }); actualízala con: { $command }
