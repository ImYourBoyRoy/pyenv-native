# ./locales/ja/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: ENCHI_ROOT 用のホームディレクトリを特定できません
error-invalid-directory = enchi: 作業ディレクトリを `{ $path }` に変更できません
error-invalid-version = enchi: 無効なバージョン `{ $version }` を `{ $path }` で無視しました
error-no-local-version = enchi: このディレクトリにローカルバージョンが設定されていません
error-version-not-installed =
    enchi: バージョン `{ $version }` はインストールされていません（{ $origin } によって設定）
    ヒント: `enchi install { $version }` でインストールするか、`enchi versions` でインストール済みバージョンを確認してください
error-unknown-config-key = enchi: 不明な設定キー `{ $key }`
error-invalid-config-value = enchi: 設定キー `{ $key }` の値 `{ $value }` は無効です
error-version-already-installed = enchi: バージョン `{ $version }` はすでにインストールされています
error-unknown-version = enchi: `{ $version }` に一致する既知のバージョンがありません
error-unsupported-install-target = enchi: このプラットフォームのインストールバックエンドは `{ $version }` をサポートしていません
error-missing-install-version = enchi: インストール操作には少なくとも 1 つのバージョン引数が必要です
error-missing-python-build = enchi: python-build バックエンドを見つけられません。install.python_build_path を設定するか、python-build を PATH に追加してください
error-checksum-mismatch = enchi: `{ $url }` のチェックサムが一致しません（{ $algorithm }）: 期待値 { $expected }、実際 { $actual }
error-missing-checksum = enchi: `{ $source }` の公開元チェックサムを取得できません
error-io = { $message }
error-self-update-portable = enchi: 自己更新は `{ $expected }` から起動したポータブルインストールのみ対応です。現在の実行ファイルは `{ $current }` です
error-self-update-managed = enchi: このインストールはパッケージマネージャー（{ $source }）が管理しています。次のコマンドで更新してください: { $command }
