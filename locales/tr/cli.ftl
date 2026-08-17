# ./locales/tr/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = Yerel odaklı, çapraz platformlu Python sürüm yöneticisi
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
cli-versions-about = pyenv için kullanılabilir tüm Python sürümlerini listele
cli-uninstall-about = Belirli bir Python sürümünü kaldır
cli-venv-about = Yönetilen sanal ortamları oluştur, incele ve ata
cli-pip-about = Bir çalışma zamanı veya venv için paketleri listele, denetle, kur ve güncelle
cli-init-about = pyenv için kabuk ortamını yapılandır
cli-gui-about = Pyenv Native grafik panelini başlat
cli-rehash-about = pyenv shim’lerini yeniden oluştur (tüm sürümlerde yürütülebilirleri kurar)
cli-shims-about = Mevcut pyenv shim’lerini listele
cli-prompt-about = Geçerli ortam için kısa bir komut istemi dizesi yazdır
cli-exec-about = Seçili Python sürümüyle bir yürütülebilir çalıştır
cli-completions-about = Komut tamamlama betiğini yazdır
cli-doctor-about = PATH, shim’ler ve kurulum önkoşullarını tanıla
cli-config-about = pyenv-native yapılandırmasını al, ayarla veya göster
cli-self-update-about = pyenv-native’i GitHub Releases’ten güncelle
cli-preflight-about = Platform bilgisi ve kurulum hazırlığı preflight
cli-environment-about = preflight için takma ad (aracılar ve kullanıcılar için OS/araç zinciri bilgileri)
cli-status-about = Kapsamlı ortam durumunu göster (sürümler, kaynaklar, venv’ler)
cli-root-about = Sürümlerin ve shim’lerin tutulduğu kök dizini göster
cli-which-about = Bir yürütülebilirin tam yolunu göster
cli-whence-about = Verilen yürütülebiliri içeren tüm Python sürümlerini listele
cli-version-file-about = Geçerli pyenv sürümünü ayarlayan dosyayı algıla
cli-version-file-read-about = Bir .python-version dosyasının içeriğini oku
cli-self-uninstall-about = pyenv-native’i sistemden kaldır
cli-help-about = Bir komutun yardımını göster
cli-commands-about = Kullanılabilir tüm pyenv komutlarını listele
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
cli-help-usage = Kullanım: pyenv <command> [<args>]
cli-help-useful = Yararlı pyenv komutları:
cli-help-concepts =
    TEMEL KAVRAMLAR:
      Shims: komutları yakalayıp geçerli sürüme yönlendiren hafif çalıştırılabilirler (`python` veya `pip`). pip paketlerinden sonra `pyenv rehash` çalıştırın.
      Versions: `pyenv install` ile kurulan ortamlar, `~/.pyenv/versions` altında.
      Managed envs: `~/.pyenv/venvs/<runtime>/<name>`. `pyenv venv create` ve `pyenv venv use` tercih edin.
      Discovery: `pyenv install --list 3.13` veya `pyenv available 3.13`.
      Selection: PYENV_VERSION, sonra `.python-version`, sonra genel sürüm dosyası.
    
    Ayrıntı için `pyenv help <command>`. Belgeler: https://github.com/imyourboyroy/pyenv-native
