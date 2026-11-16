# ./locales/ar/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: تعذر تحديد المجلد الرئيسي لـ ENCHI_ROOT
error-invalid-directory = enchi: تعذر تغيير مجلد العمل إلى `{ $path }`
error-invalid-version = enchi: تم تجاهل الإصدار غير الصالح `{ $version }` في `{ $path }`
error-no-local-version = enchi: لم يُضبط إصدار محلي لهذا المجلد
error-version-not-installed =
    enchi: الإصدار `{ $version }` غير مثبت (محدَّد بواسطة { $origin })
    تلميح: نفّذ `enchi install { $version }` لتثبيته، أو `enchi versions` لعرض الإصدارات المثبتة
error-unknown-config-key = enchi: مفتاح إعداد غير معروف `{ $key }`
error-invalid-config-value = enchi: قيمة غير صالحة `{ $value }` لمفتاح الإعداد `{ $key }`
error-version-already-installed = enchi: الإصدار `{ $version }` مثبت بالفعل
error-unknown-version = enchi: لا توجد إصدارات معروفة تطابق `{ $version }`
error-unsupported-install-target = enchi: واجهة التثبيت الخلفية لا تدعم `{ $version }` على هذه المنصة
error-missing-install-version = enchi: تتطلب عملية التثبيت وسيط إصدار واحدًا على الأقل
error-missing-python-build = enchi: تعذر العثور على واجهة python-build الخلفية؛ عيّن install.python_build_path أو أضف python-build إلى PATH
error-checksum-mismatch = enchi: عدم تطابق المجموع الاختباري لـ `{ $url }` ({ $algorithm }): المتوقع { $expected }، الفعلي { $actual }
error-missing-checksum = enchi: تعذر الحصول على مجموع اختباري من الناشر لـ `{ $source }`
error-io = { $message }
error-self-update-portable = enchi: التحديث الذاتي يدعم فقط التثبيتات المحمولة المُشغَّلة من `{ $expected }`؛ الملف التنفيذي الحالي هو `{ $current }`
error-self-update-managed = enchi: يُدار هذا التثبيت بواسطة مدير حزم ({ $source })؛ حدّثه بالأمر: { $command }
