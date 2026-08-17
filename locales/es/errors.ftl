# ./locales/es/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: no se puede determinar el directorio de inicio para PYENV_ROOT
error-invalid-directory = pyenv: no se puede cambiar el directorio de trabajo a `{ $path }`
error-invalid-version = pyenv: versión no válida `{ $version }` ignorada en `{ $path }`
error-no-local-version = pyenv: no hay una versión local configurada para este directorio
error-version-not-installed =
    pyenv: la versión `{ $version }` no está instalada (definida por { $origin })
    sugerencia: ejecute `pyenv install { $version }` para instalarla, o `pyenv versions` para ver las versiones instaladas
error-unknown-config-key = pyenv: clave de configuración desconocida `{ $key }`
error-invalid-config-value = pyenv: valor no válido `{ $value }` para la clave de configuración `{ $key }`
error-version-already-installed = pyenv: la versión `{ $version }` ya está instalada
error-unknown-version = pyenv: ninguna versión conocida coincide con `{ $version }`
error-unsupported-install-target = pyenv: el backend de instalación no admite `{ $version }` en esta plataforma
error-missing-install-version = pyenv: la instalación requiere al menos un argumento de versión
error-missing-python-build = pyenv: no se encuentra el backend python-build; establezca install.python_build_path o añada python-build a PATH
error-checksum-mismatch = pyenv: la suma de comprobación no coincide para `{ $url }` ({ $algorithm }): se esperaba { $expected }, se obtuvo { $actual }
error-missing-checksum = pyenv: no se pudo obtener una suma de comprobación del editor para `{ $source }`
error-io = { $message }
error-self-update-portable = pyenv: la autoactualización solo admite instalaciones portátiles lanzadas desde `{ $expected }`; el ejecutable actual es `{ $current }`
