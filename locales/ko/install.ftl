# ./locales/ko/install.ftl
# Installer-facing copy. Command names and paths stay English.

install-downloading = pyenv-native { $version } 다운로드 중
install-extracting = 릴리스 번들 압축 해제 중
install-installing = { $path } 에 설치 중
install-done = pyenv-native가 설치되었습니다. 새 터미널을 연 다음 `pyenv doctor`를 실행하세요.
install-failed = 설치 실패
install-lang-help = 설치 프로그램 메시지의 인터페이스 언어
install-summary-title = pyenv-native 설치 요약
install-network-summary-title = pyenv-native 네트워크 설치 요약
install-summary-blurb = 선택한 루트 아래에 이동 가능한 pyenv-native 설치를 만들거나 업데이트합니다.
install-summary-blurb-detail = pyenv와 에이전트용 pyenv-mcp 서버, 사용 가능한 경우 GUI 동반 앱을 설치하고, 설치 로그를 쓰며, 기본 상태 검사를 실행합니다.
install-network-blurb = 게시된 pyenv-native 번들을 다운로드하고 SHA-256 체크섬을 검증한 뒤 선택한 이동식 루트에 설치합니다.
install-profile-yes = 이후 세션이 pyenv-native를 자동으로 찾도록 셸 프로필이 업데이트됩니다.
install-profile-yes-pwsh = 이후 세션이 pyenv-native를 자동으로 찾도록 PowerShell 프로필이 업데이트됩니다.
install-profile-no = 셸 프로필은 변경되지 않습니다.
install-profile-no-pwsh = PowerShell 프로필은 변경되지 않습니다.
install-continue = 설치를 계속할까요? [y/N]:
install-need-yes = 대화형 설치에는 확인이 필요합니다. 비대화형으로 쓰려면 --yes를 붙여 다시 실행하세요.
install-need-yes-pwsh = 대화형 설치에는 확인이 필요합니다. 비대화형으로 쓰려면 -Yes를 붙여 다시 실행하세요.
install-cancelled = 설치가 취소되었습니다.
install-installed-to = pyenv-native를 { $path }에 설치했습니다
install-installed-command = 설치한 명령: { $path }
install-installed-mcp = 설치한 MCP 서버: { $path }
install-mcp-helper = MCP 구성 도우미: { $command }
install-installed-gui = 설치한 GUI: { $path }
install-log-file = 로그 파일: { $path }
