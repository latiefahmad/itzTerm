#!/usr/bin/env bats

load "helpers.bash"

setup() {
  make_tmp
  source "$ITZTERM_TEST_REPO_ROOT/scripts/release/homebrew.sh"
}

teardown() {
  cleanup_tmp
}

@test "normalizes full SemVer tags with dotted prerelease and build identifiers" {
  run normalize_release_version "v1.2.3-beta.1"
  [ "$status" -eq 0 ]
  [ "$output" = "1.2.3-beta.1" ]

  run normalize_release_version "1.2.3-beta.1+macos.7"
  [ "$status" -eq 0 ]
  [ "$output" = "1.2.3-beta.1+macos.7" ]

  run normalize_release_version "1.2.3+signed-macos.7"
  [ "$status" -eq 0 ]
  [ "$output" = "1.2.3+signed-macos.7" ]
}

@test "rejects incomplete unsafe and leading-zero versions" {
  for invalid in \
    "1.2" \
    "1.2.3/../../tap" \
    "01.2.3" \
    "1.02.3" \
    "1.2.03" \
    "1.2.3-01" \
    "1.2.3-beta.01"; do
    run normalize_release_version "$invalid"
    [ "$status" -ne 0 ]
  done
}

@test "classifies prerelease before build metadata only" {
  run is_release_prerelease "1.2.3-beta.1+signed-macos.7"
  [ "$status" -eq 0 ]

  run is_release_prerelease "1.2.3+signed-macos.7"
  [ "$status" -ne 0 ]
}

@test "resolves exact DMG checksums" {
  local checksums="$ITZTERM_TEST_TMP_DIR/SHA256SUMS.txt"
  local arm_sha="$(printf 'a%.0s' {1..64})"
  local intel_sha="$(printf 'b%.0s' {1..64})"
  cat >"$checksums" <<EOF
$arm_sha  itzTerm.Manager_0.4.8_aarch64.dmg
$intel_sha *itzTerm.Manager_0.4.8_x64.dmg
EOF

  run resolve_dmg_checksums "$checksums" "0.4.8"
  [ "$status" -eq 0 ]
  [ "${lines[0]}" = "$arm_sha" ]
  [ "${lines[1]}" = "$intel_sha" ]
}

@test "checksum resolution propagates missing malformed and duplicate errors" {
  local checksums="$ITZTERM_TEST_TMP_DIR/SHA256SUMS.txt"
  local arm_sha="$(printf 'a%.0s' {1..64})"
  local intel_sha="$(printf 'b%.0s' {1..64})"

  printf '%s  %s\n' "$arm_sha" "itzTerm.Manager_0.4.8_aarch64.dmg" >"$checksums"
  run resolve_dmg_checksums "$checksums" "0.4.8"
  [ "$status" -ne 0 ]
  [[ "$output" == *"x64.dmg"* ]]

  cat >"$checksums" <<EOF
not-a-hash  itzTerm.Manager_0.4.8_aarch64.dmg
$intel_sha  itzTerm.Manager_0.4.8_x64.dmg
EOF
  run resolve_dmg_checksums "$checksums" "0.4.8"
  [ "$status" -ne 0 ]
  [[ "$output" == *"aarch64.dmg"* ]]

  cat >"$checksums" <<EOF
$arm_sha  itzTerm.Manager_0.4.8_aarch64.dmg
$arm_sha  itzTerm.Manager_0.4.8_aarch64.dmg
$intel_sha  itzTerm.Manager_0.4.8_x64.dmg
EOF
  run resolve_dmg_checksums "$checksums" "0.4.8"
  [ "$status" -ne 0 ]
  [[ "$output" == *"aarch64.dmg"* ]]
}

@test "generates the exact v0.4.8 xattr exception and omits it for future releases" {
  local legacy="$ITZTERM_TEST_TMP_DIR/itzterm-0.4.8.rb"
  local future="$ITZTERM_TEST_TMP_DIR/itzterm-0.4.9.rb"
  local arm_sha="6be298c2c2c8562b340b069357e8b5d6c3838791ac77c089114004db6a663e69"
  local intel_sha="72b1d5ab617dcc72c021ec4524ec90a8607870d2011fa83686c4ccda185854c8"

  write_homebrew_cask "$legacy" "0.4.8" "$arm_sha" "$intel_sha"
  [ "$(grep -Fc 'com.apple.quarantine' "$legacy")" -eq 1 ]
  grep -Fq 'args: ["-dr", "com.apple.quarantine", "#{appdir}/itzTerm.app"]' "$legacy"

  write_homebrew_cask "$future" "0.4.9" "$arm_sha" "$intel_sha"
  ! grep -Fq 'com.apple.quarantine' "$future"
}

