# ./locales/pt-BR/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: não foi possível determinar o diretório inicial para PYENV_ROOT
error-invalid-directory = pyenv: não foi possível mudar o diretório de trabalho para `{ $path }`
error-invalid-version = pyenv: versão inválida `{ $version }` ignorada em `{ $path }`
error-no-local-version = pyenv: nenhuma versão local configurada para este diretório
error-version-not-installed =
    pyenv: a versão `{ $version }` não está instalada (definida por { $origin })
    dica: execute `pyenv install { $version }` para instalá-la, ou `pyenv versions` para ver as versões instaladas
error-unknown-config-key = pyenv: chave de configuração desconhecida `{ $key }`
error-invalid-config-value = pyenv: valor inválido `{ $value }` para a chave de configuração `{ $key }`
error-version-already-installed = pyenv: a versão `{ $version }` já está instalada
error-unknown-version = pyenv: nenhuma versão conhecida corresponde a `{ $version }`
error-unsupported-install-target = pyenv: o backend de instalação não oferece suporte a `{ $version }` nesta plataforma
error-missing-install-version = pyenv: a operação de instalação exige pelo menos um argumento de versão
error-missing-python-build = pyenv: não foi possível localizar o backend python-build; defina install.python_build_path ou adicione python-build ao PATH
error-checksum-mismatch = pyenv: soma de verificação incompatível para `{ $url }` ({ $algorithm }): esperado { $expected }, obtido { $actual }
error-missing-checksum = pyenv: não foi possível obter uma soma de verificação do editor para `{ $source }`
error-io = { $message }
error-self-update-portable = pyenv: a autoatualização só admite instalações portáteis iniciadas em `{ $expected }`; o executável atual é `{ $current }`
