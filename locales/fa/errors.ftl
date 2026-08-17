# ./locales/fa/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: نمی‌توان فهرست خانگی را برای PYENV_ROOT تعیین کرد
error-invalid-directory = pyenv: نمی‌توان پوشهٔ کاری را به `{ $path }` تغییر داد
error-invalid-version = pyenv: نسخهٔ نامعتبر `{ $version }` در `{ $path }` نادیده گرفته شد
error-no-local-version = pyenv: برای این پوشه نسخهٔ محلی پیکربندی نشده است
error-version-not-installed =
    pyenv: نسخهٔ `{ $version }` نصب نشده است (تنظیم‌شده توسط { $origin })
    راهنما: برای نصب `pyenv install { $version }` را اجرا کنید، یا برای دیدن نسخه‌های نصب‌شده `pyenv versions` را اجرا کنید
error-unknown-config-key = pyenv: کلید پیکربندی ناشناخته `{ $key }`
error-invalid-config-value = pyenv: مقدار نامعتبر `{ $value }` برای کلید پیکربندی `{ $key }`
error-version-already-installed = pyenv: نسخهٔ `{ $version }` از قبل نصب شده است
error-unknown-version = pyenv: هیچ نسخهٔ شناخته‌شده‌ای با `{ $version }` مطابقت ندارد
error-unsupported-install-target = pyenv: پشتیبان نصب از `{ $version }` روی این سکو پشتیبانی نمی‌کند
error-missing-install-version = pyenv: عملیات نصب دست‌کم به یک آرگومان نسخه نیاز دارد
error-missing-python-build = pyenv: پشتیبان python-build یافت نشد؛ install.python_build_path را تنظیم کنید یا python-build را به PATH اضافه کنید
error-checksum-mismatch = pyenv: عدم تطابق جمع‌آزمایی برای `{ $url }` ({ $algorithm }): انتظار { $expected }، دریافت { $actual }
error-missing-checksum = pyenv: دریافت جمع‌آزمایی ناشر برای `{ $source }` ممکن نیست
error-io = { $message }
error-self-update-portable = pyenv: به‌روزرسانی خودکار فقط نصب‌های قابل‌حمل اجراشده از `{ $expected }` را پشتیبانی می‌کند؛ فایل اجرایی فعلی `{ $current }` است
