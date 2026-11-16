# ./locales/pt-BR/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: não foi possível determinar o diretório inicial para ENCHI_ROOT
error-invalid-directory = enchi: não foi possível mudar o diretório de trabalho para `{ $path }`
error-invalid-version = enchi: versão inválida `{ $version }` ignorada em `{ $path }`
error-no-local-version = enchi: nenhuma versão local configurada para este diretório
error-version-not-installed =
    enchi: a versão `{ $version }` não está instalada (definida por { $origin })
    dica: execute `enchi install { $version }` para instalá-la, ou `enchi versions` para ver as versões instaladas
error-unknown-config-key = enchi: chave de configuração desconhecida `{ $key }`
error-invalid-config-value = enchi: valor inválido `{ $value }` para a chave de configuração `{ $key }`
error-version-already-installed = enchi: a versão `{ $version }` já está instalada
error-unknown-version = enchi: nenhuma versão conhecida corresponde a `{ $version }`
error-unsupported-install-target = enchi: o backend de instalação não oferece suporte a `{ $version }` nesta plataforma
error-missing-install-version = enchi: a operação de instalação exige pelo menos um argumento de versão
error-missing-python-build = enchi: não foi possível localizar o backend python-build; defina install.python_build_path ou adicione python-build ao PATH
error-checksum-mismatch = enchi: soma de verificação incompatível para `{ $url }` ({ $algorithm }): esperado { $expected }, obtido { $actual }
error-missing-checksum = enchi: não foi possível obter uma soma de verificação do publicador para `{ $source }`
error-io = { $message }
error-self-update-portable = enchi: a autoatualização só oferece suporte a instalações portáteis iniciadas a partir de `{ $expected }`; o executável atual é `{ $current }`
error-self-update-managed = enchi: esta instalação é gerenciada por um gerenciador de pacotes ({ $source }); atualize com: { $command }
