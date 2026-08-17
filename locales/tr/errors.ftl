# ./locales/tr/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: PYENV_ROOT için ev dizini belirlenemiyor
error-invalid-directory = pyenv: çalışma dizini `{ $path }` konumuna değiştirilemiyor
error-invalid-version = pyenv: geçersiz sürüm `{ $version }` `{ $path }` içinde yok sayıldı
error-no-local-version = pyenv: bu dizin için yerel sürüm yapılandırılmamış
error-version-not-installed =
    pyenv: `{ $version }` sürümü kurulu değil ({ $origin } tarafından ayarlandı)
    ipucu: kurmak için `pyenv install { $version }` çalıştırın veya kurulu sürümleri görmek için `pyenv versions` kullanın
error-unknown-config-key = pyenv: bilinmeyen yapılandırma anahtarı `{ $key }`
error-invalid-config-value = pyenv: `{ $value }` değeri `{ $key }` yapılandırma anahtarı için geçersiz
error-version-already-installed = pyenv: `{ $version }` sürümü zaten kurulu
error-unknown-version = pyenv: `{ $version }` ile eşleşen bilinen sürüm yok
error-unsupported-install-target = pyenv: kurulum arka ucu bu platformda `{ $version }` desteklemiyor
error-missing-install-version = pyenv: kurulum işlemi en az bir sürüm argümanı gerektirir
error-missing-python-build = pyenv: python-build arka ucu bulunamadı; install.python_build_path ayarlayın veya python-build’i PATH’e ekleyin
error-checksum-mismatch = pyenv: `{ $url }` için sağlama toplamı uyuşmazlığı ({ $algorithm }): beklenen { $expected }, alınan { $actual }
error-missing-checksum = pyenv: `{ $source }` için yayımcı sağlama toplamı alınamadı
error-io = { $message }
error-self-update-portable = pyenv: otomatik güncelleme yalnızca `{ $expected }` konumundan başlatılan taşınabilir kurulumları destekler; geçerli yürütülebilir `{ $current }`
