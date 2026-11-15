# ./locales/en-US/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: cannot determine home directory for PYENV_ROOT
error-invalid-directory = enchi: cannot change working directory to `{ $path }`
error-invalid-version = enchi: invalid version `{ $version }` ignored in `{ $path }`
error-no-local-version = enchi: no local version configured for this directory
error-version-not-installed =
    enchi: version `{ $version }` is not installed (set by { $origin })
    hint: run `enchi install { $version }` to install it, or `enchi versions` to see installed versions
error-unknown-config-key = enchi: unknown config key `{ $key }`
error-invalid-config-value = enchi: invalid value `{ $value }` for config key `{ $key }`
error-version-already-installed = enchi: version `{ $version }` is already installed
error-unknown-version = enchi: no known versions match `{ $version }`
error-unsupported-install-target = enchi: install backend does not support `{ $version }` on this platform
error-missing-install-version = enchi: install operation requires at least one version argument
error-missing-python-build = enchi: unable to locate python-build backend; set install.python_build_path or add python-build to PATH
error-checksum-mismatch = enchi: checksum mismatch for `{ $url }` ({ $algorithm }): expected { $expected }, got { $actual }
error-missing-checksum = enchi: unable to obtain a publisher checksum for `{ $source }`
error-io = { $message }
error-self-update-portable = enchi: self-update only supports portable installs launched from `{ $expected }`; current executable is `{ $current }`
error-self-update-managed = enchi: this install is managed by a package manager ({ $source }); update it with: { $command }
