# ./locales/ar/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = مدير إصدارات بايثون أصلي المنحى ومتعدد المنصات
cli-global-about = تعيين أو عرض إصدار بايثون العام
cli-local-about = تعيين أو عرض إصدار بايثون لمجلد محلي
cli-shell-about = تعيين أو عرض إصدار بايثون الخاص بالصدفة
cli-latest-about = طباعة أحدث إصدار مثبت أو معروف يطابق البادئة
cli-version-about = عرض إصدار بايثون الحالي ومصدره
cli-version-name-about = عرض إصدار بايثون الحالي
cli-version-origin-about = شرح كيفية تعيين إصدار بايثون الحالي
cli-prefix-about = عرض المسارات التي ثُبّتت فيها إصدارات بايثون المعطاة
cli-install-about = تثبيت إصدارات بايثون من المزوّدين الأصليين
cli-available-about = سرد إصدارات بايثون القابلة للتثبيت من المزوّدين الأصليين
cli-versions-about = سرد جميع إصدارات بايثون المتاحة لـ pyenv
cli-uninstall-about = إلغاء تثبيت إصدار بايثون محدد
cli-venv-about = إنشاء البيئات الافتراضية المُدارة وفحصها وتعيينها
cli-pip-about = سرد الحزم وفحصها وتثبيتها وتحديثها لوقت تشغيل أو venv
cli-init-about = تهيئة بيئة الصدفة لـ pyenv
cli-gui-about = تشغيل لوحة معلومات Pyenv Native الرسومية
cli-rehash-about = إعادة توليد shim الخاصة بـ pyenv (يثبّت التنفيذيات عبر جميع الإصدارات)
cli-shims-about = سرد shim الموجودة لـ pyenv
cli-prompt-about = طباعة سلسلة موجزة لموجّه البيئة الحالية
cli-exec-about = تشغيل برنامج تنفيذي بإصدار بايثون المحدد
cli-completions-about = طباعة سكربت إكمال الأوامر
cli-doctor-about = تشخيص PATH وshim ومتطلبات التثبيت
cli-config-about = قراءة إعدادات pyenv-native أو تعيينها أو عرضها
cli-self-update-about = تحديث pyenv-native من إصدارات GitHub
cli-preflight-about = معلومات المنصة وpreflight جاهزية التثبيت
cli-environment-about = اسم بديل لـ preflight (حقائق نظام التشغيل/سلسلة الأدوات للوكلاء والمستخدمين)
cli-status-about = عرض حالة البيئة الشاملة (الإصدارات والمصادر وvenv)
cli-root-about = عرض المجلد الجذر حيث تُحفظ الإصدارات وshim
cli-which-about = عرض المسار الكامل لتنفيذي
cli-whence-about = سرد جميع إصدارات بايثون التي تحتوي على التنفيذي المعطى
cli-version-file-about = اكتشاف الملف الذي يعيّن إصدار pyenv الحالي
cli-version-file-read-about = قراءة محتويات ملف .python-version
cli-self-uninstall-about = إلغاء تثبيت pyenv-native من النظام
cli-help-about = عرض مساعدة أمر
cli-commands-about = سرد جميع أوامر pyenv المتاحة
cli-hooks-about = سرد الخطافات التنفيذية لأمر معطى
cli-venv-list-about = سرد البيئات الافتراضية المُدارة
cli-venv-info-about = عرض تفاصيل بيئة افتراضية مُدارة
cli-venv-create-about = إنشاء بيئة افتراضية مُدارة تحت وقت تشغيل محدد
cli-venv-delete-about = إزالة بيئة افتراضية مُدارة
cli-venv-rename-about = إعادة تسمية بيئة افتراضية مُدارة
cli-venv-use-about = تعيين بيئة افتراضية مُدارة للمجلد الحالي أو عالميًا
cli-venv-upgrade-about = ترقية بيئة افتراضية مُدارة إلى وقت تشغيل أساسي جديد
cli-pip-list-about = سرد حزم pip المثبتة في بيئة هدف
cli-pip-outdated-about = سرد حزم pip القديمة في بيئة هدف
cli-pip-check-about = التحقق من متطلبات الحزم المعطوبة في بيئة هدف
cli-pip-precheck-about = فحص ملف متطلبات أو عنوان HTTPS فحصًا ساكنًا قبل التثبيت
cli-pip-analyze-about = فحص مصادر بايثون بحثًا عن واردات طرف ثالث مفقودة من الهدف
cli-pip-install-about = تثبيت الحزم من ملف requirements.txt أو عنوان HTTPS
cli-pip-update-about = تحديث حزم pip داخل بيئة هدف
cli-config-path-about = عرض مسار ملف الإعداد
cli-config-show-about = طباعة كل الإعداد الحالي
cli-config-get-about = طباعة قيمة مفتاح إعداد محدد
cli-config-set-about = تحديث مفتاح إعداد
cli-help-selection = التحديد
cli-help-provisioning = التوفير
cli-help-environment = البيئة
cli-help-interface = الواجهة
cli-help-diagnostics = التشخيص والإعداد
cli-help-maintenance = الصيانة
cli-help-support = الدعم
cli-help-usage = الاستخدام: pyenv <command> [<args>]
cli-help-useful = أوامر pyenv المفيدة:
cli-help-concepts =
    المفاهيم الأساسية:
      Shims: ملفات تنفيذ خفيفة (`python` أو `pip`) تعترض الأوامر وتوجّهها إلى الإصدار الحالي. شغّل `pyenv rehash` بعد تثبيت حزم pip.
      Versions: بيئات تُثبَّت عبر `pyenv install` تحت `~/.pyenv/versions`.
      Managed envs: `~/.pyenv/venvs/<runtime>/<name>`. يُفضَّل `pyenv venv create` و`pyenv venv use`.
      Discovery: `pyenv install --list 3.13` أو `pyenv available 3.13`.
      Selection: PYENV_VERSION ثم `.python-version` ثم ملف الإصدار العام.
    
    للتفاصيل شغّل `pyenv help <command>`. التوثيق: https://github.com/imyourboyroy/pyenv-native
