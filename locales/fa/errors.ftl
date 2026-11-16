# ./locales/fa/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: نمی‌توان پوشهٔ خانه را برای ENCHI_ROOT تعیین کرد
error-invalid-directory = enchi: نمی‌توان پوشهٔ کاری را به `{ $path }` تغییر داد
error-invalid-version = enchi: نسخهٔ نامعتبر `{ $version }` در `{ $path }` نادیده گرفته شد
error-no-local-version = enchi: برای این پوشه نسخهٔ محلی پیکربندی نشده است
error-version-not-installed =
    enchi: نسخهٔ `{ $version }` نصب نشده است (تنظیم‌شده توسط { $origin })
    راهنما: برای نصب `enchi install { $version }` را اجرا کنید، یا برای دیدن نسخه‌های نصب‌شده `enchi versions` را اجرا کنید
error-unknown-config-key = enchi: کلید پیکربندی ناشناخته `{ $key }`
error-invalid-config-value = enchi: مقدار نامعتبر `{ $value }` برای کلید پیکربندی `{ $key }`
error-version-already-installed = enchi: نسخهٔ `{ $version }` از قبل نصب شده است
error-unknown-version = enchi: هیچ نسخهٔ شناخته‌شده‌ای با `{ $version }` مطابقت ندارد
error-unsupported-install-target = enchi: بک‌اند نصب از `{ $version }` روی این سکو پشتیبانی نمی‌کند
error-missing-install-version = enchi: عملیات نصب دست‌کم به یک آرگومان نسخه نیاز دارد
error-missing-python-build = enchi: بک‌اند python-build یافت نشد؛ install.python_build_path را تنظیم کنید یا python-build را به PATH اضافه کنید
error-checksum-mismatch = enchi: عدم تطابق چک‌سام برای `{ $url }` ({ $algorithm }): مقدار مورد انتظار { $expected } بود، اما { $actual } دریافت شد
error-missing-checksum = enchi: دریافت چک‌سام ناشر برای `{ $source }` ممکن نیست
error-io = { $message }
error-self-update-portable = enchi: خودبه‌روزرسانی فقط از نصب‌های قابل‌حمل پشتیبانی می‌کند که از `{ $expected }` اجرا شده باشند؛ فایل اجرایی فعلی `{ $current }` است
error-self-update-managed = enchi: این نصب توسط یک مدیر بسته ({ $source }) مدیریت می‌شود؛ با این دستور به‌روزرسانی کنید: { $command }
