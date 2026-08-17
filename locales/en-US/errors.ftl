# ./locales/en-US/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: cannot determine home directory for PYENV_ROOT
error-invalid-directory = pyenv: cannot change working directory to `{ $path }`
error-invalid-version = pyenv: invalid version `{ $version }` ignored in `{ $path }`
error-no-local-version = pyenv: no local version configured for this directory
error-version-not-installed =
    pyenv: version `{ $version }` is not installed (set by { $origin })
    hint: run `pyenv install { $version }` to install it, or `pyenv versions` to see installed versions
error-unknown-config-key = pyenv: unknown config key `{ $key }`
error-invalid-config-value = pyenv: invalid value `{ $value }` for config key `{ $key }`
error-version-already-installed = pyenv: version `{ $version }` is already installed
error-unknown-version = pyenv: no known versions match `{ $version }`
error-unsupported-install-target = pyenv: install backend does not support `{ $version }` on this platform
error-missing-install-version = pyenv: install operation requires at least one version argument
error-missing-python-build = pyenv: unable to locate python-build backend; set install.python_build_path or add python-build to PATH
error-checksum-mismatch = pyenv: checksum mismatch for `{ $url }` ({ $algorithm }): expected { $expected }, got { $actual }
error-missing-checksum = pyenv: unable to obtain a publisher checksum for `{ $source }`
error-io = { $message }
error-self-update-portable = pyenv: self-update only supports portable installs launched from `{ $expected }`; current executable is `{ $current }`
