# ./locales/zh-CN/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = 原生优先、跨平台的 Python 版本管理器
cli-global-about = 设置或显示全局 Python 版本
cli-local-about = 设置或显示当前目录的 Python 版本
cli-shell-about = 设置或显示当前 shell 的 Python 版本
cli-latest-about = 打印与前缀匹配的最新已安装或已知版本
cli-version-about = 显示当前 Python 版本及其来源
cli-version-name-about = 显示当前 Python 版本
cli-version-origin-about = 说明当前 Python 版本是如何设定的
cli-prefix-about = 显示指定 Python 版本的安装路径
cli-install-about = 从原生提供方安装 Python 版本
cli-available-about = 列出原生提供方可安装的 Python 版本
cli-versions-about = 列出 pyenv 可用的全部 Python 版本
cli-uninstall-about = 卸载指定的 Python 版本
cli-venv-about = 创建、查看并指定托管虚拟环境
cli-pip-about = 列出、检查、安装并更新运行时或虚拟环境中的包
cli-init-about = 为 pyenv 配置 shell 环境
cli-gui-about = 启动 Pyenv Native 图形控制台
cli-rehash-about = 重新生成 pyenv shim（在所有版本中安装可执行文件）
cli-shims-about = 列出现有的 pyenv shim
cli-prompt-about = 打印当前环境的简洁提示符
cli-exec-about = 使用所选 Python 版本运行可执行文件
cli-completions-about = 打印命令补全脚本
cli-doctor-about = 诊断 PATH、shim 和安装前置条件
cli-config-about = 获取、设置或显示 pyenv-native 配置
cli-self-update-about = 从 GitHub Releases 更新 pyenv-native
cli-preflight-about = 平台情报与安装就绪预检
cli-environment-about = preflight 的别名（面向代理和用户的操作系统/工具链信息）
cli-status-about = 显示完整环境状态（版本、来源、venv）
cli-root-about = 显示存放版本和 shim 的根目录
cli-which-about = 显示可执行文件的完整路径
cli-whence-about = 列出包含给定可执行文件的所有 Python 版本
cli-version-file-about = 检测设置当前 pyenv 版本的文件
cli-version-file-read-about = 读取 .python-version 文件内容
cli-self-uninstall-about = 从系统卸载 pyenv-native
cli-help-about = 显示命令帮助
cli-commands-about = 列出所有可用的 pyenv 命令
cli-hooks-about = 列出给定命令的可执行钩子
cli-venv-list-about = 列出托管虚拟环境
cli-venv-info-about = 显示托管虚拟环境的详细信息
cli-venv-create-about = 在指定运行时下创建托管虚拟环境
cli-venv-delete-about = 删除托管虚拟环境
cli-venv-rename-about = 重命名托管虚拟环境
cli-venv-use-about = 将托管虚拟环境分配到当前目录或全局
cli-venv-upgrade-about = 将托管虚拟环境升级到新的基础运行时
cli-pip-list-about = 列出目标环境中已安装的 pip 包
cli-pip-outdated-about = 列出目标环境中过时的 pip 包
cli-pip-check-about = 检查目标环境中损坏的包依赖
cli-pip-precheck-about = 安装前静态预检 requirements 文件或 HTTPS URL
cli-pip-analyze-about = 扫描 Python 源码中目标环境缺失的第三方导入
cli-pip-install-about = 从 requirements.txt 或 HTTPS URL 安装软件包
cli-pip-update-about = 更新目标环境中的 pip 包
cli-config-path-about = 显示配置文件路径
cli-config-show-about = 打印全部当前配置
cli-config-get-about = 打印指定配置键的值
cli-config-set-about = 更新一个配置键
cli-help-selection = 版本选择
cli-help-provisioning = 安装供应
cli-help-environment = 环境
cli-help-interface = 界面
cli-help-diagnostics = 诊断与配置
cli-help-maintenance = 维护
cli-help-support = 支持
cli-help-usage = 用法：pyenv <command> [<args>]
cli-help-useful = 常用 pyenv 命令：
cli-help-concepts =
    核心概念：
      Shims：拦截 `python` / `pip` 并路由到当前版本。安装 pip 包后运行 `pyenv rehash`。
      Versions：通过 `pyenv install` 安装，位于 `~/.pyenv/versions`。
      Managed envs：`~/.pyenv/venvs/<runtime>/<name>`。优先使用 `pyenv venv create` 和 `pyenv venv use`。
      Discovery：`pyenv install --list 3.13` 或 `pyenv available 3.13`。
      Selection：PYENV_VERSION，然后是 `.python-version`，然后是全局版本文件。
    
    运行 `pyenv help <command>` 查看详细帮助。文档：https://github.com/imyourboyroy/pyenv-native
