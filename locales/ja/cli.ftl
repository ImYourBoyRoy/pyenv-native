# ./locales/ja/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = ネイティブ優先のクロスプラットフォーム Python バージョン管理ツール
cli-global-about = グローバルな Python バージョンを設定または表示する
cli-local-about = カレントディレクトリの Python バージョンを設定または表示する
cli-shell-about = シェル固有の Python バージョンを設定または表示する
cli-latest-about = 接頭辞に一致する、インストール済みまたは既知の最新バージョンを表示する
cli-version-about = 現在の Python バージョンとその由来を表示する
cli-version-name-about = 現在の Python バージョンを表示する
cli-version-origin-about = 現在の Python バージョンがどのように設定されているかを説明する
cli-prefix-about = 指定した Python バージョンのインストールパスを表示する
cli-install-about = ネイティブ提供元から Python バージョンをインストールする
cli-available-about = ネイティブ提供元からインストール可能な Python バージョンを一覧表示する
cli-versions-about = pyenv が利用できるすべての Python バージョンを一覧表示する
cli-uninstall-about = 指定した Python バージョンをアンインストールする
cli-venv-about = 管理対象の仮想環境を作成、確認、割り当てる
cli-pip-about = ランタイムまたは仮想環境のパッケージを一覧、検査、インストール、更新する
cli-init-about = pyenv 向けにシェル環境を構成する
cli-gui-about = Pyenv Native の GUI ダッシュボードを起動する
cli-rehash-about = pyenv shim を再生成する（全バージョンの実行ファイルを登録）
cli-shims-about = 既存の pyenv shim を一覧表示する
cli-prompt-about = 現在の環境向けの短いプロンプト文字列を表示する
cli-exec-about = 選択中の Python バージョンで実行ファイルを起動する
cli-completions-about = コマンド補完スクリプトを表示する
cli-doctor-about = PATH、shim、インストール前提条件を診断する
cli-config-about = pyenv-native の設定を取得、変更、または表示する
cli-self-update-about = GitHub Releases から pyenv-native を更新する
cli-preflight-about = プラットフォーム情報とインストール準備の preflight
cli-environment-about = preflight の別名（エージェントとユーザー向けの OS/ツールチェーン情報）
cli-status-about = 包括的な環境ステータス（バージョン、由来、venv）を表示する
cli-root-about = バージョンと shim を置くルートディレクトリを表示する
cli-which-about = 実行ファイルのフルパスを表示する
cli-whence-about = 指定した実行ファイルを含むすべての Python バージョンを一覧表示する
cli-version-file-about = 現在の pyenv バージョンを設定しているファイルを検出する
cli-version-file-read-about = .python-version ファイルの内容を読み取る
cli-self-uninstall-about = システムから pyenv-native をアンインストールする
cli-help-about = コマンドのヘルプを表示する
cli-commands-about = 利用できるすべての pyenv コマンドを一覧表示する
cli-hooks-about = 指定コマンドの実行可能なフックを一覧表示する
cli-venv-list-about = 管理対象の仮想環境を一覧表示する
cli-venv-info-about = 管理対象の仮想環境の詳細を表示する
cli-venv-create-about = 指定ランタイムの下に管理対象の仮想環境を作成する
cli-venv-delete-about = 管理対象の仮想環境を削除する
cli-venv-rename-about = 管理対象の仮想環境を名前変更する
cli-venv-use-about = 管理対象の仮想環境をカレントディレクトリまたはグローバルに割り当てる
cli-venv-upgrade-about = 管理対象の仮想環境を新しいベースランタイムへアップグレードする
cli-pip-list-about = 対象環境にインストール済みの pip パッケージを一覧表示する
cli-pip-outdated-about = 対象環境の古くなった pip パッケージを一覧表示する
cli-pip-check-about = 対象環境の壊れたパッケージ要件を検査する
cli-pip-precheck-about = インストール前に requirements ファイルまたは HTTPS URL を静的に事前チェックする
cli-pip-analyze-about = Python ソースを走査し、対象に欠けているサードパーティ import を見つける
cli-pip-install-about = requirements.txt または HTTPS URL からパッケージをインストールする
cli-pip-update-about = 対象環境内の pip パッケージを更新する
cli-config-path-about = 設定ファイルのパスを表示する
cli-config-show-about = 現在の設定をすべて表示する
cli-config-get-about = 指定した設定キーの値を表示する
cli-config-set-about = 設定キーを更新する
cli-help-selection = バージョン選択
cli-help-provisioning = プロビジョニング
cli-help-environment = 環境
cli-help-interface = インターフェイス
cli-help-diagnostics = 診断と設定
cli-help-maintenance = メンテナンス
cli-help-support = サポート
cli-help-usage = 使い方: pyenv <command> [<args>]
cli-help-useful = よく使う pyenv コマンド:
cli-help-concepts =
    基本概念:
      Shims: `python` / `pip` を横取りして現在のバージョンへ送る実行ファイル。pip パッケージ追加後は `pyenv rehash`。
      Versions: `pyenv install` で入れる環境。`~/.pyenv/versions`。
      Managed envs: `~/.pyenv/venvs/<runtime>/<name>`。`pyenv venv create` と `pyenv venv use` を推奨。
      Discovery: `pyenv install --list 3.13` または `pyenv available 3.13`。
      Selection: PYENV_VERSION、次に `.python-version`、次にグローバル版。
    
    詳細は `pyenv help <command>`。文書: https://github.com/imyourboyroy/pyenv-native
