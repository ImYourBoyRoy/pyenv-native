# ./locales/zh-CN/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: 无法确定 PYENV_ROOT 的主目录
error-invalid-directory = pyenv: 无法将工作目录切换到 `{ $path }`
error-invalid-version = pyenv: 无效版本 `{ $version }` 已在 `{ $path }` 中被忽略
error-no-local-version = pyenv: 此目录未配置本地版本
error-version-not-installed =
    pyenv: 版本 `{ $version }` 未安装（由 { $origin } 设定）
    提示: 运行 `pyenv install { $version }` 进行安装，或运行 `pyenv versions` 查看已安装版本
error-unknown-config-key = pyenv: 未知配置键 `{ $key }`
error-invalid-config-value = pyenv: 无效值 `{ $value }`，配置键 `{ $key }`
error-version-already-installed = pyenv: 版本 `{ $version }` 已安装
error-unknown-version = pyenv: 没有与 `{ $version }` 匹配的已知版本
error-unsupported-install-target = pyenv: 安装后端在此平台不支持 `{ $version }`
error-missing-install-version = pyenv: 安装操作至少需要一个版本参数
error-missing-python-build = pyenv: 找不到 python-build 后端；请设置 install.python_build_path 或将 python-build 加入 PATH
error-checksum-mismatch = pyenv: `{ $url }` 校验和不匹配（{ $algorithm }）：期望 { $expected }，实际 { $actual }
error-missing-checksum = pyenv: 无法获取 `{ $source }` 的发布方校验和
error-io = { $message }
error-self-update-portable = pyenv：自更新仅支持从 `{ $expected }` 启动的便携安装；当前可执行文件是 `{ $current }`
