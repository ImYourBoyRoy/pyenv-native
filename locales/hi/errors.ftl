# ./locales/hi/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: PYENV_ROOT के लिए होम निर्देशिका निर्धारित नहीं की जा सकी
error-invalid-directory = enchi: कार्य निर्देशिका `{ $path }` पर नहीं बदली जा सकी
error-invalid-version = enchi: अमान्य संस्करण `{ $version }` को `{ $path }` में अनदेखा किया गया
error-no-local-version = enchi: इस निर्देशिका के लिए कोई स्थानीय संस्करण कॉन्फ़िगर नहीं है
error-version-not-installed =
    enchi: संस्करण `{ $version }` स्थापित नहीं है ({ $origin } द्वारा सेट)
    संकेत: स्थापित करने के लिए `enchi install { $version }` चलाएँ, या स्थापित संस्करण देखने के लिए `enchi versions` चलाएँ
error-unknown-config-key = enchi: अज्ञात कॉन्फ़िग कुंजी `{ $key }`
error-invalid-config-value = enchi: अमान्य मान `{ $value }` कॉन्फ़िग कुंजी `{ $key }` के लिए
error-version-already-installed = enchi: संस्करण `{ $version }` पहले से स्थापित है
error-unknown-version = enchi: `{ $version }` से मेल खाता कोई ज्ञात संस्करण नहीं
error-unsupported-install-target = enchi: स्थापना बैकएंड इस प्लेटफ़ॉर्म पर `{ $version }` का समर्थन नहीं करता
error-missing-install-version = enchi: स्थापना कार्रवाई के लिए कम से कम एक संस्करण तर्क चाहिए
error-missing-python-build = enchi: python-build बैकएंड नहीं मिला; install.python_build_path सेट करें या python-build को PATH में जोड़ें
error-checksum-mismatch = enchi: `{ $url }` ({ $algorithm }) के लिए चेकसम बेमेल: अपेक्षित { $expected }, मिला { $actual }
error-missing-checksum = enchi: `{ $source }` के लिए प्रकाशक चेकसम प्राप्त नहीं हो सका
error-io = { $message }
error-self-update-portable = enchi: स्व-अपडेट केवल `{ $expected }` से चलाई गई पोर्टेबल स्थापनाओं का समर्थन करता है; वर्तमान निष्पादन योग्य `{ $current }` है
error-self-update-managed = enchi: यह स्थापना एक पैकेज प्रबंधक ({ $source }) द्वारा प्रबंधित है; इसे इस कमांड से अपडेट करें: { $command }
