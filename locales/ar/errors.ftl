# ./locales/ar/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: تعذر تحديد مجلد المنزل لـ PYENV_ROOT
error-invalid-directory = pyenv: تعذر تغيير مجلد العمل إلى `{ $path }`
error-invalid-version = pyenv: تم تجاهل الإصدار غير الصالح `{ $version }` في `{ $path }`
error-no-local-version = pyenv: لم يُضبط إصدار محلي لهذا المجلد
error-version-not-installed =
    pyenv: الإصدار `{ $version }` غير مثبت (عيّنه { $origin })
    تلميح: نفّذ `pyenv install { $version }` لتثبيته، أو `pyenv versions` لعرض الإصدارات المثبتة
error-unknown-config-key = pyenv: مفتاح إعداد غير معروف `{ $key }`
error-invalid-config-value = pyenv: قيمة غير صالحة `{ $value }` لمفتاح الإعداد `{ $key }`
error-version-already-installed = pyenv: الإصدار `{ $version }` مثبت بالفعل
error-unknown-version = pyenv: لا توجد إصدارات معروفة تطابق `{ $version }`
error-unsupported-install-target = pyenv: واجهة التثبيت الخلفية لا تدعم `{ $version }` على هذه المنصة
error-missing-install-version = pyenv: تتطلب عملية التثبيت وسيط إصدار واحدًا على الأقل
error-missing-python-build = pyenv: تعذر العثور على واجهة python-build الخلفية؛ عيّن install.python_build_path أو أضف python-build إلى PATH
error-checksum-mismatch = pyenv: عدم تطابق المجموع الاختباري لـ `{ $url }` ({ $algorithm }): المتوقع { $expected }، المستلم { $actual }
error-missing-checksum = pyenv: تعذر الحصول على مجموع اختباري من الناشر لـ `{ $source }`
error-io = { $message }
error-self-update-portable = pyenv: التحديث الذاتي يدعم فقط التثبيتات المحمولة المُشغَّلة من `{ $expected }`؛ الملف التنفيذي الحالي هو `{ $current }`
