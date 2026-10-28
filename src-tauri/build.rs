use std::path::Path;

/// Cfg set when a release-profile build would compile with Tauri's `cfg(dev)`
/// on (no `custom-protocol`). The desktop entry point (`src/main.rs`) turns it
/// into a `compile_error!`, so only the desktop binary is blocked: a build
/// script cannot see which `--bin` targets Cargo selected, but the guard then
/// fires exactly when the desktop binary compiles, whatever features are on.
/// The `itzterm-server` binary (`server_main.rs`) and the library never trip it.
const RELEASE_WITHOUT_CUSTOM_PROTOCOL_CFG: &str = "itzterm_release_without_custom_protocol";

/// Whether a release-profile build lacks `custom-protocol`, which makes the
/// desktop binary load `devUrl` and show "Could not connect to localhost".
///
/// `dev` is `tauri_build::is_dev()` (the `DEP_TAURI_DEV` value emitted by the
/// `tauri` crate's build script, `true` whenever `tauri/custom-protocol` is
/// off). The escape hatch `1` / `true` disables the guard.
fn release_without_custom_protocol(profile: &str, dev: bool, allow_override: &str) -> bool {
    let allowed = matches!(allow_override.trim(), "1" | "true");
    profile == "release" && dev && !allowed
}

fn main() {
    // The Vite web build output (`../dist-web/`) is embedded into BOTH the
    // standalone `itzterm-server` binary and the desktop app (the desktop's
    // in-process shared-live server serves the embedded bundle in a release
    // install). Watch it so a web-client rebuild triggers a cargo rebuild, for
    // both build targets.
    println!("cargo:rerun-if-changed=../dist-web");
    // The figcraft web bundle (`canvas/bundle/`, fetched by
    // `scripts/canvas/fetch-figcraft-bundle.mjs`) is embedded the same way.
    // A missing bundle must not fail dev/CI, but a release build without it
    // would ship a canvas that answers BUNDLE_MISSING, so say so loudly.
    println!("cargo:rerun-if-changed=../canvas/bundle");
    println!("cargo:rerun-if-changed=../canvas/figcraft.pin");
    if std::env::var("PROFILE").as_deref() == Ok("release")
        && !Path::new("../canvas/bundle/index.html").is_file()
    {
        println!(
            "cargo:warning=canvas/bundle/index.html is missing: this release build will not embed \
             the figcraft canvas bundle (run scripts/canvas/fetch-figcraft-bundle.mjs first)"
        );
    }

    // Declare the custom `web_embed_missing` cfg so the 2024 edition's
    // unexpected-cfg check doesn't reject the `#[cfg(web_embed_missing)]` in
    // `assets.rs`.
    println!("cargo:rustc-check-cfg=cfg(web_embed_missing)");
    // Set by `.cargo/config.toml` (repo root and src-tauri) so async-process
    // uses its SIGCHLD reaper instead of the Linux pidfd epoll loop.
    println!("cargo:rustc-check-cfg=cfg(async_process_force_signal_backend)");

    // Build sequencing + clear missing-bundle failure:
    // `rust-embed`'s `#[allow_missing]` compiles an EMPTY embed when
    // `dist-web/` is absent — fine for dev (`cargo check`/`cargo clippy`
    // without `bun run build:web`) and CI compile-green. But a RELEASE build
    // with a missing/stale bundle would ship a self-contained binary that 404s
    // every static route — a silent deploy bug. Emit a `web_embed_missing` cfg
    // so the release path (NOT debug) hits a `compile_error!` in `assets.rs`
    // telling the operator to run `bun run build:web` first. The Vite build
    // MUST run before `cargo build --bin itzterm-server` (rust-embed embeds at
    // build time) — CI enforces this ordering (`.github/workflows/*`).
    // `index.html` alone is not sufficient: a stale `dist-web/` that predates
    // the PWA files would compile cleanly yet ship an `index.html` linking a
    // manifest + registering a service worker that 404. Require the whole
    // PWA surface — manifest, worker, favicon, and every install icon — so a
    // stale bundle also trips the release-time gate.
    let required = [
        "index.html",
        "manifest.webmanifest",
        "sw.js",
        "favicon.ico",
        "icons/pwa-192.png",
        "icons/pwa-512.png",
        "icons/pwa-maskable-512.png",
        "icons/apple-touch-icon.png",
    ];
    if required
        .iter()
        .any(|file| !Path::new("../dist-web").join(file).is_file())
    {
        println!("cargo:rustc-cfg=web_embed_missing");
    }

    // Block a raw release desktop build without `custom-protocol` (enforced by
    // the `compile_error!` in `src/main.rs`).
    println!("cargo:rustc-check-cfg=cfg({RELEASE_WITHOUT_CUSTOM_PROTOCOL_CFG})");
    println!("cargo:rerun-if-env-changed=ITZTERM_ALLOW_RELEASE_WITHOUT_CUSTOM_PROTOCOL");
    if release_without_custom_protocol(
        &std::env::var("PROFILE").unwrap_or_default(),
        tauri_build::is_dev(),
        &std::env::var("ITZTERM_ALLOW_RELEASE_WITHOUT_CUSTOM_PROTOCOL").unwrap_or_default(),
    ) {
        println!("cargo:rustc-cfg={RELEASE_WITHOUT_CUSTOM_PROTOCOL_CFG}");
    }

    // Windows: embed the Common-Controls v6 manifest into every cargo
    // artifact, including the lib unit-test harness. App binaries get their
    // manifest from tauri-build's generated resource.lib (linked into bins
    // only), and cargo's `link-arg-tests` directive reaches integration-test
    // targets but NOT the lib test harness, so out of the box only the lib
    // harness shipped rustc's default manifest WITHOUT the Common-Controls
    // dependency. The loader then resolved comctl32 5.82, which lacks the
    // v6-only entry points the library imports (TaskDialogIndirect, window
    // subclassing helpers) and every test run died at load with 0xC0000139
    // STATUS_ENTRYPOINT_NOT_FOUND.
    // /MANIFESTINPUT is only valid together with /MANIFEST:EMBED (LNK1220),
    // so both go through `rustc-link-arg`, which reaches every target.
    // MSVC-only flags: guard on the target OS triple's env, not just OS, so
    // a hypothetical GNU toolchain target never sees them.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("manifest/tests.xml");
        println!("cargo:rerun-if-changed=manifest/tests.xml");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
            manifest.display()
        );
        // Bins already carry their manifest as an NT resource from
        // tauri-build's generated resource.lib (RT_MANIFEST id 1) — letting
        // the linker also generate one duplicates that resource and fails
        // with CVT1100. /MANIFEST:NO runs after /MANIFEST:EMBED on bin
        // targets, so the linker skips manifest generation there and the
        // resource.lib manifest (which includes Common-Controls) stays the
        // only one, exactly matching the pre-workaround link behavior.
        println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    }

    tauri_build::build()
}
