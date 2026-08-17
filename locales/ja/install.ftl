# ./locales/ja/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = pyenv-native { $version } をダウンロードしています
install-extracting = リリースバンドルを展開しています
install-installing = { $path } にインストールしています
install-done = pyenv-native をインストールしました。新しい端末を開き、`pyenv doctor` を実行してください。
install-failed = インストールに失敗しました
install-lang-help = インストーラーメッセージのインターフェイス言語
install-summary-title = pyenv-native インストール概要
install-network-summary-title = pyenv-native ネットワークインストール概要
install-summary-blurb = 選択したルートの下に、ポータブルな pyenv-native インストールを作成または更新します。
install-summary-blurb-detail = pyenv に加え、エージェント向け pyenv-mcp サーバーと、利用できる場合は GUI コンパニオンをインストールし、インストールログを書き、基本的な健全性チェックを実行します。
install-network-blurb = 公開済みの pyenv-native バンドルをダウンロードし、SHA-256 チェックサムを検証して、選択したポータブルルートへインストールします。
install-profile-yes = 今後のセッションが pyenv-native を自動検出できるよう、シェルプロファイルを更新します。
install-profile-yes-pwsh = 今後のセッションが pyenv-native を自動検出できるよう、PowerShell プロファイルを更新します。
install-profile-no = シェルプロファイルは変更しません。
install-profile-no-pwsh = PowerShell プロファイルは変更しません。
install-continue = インストールを続行しますか？ [y/N]:
install-need-yes = 対話型インストールには確認が必要です。非対話で使うには --yes を付けて再実行してください。
install-need-yes-pwsh = 対話型インストールには確認が必要です。非対話で使うには -Yes を付けて再実行してください。
install-cancelled = インストールをキャンセルしました。
install-installed-to = pyenv-native を { $path } にインストールしました
install-installed-command = インストールしたコマンド: { $path }
install-installed-mcp = インストールした MCP サーバー: { $path }
install-mcp-helper = MCP 設定ヘルパー: { $command }
install-installed-gui = インストールした GUI: { $path }
install-log-file = ログファイル: { $path }
