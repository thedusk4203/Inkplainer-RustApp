use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const UPSTREAM_COMMIT: &str = "760af54d3c7e0cf3a846fdf3c7bbe278c9f6df3e";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let hand_dir = out_dir.join("inkplainer-hands");
    fs::create_dir_all(&hand_dir).expect("create hand asset output directory");

    for hand in 1..=6 {
        for resolution in ["720p", "1080p", "1440p"] {
            stage(
                &hand_dir,
                &format!("hand{hand}-{resolution}.png"),
                &format!("images/hand{hand}-{resolution}.png"),
            );
            stage(
                &hand_dir,
                &format!("hand{hand}vertical-{resolution}.png"),
                &format!("images/hand{hand}vertical-{resolution}.png"),
            );
        }
    }
}

fn stage(out_dir: &Path, local_name: &str, upstream_path: &str) {
    let destination = out_dir.join(local_name);
    if destination.metadata().map(|m| m.len() > 64).unwrap_or(false) {
        return;
    }

    let url = format!(
        "https://raw.githubusercontent.com/NadirWeb-App/Inkplainer-OS/{UPSTREAM_COMMIT}/{upstream_path}"
    );

    if download_with_powershell(&url, &destination) {
        return;
    }

    // Keep source builds working without network access. CI/release builds have
    // network access and therefore embed the exact pinned upstream sprites.
    // This is a valid 1x1 transparent PNG used only as an offline build fallback.
    const TRANSPARENT_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
        0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
        0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41,
        0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
        0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d,
        0xb1, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
        0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    fs::write(&destination, TRANSPARENT_PNG).expect("write transparent hand fallback");
    println!("cargo:warning=Could not download {url}; using transparent hand fallback");
}

fn download_with_powershell(url: &str, destination: &Path) -> bool {
    let dest = destination.to_string_lossy().replace(''', "''");
    let url = url.replace(''', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; \
         Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile '{dest}'"
    );
    Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
