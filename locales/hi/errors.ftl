# ./locales/hi/errors.ftl
# Structured enchi-core errors. Placeholders are data, never translated tokens.

error-missing-home = enchi: ENCHI_ROOT के लिए होम निर्देशिका निर्धारित नहीं की जा सकी
error-invalid-directory = enchi: कार्य निर्देशिका `{ $path }` पर नहीं बदली जा सकी
error-invalid-version = enchi: अमान्य संस्करण `{ $version }` को `{ $path }` में अनदेखा किया गया
error-no-local-version = enchi: इस निर्देशिका के लिए कोई स्थानीय संस्करण कॉन्फ़िगर नहीं है
error-version-not-installed =
    enchi: संस्करण `{ $version }` इंस्टॉल नहीं है ({ $origin } द्वारा सेट)
    सुझाव: इंस्टॉल करने के लिए `enchi install { $version }` चलाएँ, या इंस्टॉल किए गए संस्करण देखने के लिए `enchi versions` चलाएँ
error-unknown-config-key = enchi: अज्ञात कॉन्फ़िग कुंजी `{ $key }`
error-invalid-config-value = enchi: कॉन्फ़िग कुंजी `{ $key }` के लिए अमान्य मान `{ $value }`
error-version-already-installed = enchi: संस्करण `{ $version }` पहले से इंस्टॉल है
error-unknown-version = enchi: `{ $version }` से मेल खाता कोई ज्ञात संस्करण नहीं
error-unsupported-install-target = enchi: इंस्टॉल बैकएंड इस प्लेटफ़ॉर्म पर `{ $version }` को सपोर्ट नहीं करता
error-missing-install-version = enchi: इंस्टॉल के लिए कम से कम एक संस्करण आर्ग्युमेंट ज़रूरी है
error-missing-python-build = enchi: python-build बैकएंड नहीं मिला; install.python_build_path सेट करें या python-build को PATH में जोड़ें
error-checksum-mismatch = enchi: `{ $url }` ({ $algorithm }) के लिए चेकसम बेमेल: अपेक्षित { $expected }, मिला { $actual }
error-missing-checksum = enchi: `{ $source }` के लिए प्रकाशक चेकसम प्राप्त नहीं हो सका
error-io = { $message }
error-self-update-portable = enchi: self-update केवल `{ $expected }` से चलाए गए पोर्टेबल इंस्टॉल के लिए उपलब्ध है; मौजूदा एक्ज़ीक्यूटेबल `{ $current }` है
error-self-update-managed = enchi: यह इंस्टॉल एक पैकेज मैनेजर ({ $source }) द्वारा प्रबंधित है; इसे इस कमांड से अपडेट करें: { $command }
