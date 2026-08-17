# ./locales/ja/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: PYENV_ROOT のホームディレクトリを特定できません
error-invalid-directory = pyenv: 作業ディレクトリを `{ $path }` に変更できません
error-invalid-version = pyenv: 無効なバージョン `{ $version }` を `{ $path }` で無視しました
error-no-local-version = pyenv: このディレクトリにローカルバージョンが設定されていません
error-version-not-installed =
    pyenv: バージョン `{ $version }` はインストールされていません（{ $origin } によって設定）
    ヒント: `pyenv install { $version }` でインストールするか、`pyenv versions` でインストール済みバージョンを確認してください
error-unknown-config-key = pyenv: 未知の設定キー `{ $key }`
error-invalid-config-value = pyenv: 設定キーに対する無効な値 `{ $value }`（キー `{ $key }`）
error-version-already-installed = pyenv: バージョン `{ $version }` はすでにインストールされています
error-unknown-version = pyenv: `{ $version }` に一致する既知のバージョンがありません
error-unsupported-install-target = pyenv: このプラットフォームのインストールバックエンドは `{ $version }` をサポートしていません
error-missing-install-version = pyenv: インストール操作には少なくとも 1 つのバージョン引数が必要です
error-missing-python-build = pyenv: python-build バックエンドを見つけられません。install.python_build_path を設定するか、python-build を PATH に追加してください
error-checksum-mismatch = pyenv: `{ $url }` のチェックサムが一致しません（{ $algorithm }）: 期待値 { $expected }、実際 { $actual }
error-missing-checksum = pyenv: `{ $source }` の公開元チェックサムを取得できません
error-io = { $message }
error-self-update-portable = pyenv: 自己更新は `{ $expected }` から起動したポータブルインストールのみ対応です。現在の実行ファイルは `{ $current }` です
