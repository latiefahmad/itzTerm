ITZTERM_TEST_REPO_ROOT="$(cd "${BATS_TEST_DIRNAME:-$(dirname "${BASH_SOURCE[0]}")}/../.." && pwd)"

make_tmp() {
  ITZTERM_TEST_TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/itzterm-install-test.XXXXXX")"
  ITZTERM_TEST_STUB_BIN="$ITZTERM_TEST_TMP_DIR/bin"
  mkdir -p "$ITZTERM_TEST_STUB_BIN"

  ITZTERM_TEST_ORIGINAL_PATH="${ITZTERM_TEST_ORIGINAL_PATH:-$PATH}"
  PATH="$ITZTERM_TEST_STUB_BIN:$ITZTERM_TEST_ORIGINAL_PATH"

  export ITZTERM_TEST_TMP_DIR
  export ITZTERM_TEST_STUB_BIN
  export ITZTERM_TEST_ORIGINAL_PATH
  export PATH
}

cleanup_tmp() {
  if [[ -n "${ITZTERM_TEST_TMP_DIR:-}" && -d "$ITZTERM_TEST_TMP_DIR" ]]; then
    rm -rf "$ITZTERM_TEST_TMP_DIR"
  fi

  if [[ -n "${ITZTERM_TEST_ORIGINAL_PATH:-}" ]]; then
    PATH="$ITZTERM_TEST_ORIGINAL_PATH"
    export PATH
  fi

  unset ITZTERM_TEST_TMP_DIR
  unset ITZTERM_TEST_STUB_BIN
  unset ITZTERM_TEST_ORIGINAL_PATH
}

stub_cmd() {
  local name="$1"
  shift

  if [[ -z "${ITZTERM_TEST_STUB_BIN:-}" ]]; then
    make_tmp
  fi

  local stub_path="$ITZTERM_TEST_STUB_BIN/$name"
  {
    printf '#!/usr/bin/env bash\n'
    printf '%s\n' "$*"
  } >"$stub_path"
  chmod +x "$stub_path"
}

load_install() {
  source "$ITZTERM_TEST_REPO_ROOT/scripts/install.sh"
}
