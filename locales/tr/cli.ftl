# ./locales/tr/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Python çalışma zamanı ve ortam yöneticisi
cli-global-about = Genel Python sürümünü ayarla veya göster
cli-local-about = Dizin yerel Python sürümünü ayarla veya göster
cli-shell-about = Kabuğa özel Python sürümünü ayarla veya göster
cli-latest-about = Önekle eşleşen en son kurulu veya bilinen sürümü yazdır
cli-version-about = Geçerli Python sürümünü ve kaynağını göster
cli-version-name-about = Geçerli Python sürümünü göster
cli-version-origin-about = Geçerli Python sürümünün nasıl ayarlandığını açıkla
cli-prefix-about = Verilen Python sürümlerinin kurulduğu yolları göster
cli-install-about = Yerel sağlayıcılardan Python sürümleri kur
cli-available-about = Yerel sağlayıcılardan kurulabilir Python sürümlerini listele
cli-versions-about = enchi için kullanılabilir tüm Python sürümlerini listele
cli-upgrade-about = Kurulu çalışma zamanlarını en yeni yama sürümüne taşı (venv'ler dahil)
cli-uninstall-about = Belirli bir Python sürümünü kaldır
cli-venv-about = Yönetilen sanal ortamları oluştur, incele ve ata
cli-pip-about = Bir çalışma zamanı veya venv için paketleri listele, denetle, kur ve güncelle
cli-init-about = enchi için kabuk ortamını yapılandır
cli-gui-about = Enchi grafik panelini başlat
cli-rehash-about = enchi shim’lerini yeniden oluştur (tüm sürümlerde yürütülebilirleri kurar)
cli-shims-about = Mevcut enchi shim’lerini listele
cli-prompt-about = Geçerli ortam için kısa bir komut istemi dizesi yazdır
cli-exec-about = Seçili Python sürümüyle bir yürütülebilir çalıştır
cli-completions-about = Komut tamamlama betiğini yazdır
cli-doctor-about = PATH, shim’ler ve kurulum önkoşullarını tanıla
cli-config-about = Enchi yapılandırmasını al, ayarla veya göster
cli-self-update-about = Enchi’yi GitHub Releases’ten güncelle
cli-preflight-about = Platform bilgisi ve kurulum hazırlığı preflight
cli-environment-about = preflight için takma ad (aracılar ve kullanıcılar için OS/araç zinciri bilgileri)
cli-status-about = Kapsamlı ortam durumunu göster (sürümler, kaynaklar, venv’ler)
cli-root-about = Sürümlerin ve shim’lerin tutulduğu kök dizini göster
cli-which-about = Bir yürütülebilirin tam yolunu göster
cli-whence-about = Verilen yürütülebiliri içeren tüm Python sürümlerini listele
cli-version-file-about = Geçerli enchi sürümünü ayarlayan dosyayı algıla
cli-version-file-read-about = Bir .python-version dosyasının içeriğini oku
cli-self-install-about = Enchi’yi bir sürüm paketinden kur veya onar
cli-self-uninstall-about = Enchi’yi sistemden kaldır
cli-agent-skills-about = Kodlama asistanları için Enchi becerisini kur
cli-help-about = Bir komutun yardımını göster
cli-commands-about = Kullanılabilir tüm enchi komutlarını listele
cli-hooks-about = Verilen komutun çalıştırılabilir kancalarını listele
cli-venv-list-about = Yönetilen sanal ortamları listele
cli-venv-info-about = Yönetilen sanal ortamın ayrıntılarını göster
cli-venv-create-about = Belirli bir çalışma zamanı altında yönetilen sanal ortam oluştur
cli-venv-delete-about = Yönetilen sanal ortamı kaldır
cli-venv-rename-about = Yönetilen sanal ortamı yeniden adlandır
cli-venv-use-about = Yönetilen sanal ortamı geçerli dizine veya genel olarak ata
cli-venv-upgrade-about = Yönetilen sanal ortamı yeni bir taban çalışma zamanına yükselt
cli-pip-list-about = Hedef ortamda kurulu pip paketlerini listele
cli-pip-outdated-about = Hedef ortamda güncel olmayan pip paketlerini listele
cli-pip-check-about = Hedef ortamda bozuk paket gereksinimlerini denetle
cli-pip-precheck-about = Kurulumdan önce bir gereksinim dosyasını veya HTTPS URL’yi durağan olarak ön denetle
cli-pip-analyze-about = Python kaynaklarını hedeften eksik üçüncü taraf içe aktarmalar için tara
cli-pip-install-about = requirements.txt dosyasından veya HTTPS URL’den paket kur
cli-pip-update-about = Hedef ortam içindeki pip paketlerini güncelle
cli-config-path-about = Yapılandırma dosyasının yolunu göster
cli-config-show-about = Tüm geçerli yapılandırmayı yazdır
cli-config-get-about = Belirli bir yapılandırma anahtarının değerini yazdır
cli-config-set-about = Bir yapılandırma anahtarını güncelle
cli-help-selection = SEÇİM
cli-help-provisioning = KURULUM
cli-help-environment = ORTAM
cli-help-interface = ARAYÜZ
cli-help-diagnostics = TANILAMA VE YAPILANDIRMA
cli-help-maintenance = BAKIM
cli-help-support = DESTEK
cli-help-usage = Kullanım: enchi <command> [<args>]
cli-help-useful = Yararlı enchi komutları:
cli-help-concepts =
    TEMEL KAVRAMLAR:
      Shim’ler: `python` veya `pip` gibi komutlarınızı yakalayıp doğru Python sürümüne yönlendiren hafif çalıştırılabilir dosyalar. Yeni pip paketleri kurduktan sonra `enchi rehash` çalıştırın.
      Sürümler: `enchi install` ile kurulan Python ortamları, `$(enchi root)/versions` altında.
      Yönetilen ortamlar: `$(enchi root)/venvs/<runtime>/<name>` altındaki adlandırılmış sanal ortamlar. `enchi venv create` ve `enchi venv use` komutlarını tercih edin.
      Keşif: `enchi install --list 3.13` veya `enchi available 3.13`.
      Seçim: PYENV_VERSION, ardından `.python-version`, ardından genel sürüm dosyası.
    
    Ayrıntılı yardım için `enchi help <command>` çalıştırın. Belgeler: https://github.com/imyourboyroy/pyenv-native