@test "prerelease metadata path does not require a Homebrew token" {
  local workflow="$ITZTERM_TEST_REPO_ROOT/.github/workflows/publish-homebrew.yml"
  local metadata_section
  local checksums_section
  metadata_section="$(sed -n '/release_metadata:/,/checksums:/p' "$workflow")"
  checksums_section="$(sed -n '/checksums:/,/homebrew:/p' "$workflow")"

  grep -Fq "if: needs.release_metadata.outputs.is_prerelease == 'false'" "$workflow"
  [ -n "$metadata_section" ]
  [ -n "$checksums_section" ]
  ! grep -q 'HOMEBREW_TAP_TOKEN' <<<"$metadata_section"
  ! grep -q 'HOMEBREW_TAP_TOKEN' <<<"$checksums_section"
}

@test "release workflows preserve permissions token flow portability and tap serialization" {
  local release_workflow="$ITZTERM_TEST_REPO_ROOT/.github/workflows/release.yml"
  local homebrew_workflow="$ITZTERM_TEST_REPO_ROOT/.github/workflows/publish-homebrew.yml"

  grep -Fq 'group: publish-homebrew-tap' "$homebrew_workflow"
  grep -Fq 'GH_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}' "$homebrew_workflow"
  grep -Fq 'contents: write' "${release_workflow}"
  grep -Fq 'source scripts/release/homebrew.sh' "$release_workflow"
  grep -Fq 'otool -L "$executable"' "$release_workflow"
  grep -Fq 'LC_RPATH' "$release_workflow"
  local macos_verification_section
  macos_verification_section="$(sed -n '/Verify macOS bundle library portability and signing/,/Collect platform release assets/p' "$release_workflow")"
  [ -n "$macos_verification_section" ]
  ! grep -q 'mapfile' <<<"$macos_verification_section"
  ! grep -Fq 'env.APPLE_CERTIFICATE' "$release_workflow"
  grep -Fq 'security import "$p12"' "$release_workflow"
  grep -Fq 'secrets.APPLE_SIGNING_IDENTITY' "$release_workflow"
  grep -Fq 'secrets.APPLE_TEAM_ID' "$release_workflow"
}

@test "generate_sha256sums writes bare-name entries and skips signatures and manifests" {
  local dir="$ITZTERM_TEST_TMP_DIR/assets"
  local names="$ITZTERM_TEST_TMP_DIR/names.txt"
  local out="$ITZTERM_TEST_TMP_DIR/SHA256SUMS.txt"
  mkdir -p "$dir"
  printf 'app-image-bytes' >"$dir/itzTerm.Manager_0.4.11_amd64.AppImage"
  printf 'sig-bytes' >"$dir/itzTerm.Manager_0.4.11_amd64.AppImage.sig"
  printf '{}' >"$dir/latest.json"
  printf 'server-bytes' >"$dir/itzterm-server"
  cat >"$names" <<NAMES
itzTerm.Manager_0.4.11_amd64.AppImage
itzTerm.Manager_0.4.11_amd64.AppImage.sig
latest.json
itzterm-server
NAMES

  run generate_sha256sums "$dir" "$names" "$out"
  [ "$status" -eq 0 ]

  # Entries carry the bare asset name (two-space separator), not a path.
  local expected_appimage_sha expected_server_sha
  expected_appimage_sha="$(printf 'app-image-bytes' | shasum -a 256 | awk '{print $1}')"
  expected_server_sha="$(printf 'server-bytes' | shasum -a 256 | awk '{print $1}')"
  grep -Fq "$expected_appimage_sha  itzTerm.Manager_0.4.11_amd64.AppImage" "$out"
  grep -Fq "$expected_server_sha  itzterm-server" "$out"

  # Signatures and updater manifests are not checksummed.
  [ "$(grep -c '' "$out")" -eq 2 ]
  ! grep -Fq '.sig' "$out"
  ! grep -Fq 'latest.json' "$out"
}

