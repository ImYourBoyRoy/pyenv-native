# ./locales/tr/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = pyenv-native { $version } indiriliyor
install-extracting = Sürüm paketi çıkarılıyor
install-installing = { $path } konumuna kuruluyor
install-done = pyenv-native kuruldu. Yeni bir terminal açın, ardından `pyenv doctor` çalıştırın.
install-failed = Kurulum başarısız
install-lang-help = Kurucu iletileri için arayüz dili
install-summary-title = pyenv-native kurulum özeti
install-network-summary-title = pyenv-native ağ kurulumu özeti
install-summary-blurb = Bu, seçilen kök altında taşınabilir bir pyenv-native kurulumu oluşturur veya günceller.
install-summary-blurb-detail = pyenv’i, aracı dostu pyenv-mcp sunucusunu ve varsa GUI eşliğini kurar, bir kurulum günlüğü yazar ve temel sağlık denetimleri çalıştırır.
install-network-blurb = Bu, yayımlanmış bir pyenv-native paketini indirir, SHA-256 sağlama toplamını doğrular ve seçilen taşınabilir köke kurar.
install-profile-yes = Gelecekteki oturumların pyenv-native’i otomatik bulması için kabuk profiliniz güncellenecek.
install-profile-yes-pwsh = Gelecekteki oturumların pyenv-native’i otomatik bulması için PowerShell profiliniz güncellenecek.
install-profile-no = Kabuk profilinde değişiklik yapılmayacak.
install-profile-no-pwsh = PowerShell profilinde değişiklik yapılmayacak.
install-continue = Kuruluma devam edilsin mi? [y/N]:
install-need-yes = Etkileşimli kurulumlar onay gerektirir. Etkileşimsiz kullanım için --yes ile yeniden çalıştırın.
install-need-yes-pwsh = Etkileşimli kurulumlar onay gerektirir. Etkileşimsiz kullanım için -Yes ile yeniden çalıştırın.
install-cancelled = Kurulum iptal edildi.
install-installed-to = pyenv-native { $path } konumuna kuruldu
install-installed-command = Kurulan komut: { $path }
install-installed-mcp = Kurulan MCP sunucusu: { $path }
install-mcp-helper = MCP yapılandırma yardımcısı: { $command }
install-installed-gui = Kurulan GUI: { $path }
install-log-file = Günlük dosyası: { $path }
