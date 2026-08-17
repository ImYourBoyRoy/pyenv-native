# ./locales/ko/cli.ftl
# Human-facing CLI about/help. Subcommand and flag *names* stay English.

cli-about = 네이티브 우선의 크로스 플랫폼 Python 버전 관리자
cli-global-about = 전역 Python 버전을 설정하거나 표시합니다
cli-local-about = 현재 디렉터리의 Python 버전을 설정하거나 표시합니다
cli-shell-about = 셸 전용 Python 버전을 설정하거나 표시합니다
cli-latest-about = 접두사와 일치하는 설치된 버전 또는 알려진 최신 버전을 출력합니다
cli-version-about = 현재 Python 버전과 출처를 표시합니다
cli-version-name-about = 현재 Python 버전을 표시합니다
cli-version-origin-about = 현재 Python 버전이 어떻게 설정되었는지 설명합니다
cli-prefix-about = 지정한 Python 버전의 설치 경로를 표시합니다
cli-install-about = 네이티브 제공자에서 Python 버전을 설치합니다
cli-available-about = 네이티브 제공자에서 설치 가능한 Python 버전을 나열합니다
cli-versions-about = pyenv에서 사용할 수 있는 모든 Python 버전을 나열합니다
cli-uninstall-about = 지정한 Python 버전을 제거합니다
cli-venv-about = 관리형 가상 환경을 만들고 확인하고 지정합니다
cli-pip-about = 런타임 또는 가상 환경의 패키지를 나열, 검사, 설치, 업데이트합니다
cli-init-about = pyenv용 셸 환경을 구성합니다
cli-gui-about = Pyenv Native GUI 대시보드를 실행합니다
cli-rehash-about = pyenv shim을 다시 생성합니다(모든 버전의 실행 파일을 등록)
cli-shims-about = 기존 pyenv shim을 나열합니다
cli-prompt-about = 현재 환경용 짧은 프롬프트 문자열을 출력합니다
cli-exec-about = 선택한 Python 버전으로 실행 파일을 실행합니다
cli-completions-about = 명령 완성 스크립트를 출력합니다
cli-doctor-about = PATH, shim, 설치 선행 조건을 진단합니다
cli-config-about = pyenv-native 구성을 가져오거나 설정하거나 표시합니다
cli-self-update-about = GitHub Releases에서 pyenv-native를 업데이트합니다
cli-preflight-about = 플랫폼 정보 및 설치 준비 preflight
cli-environment-about = preflight의 별칭(에이전트와 사용자를 위한 OS/툴체인 정보)
cli-status-about = 포괄적인 환경 상태(버전, 출처, venv)를 표시합니다
cli-root-about = 버전과 shim이 보관되는 루트 디렉터리를 표시합니다
cli-which-about = 실행 파일의 전체 경로를 표시합니다
cli-whence-about = 지정한 실행 파일을 포함하는 모든 Python 버전을 나열합니다
cli-version-file-about = 현재 pyenv 버전을 설정하는 파일을 감지합니다
cli-version-file-read-about = .python-version 파일 내용을 읽습니다
cli-self-uninstall-about = 시스템에서 pyenv-native를 제거합니다
cli-help-about = 명령 도움말을 표시합니다
cli-commands-about = 사용 가능한 모든 pyenv 명령을 나열합니다
cli-hooks-about = 지정한 명령의 실행 가능한 훅을 나열합니다
cli-venv-list-about = 관리형 가상 환경을 나열합니다
cli-venv-info-about = 관리형 가상 환경의 세부 정보를 표시합니다
cli-venv-create-about = 지정한 런타임 아래에 관리형 가상 환경을 만듭니다
cli-venv-delete-about = 관리형 가상 환경을 제거합니다
cli-venv-rename-about = 관리형 가상 환경의 이름을 바꿉니다
cli-venv-use-about = 관리형 가상 환경을 현재 디렉터리 또는 전역에 할당합니다
cli-venv-upgrade-about = 관리형 가상 환경을 새 기반 런타임으로 업그레이드합니다
cli-pip-list-about = 대상 환경에 설치된 pip 패키지를 나열합니다
cli-pip-outdated-about = 대상 환경의 오래된 pip 패키지를 나열합니다
cli-pip-check-about = 대상 환경의 깨진 패키지 요구 사항을 검사합니다
cli-pip-precheck-about = 설치 전에 requirements 파일 또는 HTTPS URL을 정적으로 사전 검사합니다
cli-pip-analyze-about = Python 소스를 검사하여 대상에 없는 서드파티 import를 찾습니다
cli-pip-install-about = requirements.txt 파일 또는 HTTPS URL에서 패키지를 설치합니다
cli-pip-update-about = 대상 환경의 pip 패키지를 업데이트합니다
cli-config-path-about = 구성 파일 경로를 표시합니다
cli-config-show-about = 현재 구성을 모두 출력합니다
cli-config-get-about = 특정 구성 키의 값을 출력합니다
cli-config-set-about = 구성 키를 업데이트합니다
cli-help-selection = 버전 선택
cli-help-provisioning = 프로비저닝
cli-help-environment = 환경
cli-help-interface = 인터페이스
cli-help-diagnostics = 진단 및 구성
cli-help-maintenance = 유지관리
cli-help-support = 지원
cli-help-usage = 사용법: pyenv <command> [<args>]
cli-help-useful = 유용한 pyenv 명령:
cli-help-concepts =
    핵심 개념:
      Shims: `python` / `pip`을 가로채 현재 버전으로 보내는 실행 파일. pip 패키지 설치 후 `pyenv rehash`를 실행하세요.
      Versions: `pyenv install`로 설치하며 `~/.pyenv/versions`에 있습니다.
      Managed envs: `~/.pyenv/venvs/<runtime>/<name>`. `pyenv venv create`와 `pyenv venv use`를 권장합니다.
      Discovery: `pyenv install --list 3.13` 또는 `pyenv available 3.13`.
      Selection: PYENV_VERSION, 그다음 `.python-version`, 그다음 전역 버전 파일.
    
    자세한 도움말은 `pyenv help <command>`. 문서: https://github.com/imyourboyroy/pyenv-native
