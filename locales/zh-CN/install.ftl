# ./locales/zh-CN/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = 正在下载 pyenv-native { $version }
install-extracting = 正在解压发布包
install-installing = 正在安装到 { $path }
install-done = pyenv-native 已安装。打开新终端，然后运行 `pyenv doctor`。
install-failed = 安装失败
install-lang-help = 安装程序消息的界面语言
install-summary-title = pyenv-native 安装摘要
install-network-summary-title = pyenv-native 网络安装摘要
install-summary-blurb = 将在所选根目录下创建或更新便携式 pyenv-native 安装。
install-summary-blurb-detail = 会安装 pyenv 以及面向代理的 pyenv-mcp 服务器，并在可用时安装 GUI 配套程序，写入安装日志，并运行基本健全性检查。
install-network-blurb = 将下载已发布的 pyenv-native 包，校验其 SHA-256 校验和，并安装到所选便携根目录。
install-profile-yes = 将更新你的 shell 配置文件，以便日后会话能自动找到 pyenv-native。
install-profile-yes-pwsh = 将更新你的 PowerShell 配置文件，以便日后会话能自动找到 pyenv-native。
install-profile-no = 不会修改 shell 配置文件。
install-profile-no-pwsh = 不会修改 PowerShell 配置文件。
install-continue = 继续安装？ [y/N]:
install-need-yes = 交互式安装需要确认。请加上 --yes 重新运行以用于非交互场景。
install-need-yes-pwsh = 交互式安装需要确认。请加上 -Yes 重新运行以用于非交互场景。
install-cancelled = 已取消安装。
install-installed-to = 已将 pyenv-native 安装到 { $path }
install-installed-command = 已安装命令：{ $path }
install-installed-mcp = 已安装 MCP 服务器：{ $path }
install-mcp-helper = MCP 配置助手：{ $command }
install-installed-gui = 已安装 GUI：{ $path }
install-log-file = 日志文件：{ $path }
