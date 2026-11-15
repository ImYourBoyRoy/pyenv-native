# ./locales/tr/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: PYENV_ROOT için ev dizini belirlenemiyor
error-invalid-directory = enchi: çalışma dizini `{ $path }` konumuna değiştirilemiyor
error-invalid-version = enchi: geçersiz sürüm `{ $version }` `{ $path }` içinde yok sayıldı
error-no-local-version = enchi: bu dizin için yerel sürüm yapılandırılmamış
error-version-not-installed =
    enchi: `{ $version }` sürümü kurulu değil ({ $origin } tarafından ayarlandı)
    ipucu: kurmak için `enchi install { $version }` çalıştırın veya kurulu sürümleri görmek için `enchi versions` kullanın
error-unknown-config-key = enchi: bilinmeyen yapılandırma anahtarı `{ $key }`
error-invalid-config-value = enchi: `{ $value }` değeri `{ $key }` yapılandırma anahtarı için geçersiz
error-version-already-installed = enchi: `{ $version }` sürümü zaten kurulu
error-unknown-version = enchi: `{ $version }` ile eşleşen bilinen sürüm yok
error-unsupported-install-target = enchi: kurulum arka ucu bu platformda `{ $version }` desteklemiyor
error-missing-install-version = enchi: kurulum işlemi en az bir sürüm argümanı gerektirir
error-missing-python-build = enchi: python-build arka ucu bulunamadı; install.python_build_path ayarlayın veya python-build’i PATH’e ekleyin
error-checksum-mismatch = enchi: `{ $url }` için sağlama toplamı uyuşmazlığı ({ $algorithm }): beklenen { $expected }, alınan { $actual }
error-missing-checksum = enchi: `{ $source }` için yayımcı sağlama toplamı alınamadı
error-io = { $message }
error-self-update-portable = enchi: otomatik güncelleme yalnızca `{ $expected }` konumundan başlatılan taşınabilir kurulumları destekler; geçerli yürütülebilir `{ $current }`
error-self-update-managed = enchi: bu kurulum bir paket yöneticisi ({ $source }) tarafından yönetiliyor; şu komutla güncelleyin: { $command }
