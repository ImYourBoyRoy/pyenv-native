# ./locales/ar/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = جارٍ تنزيل pyenv-native { $version }
install-extracting = جارٍ استخراج حزمة الإصدار
install-installing = جارٍ التثبيت في { $path }
install-done = ثُبّت pyenv-native. افتح طرفية جديدة، ثم نفّذ `pyenv doctor`.
install-failed = فشل التثبيت
install-lang-help = لغة الواجهة لرسائل المثبّت
install-summary-title = ملخص تثبيت pyenv-native
install-network-summary-title = ملخص التثبيت الشبكي لـ pyenv-native
install-summary-blurb = سيُنشئ هذا أو يحدّث تثبيت pyenv-native المحمول تحت الجذر المحدد.
install-summary-blurb-detail = يثبّت pyenv وخادم pyenv-mcp الملائم للوكلاء ورفيق GUI عند توفره، ويكتب سجل تثبيت، ويجري فحوصات سلامة أساسية.
install-network-blurb = سينزّل هذا حزمة pyenv-native منشورة، ويتحقق من مجموع SHA-256، ويثبّتها في الجذر المحمول المحدد.
install-profile-yes = سيُحدَّث ملف تعريف الصَدفة حتى تجد الجلسات القادمة pyenv-native تلقائيًا.
install-profile-yes-pwsh = سيُحدَّث ملف تعريف PowerShell حتى تجد الجلسات القادمة pyenv-native تلقائيًا.
install-profile-no = لن تُجرى تغييرات على ملف تعريف الصَدفة.
install-profile-no-pwsh = لن تُجرى تغييرات على ملف تعريف PowerShell.
install-continue = متابعة التثبيت؟ [y/N]:
install-need-yes = التثبيت التفاعلي يتطلب تأكيدًا. أعد التشغيل مع --yes للاستخدام غير التفاعلي.
install-need-yes-pwsh = التثبيت التفاعلي يتطلب تأكيدًا. أعد التشغيل مع -Yes للاستخدام غير التفاعلي.
install-cancelled = أُلغي التثبيت.
install-installed-to = ثُبّت pyenv-native في { $path }
install-installed-command = الأمر المثبَّت: { $path }
install-installed-mcp = خادم MCP المثبَّت: { $path }
install-mcp-helper = مساعد إعداد MCP: { $command }
install-installed-gui = GUI المثبَّت: { $path }
install-log-file = ملف السجل: { $path }