@test "generate_sha256sums refuses filenames that do not match published asset names" {
  # Regression for #546: v0.4.10 shipped SHA256SUMS entries named
  # "itzTerm_..." (product display name with a space) while the
  # published assets are "itzTerm.Manager_..." — the installer's exact-match
  # lookup then fails with "checksum not found".
  local dir="$ITZTERM_TEST_TMP_DIR/assets"
  local names="$ITZTERM_TEST_TMP_DIR/names.txt"
  local out="$ITZTERM_TEST_TMP_DIR/SHA256SUMS.txt"
  mkdir -p "$dir"
  printf 'app-image-bytes' >"$dir/itzTerm_0.4.10_amd64.AppImage"
  printf 'itzTerm.Manager_0.4.10_amd64.AppImage' >"$names"

  run generate_sha256sums "$dir" "$names" "$out"
  [ "$status" -ne 0 ]
  [[ "$output" == *"not a published release asset name"* ]]
  [[ "$output" == *"itzTerm_0.4.10_amd64.AppImage"* ]]

  # Nothing may be written when validation fails.
  [ ! -s "$out" ]
  # The atomic-write temp file must be cleaned up too.
  [ -z "$(find "$ITZTERM_TEST_TMP_DIR" -maxdepth 1 -name 'SHA256SUMS.txt.tmp.*' -print -quit)" ]
}

@test "generate_sha256sums leaves no partial output when a valid asset sorts before a mismatched one" {
  # CodeRabbit review: a valid asset processed before the failure must not
  # leave partial checksum output behind, and a previously published
  # checksum file must survive the failed regeneration.
  local dir="$ITZTERM_TEST_TMP_DIR/assets"
  local names="$ITZTERM_TEST_TMP_DIR/names.txt"
  local out="$ITZTERM_TEST_TMP_DIR/SHA256SUMS.txt"
  mkdir -p "$dir"
  # "AAA..." sorts before "itzTerm...", so the valid asset is
  # checksummed first and the mismatch then aborts the run.
  printf 'deb-bytes' >"$dir/AAA_valid.deb"
  printf 'app-image-bytes' >"$dir/itzTerm_0.4.10_amd64.AppImage"
  printf 'AAA_valid.deb' >"$names"

  printf 'old-checksum-content\n' >"$out"

  run generate_sha256sums "$dir" "$names" "$out"
  [ "$status" -ne 0 ]
  [[ "$output" == *"not a published release asset name"* ]]

  # The prior checksum file is untouched — no partial write, no clobber.
  [ "$(cat "$out")" = "old-checksum-content" ]
  [ -z "$(find "$ITZTERM_TEST_TMP_DIR" -maxdepth 1 -name 'SHA256SUMS.txt.tmp.*' -print -quit)" ]
}

@test "generate_sha256sums requires assets and a nonempty expected-names list" {
  local dir="$ITZTERM_TEST_TMP_DIR/empty-assets"
  local names="$ITZTERM_TEST_TMP_DIR/names.txt"
  local out="$ITZTERM_TEST_TMP_DIR/SHA256SUMS.txt"
  mkdir -p "$dir"
  : >"$names"

  run generate_sha256sums "$dir" "$names" "$out"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing or empty"* ]]

  printf 'itzTerm.Manager_0.4.11_amd64.AppImage' >"$names"
  run generate_sha256sums "$dir" "$names" "$out"
  [ "$status" -ne 0 ]
  [[ "$output" == *"No binary release assets"* ]]
}

@test "generated cask does not declare a depends_on macos requirement" {
  local cask="$ITZTERM_TEST_TMP_DIR/itzterm-0.4.9.rb"
  local arm_sha="6be298c2c2c8562b340b069357e8b5d6c3838791ac77c089114004db6a663e69"
  local intel_sha="72b1d5ab617dcc72c021ec4524ec90a8607870d2011fa83686c4ccda185854c8"

  write_homebrew_cask "$cask" "0.4.9" "$arm_sha" "$intel_sha"

  run grep -E '^[[:space:]]*depends_on[[:space:]]' "$cask"
  [ "$status" -eq 1 ]
  [ -z "$output" ]
}
