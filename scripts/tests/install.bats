#!/usr/bin/env bats

load "helpers.bash"

fixture_sums="$ITZTERM_TEST_REPO_ROOT/scripts/tests/fixtures/SHA256SUMS.txt"

setup() {
  make_tmp
  export HOME="$ITZTERM_TEST_TMP_DIR/home"
  export ITZTERM_TEST_LOG="$ITZTERM_TEST_TMP_DIR/commands.log"
  export ITZTERM_INSTALL_APPLICATIONS_DIR="$ITZTERM_TEST_TMP_DIR/Applications"
  export ITZTERM_INSTALL_BIN_DIR="$HOME/.local/bin"
  export ITZTERM_INSTALL_DESKTOP_DIR="$HOME/.local/share/applications"
  mkdir -p "$HOME" "$ITZTERM_INSTALL_APPLICATIONS_DIR"
  : >"$ITZTERM_TEST_LOG"
}

teardown() {
  unset ITZTERM_INSTALL_YES
  unset ITZTERM_INSTALL_APPLICATIONS_DIR
  unset ITZTERM_INSTALL_BIN_DIR
  unset ITZTERM_INSTALL_DESKTOP_DIR
  cleanup_tmp
}

stub_uname() {
  local os="$1"
  local arch="$2"

  stub_cmd uname "
case \"\$1\" in
  -s) printf '%s\\n' '$os' ;;
  -m) printf '%s\\n' '$arch' ;;
  *) exit 1 ;;
esac
"
}

stub_common_tools() {
  stub_cmd curl "printf 'curl %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\""
  stub_cmd mktemp "if [[ \"\${1:-}\" == '-d' ]]; then dir=\"\$ITZTERM_TEST_TMP_DIR/mktemp.\$RANDOM\"; mkdir -p \"\$dir\"; printf '%s\\n' \"\$dir\"; else file=\"\$ITZTERM_TEST_TMP_DIR/mktemp.\$RANDOM\"; : >\"\$file\"; printf '%s\\n' \"\$file\"; fi"
}

