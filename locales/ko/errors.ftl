# ./locales/ko/errors.ftl
# Structured pyenv-core errors. Placeholders are data, never translated tokens.

error-missing-home = pyenv: PYENV_ROOT의 홈 디렉터리를 확인할 수 없습니다
error-invalid-directory = pyenv: 작업 디렉터리를 `{ $path }` 로 바꿀 수 없습니다
error-invalid-version = pyenv: 잘못된 버전 `{ $version }` 을 `{ $path }` 에서 무시했습니다
error-no-local-version = pyenv: 이 디렉터리에 로컬 버전이 구성되어 있지 않습니다
error-version-not-installed =
    pyenv: `{ $version }` 버전이 설치되어 있지 않습니다({ $origin }에서 설정됨)
    힌트: `pyenv install { $version }` 으로 설치하거나 `pyenv versions` 로 설치된 버전을 확인하세요
error-unknown-config-key = pyenv: 알 수 없는 구성 키 `{ $key }`
error-invalid-config-value = pyenv: 잘못된 값 `{ $value }`, 구성 키 `{ $key }`
error-version-already-installed = pyenv: `{ $version }` 버전이 이미 설치되어 있습니다
error-unknown-version = pyenv: `{ $version }` 와 일치하는 알려진 버전이 없습니다
error-unsupported-install-target = pyenv: 이 플랫폼의 설치 백엔드는 `{ $version }` 을 지원하지 않습니다
error-missing-install-version = pyenv: 설치 작업에는 버전 인수가 하나 이상 필요합니다
error-missing-python-build = pyenv: python-build 백엔드를 찾을 수 없습니다. install.python_build_path를 설정하거나 python-build를 PATH에 추가하세요
error-checksum-mismatch = pyenv: `{ $url }` 체크섬이 일치하지 않습니다({ $algorithm }): 기대값 { $expected }, 실제 { $actual }
error-missing-checksum = pyenv: `{ $source }` 의 게시자 체크섬을 가져올 수 없습니다
error-io = { $message }
error-self-update-portable = pyenv: 자체 업데이트는 `{ $expected }`에서 실행된 휴대용 설치만 지원합니다. 현재 실행 파일은 `{ $current }`입니다