stub_curl_release_flow() {
  local payload="$1"

  stub_cmd curl "
printf 'curl %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"
args=\" \$* \"
if [[ \"\$args\" == *'/releases/latest'* ]]; then
  printf '%s\\n' 'https://github.com/latiefahmad/itzTerm/releases/tag/v1.2.3'
  exit 0
fi
out=''
prev=''
for arg in \"\$@\"; do
  if [[ \"\$prev\" == '-o' ]]; then
    out=\"\$arg\"
    break
  fi
  prev=\"\$arg\"
done
if [[ \"\$args\" == *'SHA256SUMS.txt'* ]]; then
  /bin/cp '$fixture_sums' \"\$out\"
else
  printf '%s' '$payload' >\"\$out\"
fi
"
}

stub_macos_install_tools() {
  stub_cmd hdiutil "
printf 'hdiutil %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"
if [[ \"\$1\" == 'attach' ]]; then
  mountpoint=''
  prev=''
  for arg in \"\$@\"; do
    if [[ \"\$prev\" == '-mountpoint' ]]; then
      mountpoint=\"\$arg\"
      break
    fi
    prev=\"\$arg\"
  done
  mkdir -p \"\$mountpoint/itzTerm.app\"
fi
"
  stub_cmd cp "
printf 'cp %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"
if [[ \"\${ITZTERM_TEST_CP_FAIL_ONCE:-}\" == '1' && ! -f \"\$ITZTERM_TEST_TMP_DIR/cp_failed\" ]]; then
  : >\"\$ITZTERM_TEST_TMP_DIR/cp_failed\"
  exit 1
fi
exit 0
"
  stub_cmd sudo "printf 'sudo %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\""
  stub_cmd xattr "printf 'xattr %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\""
}

stub_logging_rm() {
  stub_cmd rm "
printf 'rm %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"
/bin/rm \"\$@\"
"
}

stub_linux_install_tools() {
  stub_cmd cp "
printf 'cp %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"
command cp \"\$@\"
"
  stub_cmd chmod "
printf 'chmod %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"
command chmod \"\$@\"
"
}

@test "detect_os and detect_arch normalize supported Darwin and Linux pairs" {
  stub_uname Darwin arm64
  load_install

  run detect_os
  [ "$status" -eq 0 ]
  [ "$output" = "darwin" ]

  run detect_arch
  [ "$status" -eq 0 ]
  [ "$output" = "aarch64" ]

  stub_uname Linux x86_64
  run detect_os
  [ "$status" -eq 0 ]
  [ "$output" = "linux" ]

  run detect_arch
  [ "$status" -eq 0 ]
  [ "$output" = "x86_64" ]
}

@test "detect_os and detect_arch reject unsupported platforms" {
  stub_uname FreeBSD sparc
  load_install

  run detect_os
  [ "$status" -ne 0 ]
  [[ "$output" == *"Unsupported operating system"* ]]

  stub_uname Linux sparc
  run detect_arch
  [ "$status" -ne 0 ]
  [[ "$output" == *"Unsupported architecture"* ]]
}

@test "main rejects Windows shells before curl download" {
  stub_uname MINGW64_NT-10.0 x86_64
  stub_common_tools
  load_install

  run main

  [ "$status" -ne 0 ]
  [[ "$output" == *".exe or .msi"* ]]
  [[ "$output" == *"https://github.com/latiefahmad/itzTerm/releases"* ]]
  ! grep -q "^curl " "$ITZTERM_TEST_LOG"
}

@test "require_tools fails early and names missing tools before download" {
  stub_uname Darwin arm64
  load_install
  # Stubs are `#!/usr/bin/env bash` scripts, so bash must stay resolvable
  # once PATH is narrowed to the stub dir; nothing else from the system leaks in.
  printf '#!/bin/sh\nexec "%s" "$@"\n' "$(command -v bash)" >"$ITZTERM_TEST_STUB_BIN/bash"
  chmod +x "$ITZTERM_TEST_STUB_BIN/bash"
  local saved_path="$PATH"
  PATH="$ITZTERM_TEST_STUB_BIN"

  run main

  PATH="$saved_path"

  [ "$status" -ne 0 ]
  [[ "$output" == *"Missing required tools"* ]]
  [[ "$output" == *"curl"* ]]
  [[ "$output" == *"mktemp"* ]]
  ! grep -q "curl" "$ITZTERM_TEST_LOG"
}

@test "resolve_version parses latest redirect without api.github.com" {
  stub_cmd curl "
printf '%s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"
printf '%s\\n' 'https://github.com/latiefahmad/itzTerm/releases/tag/v1.2.3'
"
  load_install

  run resolve_version

  [ "$status" -eq 0 ]
  [ "$output" = "v1.2.3" ]
  ! grep -q "api.github.com" "$ITZTERM_TEST_LOG"
  grep -q "https://github.com/latiefahmad/itzTerm/releases/latest" "$ITZTERM_TEST_LOG"
}

@test "verify_sha256 succeeds for matching fixture" {
  load_install
  local payload="$ITZTERM_TEST_TMP_DIR/itzTerm.Manager_1.2.3_amd64.AppImage"
  printf '%s' "linux appimage payload" >"$payload"

  run verify_sha256 "$payload" "itzTerm.Manager_1.2.3_amd64.AppImage" "$fixture_sums"

  [ "$status" -eq 0 ]
}

@test "verify_sha256 fails missing and mismatched entries without creating install targets" {
  load_install
  local payload="$ITZTERM_TEST_TMP_DIR/payload"
  local target="$ITZTERM_INSTALL_BIN_DIR/itzterm-manager"
  printf '%s' "tampered payload" >"$payload"

  run verify_sha256 "$payload" "itzTerm.Manager_9.9.9_amd64.AppImage" "$fixture_sums"
  [ "$status" -ne 0 ]
  [[ "$output" == *"Integrity check failed, nothing was installed"* ]]
  [ ! -e "$target" ]

  run verify_sha256 "$payload" "itzTerm.Manager_1.2.3_amd64.AppImage" "$fixture_sums"
  [ "$status" -ne 0 ]
  [[ "$output" == *"Integrity check failed, nothing was installed"* ]]
  [ ! -e "$target" ]
}

@test "confirm_install aborts non-tty without env var" {
  load_install
  unset ITZTERM_INSTALL_YES

  run confirm_install "Install itzTerm?"

  [ "$status" -ne 0 ]
  [[ "$output" == *"ITZTERM_INSTALL_YES=1"* ]]
}

@test "confirm_install proceeds with env var and echoes prompt" {
  load_install
  export ITZTERM_INSTALL_YES=1

  run confirm_install "Install itzTerm v1.2.3?"

  [ "$status" -eq 0 ]
  [[ "$output" == *"ITZTERM_INSTALL_YES=1"* ]]
  [[ "$output" == *"Install itzTerm v1.2.3?"* ]]
}

@test "install_macos downloads correct DMG, verifies before copy fallback, detaches, then xattrs" {
  stub_common_tools
  stub_curl_release_flow "mac dmg payload"
  stub_macos_install_tools
  export ITZTERM_TEST_CP_FAIL_ONCE=1
  load_install

  run install_macos "v1.2.3" "aarch64" "$fixture_sums"

  [ "$status" -eq 0 ]
  grep -q "releases/download/v1.2.3/itzTerm.Manager_1.2.3_aarch64.dmg" "$ITZTERM_TEST_LOG"
  ! grep -q "itzTerm.Manager_v1.2.3_aarch64.dmg" "$ITZTERM_TEST_LOG"
  local verify_line
  local attach_line
  local cp_line
  local sudo_line
  local detach_line
  local xattr_line
  verify_line="$(grep -n "curl .*itzTerm.Manager_1.2.3_aarch64.dmg" "$ITZTERM_TEST_LOG" | head -n1 | cut -d: -f1)"
  attach_line="$(grep -n "hdiutil attach" "$ITZTERM_TEST_LOG" | cut -d: -f1)"
  cp_line="$(grep -n "^cp " "$ITZTERM_TEST_LOG" | head -n1 | cut -d: -f1)"
  sudo_line="$(grep -n "^sudo cp" "$ITZTERM_TEST_LOG" | cut -d: -f1)"
  detach_line="$(grep -n "hdiutil detach" "$ITZTERM_TEST_LOG" | cut -d: -f1)"
  xattr_line="$(grep -n "^xattr " "$ITZTERM_TEST_LOG" | cut -d: -f1)"
  [ "$verify_line" -lt "$attach_line" ]
  [ "$attach_line" -lt "$cp_line" ]
  [ "$cp_line" -lt "$sudo_line" ]
  [ "$sudo_line" -lt "$detach_line" ]
  [ "$detach_line" -lt "$xattr_line" ]
}

@test "install_macos detaches mounted DMG on copy failure" {
  stub_common_tools
  stub_curl_release_flow "mac dmg payload"
  stub_macos_install_tools
  stub_cmd cp "printf 'cp %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"; exit 1"
  stub_cmd sudo "printf 'sudo %s\\n' \"\$*\" >>\"\$ITZTERM_TEST_LOG\"; exit 1"
  load_install

  run install_macos "v1.2.3" "x86_64" "$fixture_sums"

  [ "$status" -ne 0 ]
  grep -q "itzTerm.Manager_1.2.3_x64.dmg" "$ITZTERM_TEST_LOG"
  ! grep -q "itzTerm.Manager_v1.2.3_x64.dmg" "$ITZTERM_TEST_LOG"
  grep -q "hdiutil detach" "$ITZTERM_TEST_LOG"
}

@test "install_macos replaces existing app bundle before copying" {
  stub_common_tools
  stub_curl_release_flow "mac dmg payload"
  stub_macos_install_tools
  stub_logging_rm
  mkdir -p "$ITZTERM_INSTALL_APPLICATIONS_DIR/itzTerm.app"
  load_install

  run install_macos "v1.2.3" "aarch64" "$fixture_sums"

  [ "$status" -eq 0 ]
  local rm_line
  local cp_line
  rm_line="$(grep -n "^rm -rf .*itzTerm.app" "$ITZTERM_TEST_LOG" | head -n1 | cut -d: -f1)"
  cp_line="$(grep -n "^cp -R " "$ITZTERM_TEST_LOG" | head -n1 | cut -d: -f1)"
  [ -n "$rm_line" ]
  [ -n "$cp_line" ]
  [ "$rm_line" -lt "$cp_line" ]
}

@test "install_linux installs AppImage, desktop entry, chmod, and PATH warning" {
  stub_common_tools
  stub_curl_release_flow "linux appimage payload"
  load_install

  run install_linux "v1.2.3" "x86_64" "$fixture_sums"

  [ "$status" -eq 0 ]
  grep -q "releases/download/v1.2.3/itzTerm.Manager_1.2.3_amd64.AppImage" "$ITZTERM_TEST_LOG"
  ! grep -q "itzTerm.Manager_v1.2.3_amd64.AppImage" "$ITZTERM_TEST_LOG"
  [ -x "$ITZTERM_INSTALL_BIN_DIR/itzterm-manager" ]
  [ -f "$ITZTERM_INSTALL_DESKTOP_DIR/itzterm-manager.desktop" ]
  grep -q "Exec=$ITZTERM_INSTALL_BIN_DIR/itzterm-manager" "$ITZTERM_INSTALL_DESKTOP_DIR/itzterm-manager.desktop"
  [[ "$output" == *"not in PATH"* ]]
}

@test "main happy macOS path reaches hdiutil once" {
  stub_uname Darwin arm64
  stub_common_tools
  stub_curl_release_flow "mac dmg payload"
  stub_macos_install_tools
  export ITZTERM_INSTALL_YES=1
  load_install

  run main

  [ "$status" -eq 0 ]
  [[ "$output" == *"Install itzTerm v1.2.3 (darwin-aarch64) to $ITZTERM_INSTALL_APPLICATIONS_DIR?"* ]]
  [ "$(grep -c "hdiutil attach" "$ITZTERM_TEST_LOG")" -eq 1 ]
}

@test "main happy Linux prompt includes platform tuple and installs AppImage" {
  stub_uname Linux x86_64
  stub_common_tools
  stub_curl_release_flow "linux appimage payload"
  export ITZTERM_INSTALL_YES=1
  load_install

  run main

  [ "$status" -eq 0 ]
  [[ "$output" == *"Install itzTerm v1.2.3 (linux-x86_64) to $HOME/.local/bin?"* ]]
  [ -x "$ITZTERM_INSTALL_BIN_DIR/itzterm-manager" ]
  grep -q "itzTerm.Manager_1.2.3_amd64.AppImage" "$ITZTERM_TEST_LOG"
  ! grep -q "itzTerm.Manager_v1.2.3_amd64.AppImage" "$ITZTERM_TEST_LOG"
}

@test "main non-tty abort runs no install commands" {
  stub_uname Darwin arm64
  stub_common_tools
  stub_curl_release_flow "mac dmg payload"
  stub_macos_install_tools
  unset ITZTERM_INSTALL_YES
  load_install

  run main

  [ "$status" -ne 0 ]
  ! grep -q "hdiutil" "$ITZTERM_TEST_LOG"
  ! grep -q "^cp " "$ITZTERM_TEST_LOG"
  ! grep -q "xattr" "$ITZTERM_TEST_LOG"
}

@test "main tampered download aborts before hdiutil cp or xattr" {
  stub_uname Darwin arm64
  stub_common_tools
  stub_curl_release_flow "tampered payload"
  stub_macos_install_tools
  export ITZTERM_INSTALL_YES=1
  load_install

  run main

  [ "$status" -ne 0 ]
  [[ "$output" == *"Integrity check failed, nothing was installed"* ]]
  ! grep -q "hdiutil" "$ITZTERM_TEST_LOG"
  ! grep -q "^cp " "$ITZTERM_TEST_LOG"
  ! grep -q "xattr" "$ITZTERM_TEST_LOG"
}

@test "piping the script to bash runs main without an unbound BASH_SOURCE error" {
  stub_uname Plan9 x86_64

  run bash -c 'cat "$1" | bash' _ "$ITZTERM_TEST_REPO_ROOT/scripts/install.sh"

  [ "$status" -ne 0 ]
  [[ "$output" == *"Unsupported operating system"* ]]
  [[ "$output" != *"unbound variable"* ]]
}

@test "executing the script directly runs main" {
  stub_uname Plan9 x86_64

  run bash "$ITZTERM_TEST_REPO_ROOT/scripts/install.sh"

  [ "$status" -ne 0 ]
  [[ "$output" == *"Unsupported operating system"* ]]
}

@test "sourcing the script does not run main" {
  stub_uname Plan9 x86_64

  run bash -c 'source "$1"; declare -F main >/dev/null && echo defined' _ "$ITZTERM_TEST_REPO_ROOT/scripts/install.sh"

  [ "$status" -eq 0 ]
  [ "$output" = "defined" ]
}
