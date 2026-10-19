use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const UPSTREAM_COMMIT: &str = "760af54d3c7e0cf3a846fdf3c7bbe278c9f6df3e";
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=INKPLAINER_HAND_ASSET_DIR");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let hand_dir = out_dir.join("inkplainer-hands");
    fs::create_dir_all(&hand_dir).expect("create hand asset output directory");

    let cache_dir = hand_asset_cache_dir();
    fs::create_dir_all(&cache_dir).expect("create hand asset cache directory");

    for hand in 1..=6 {
        for resolution in ["720p", "1080p", "1440p"] {
            stage(
                &hand_dir,
                &cache_dir,
                &format!("hand{hand}-{resolution}.png"),
                &format!("images/hand{hand}-{resolution}.png"),
            );
            stage(
                &hand_dir,
                &cache_dir,
                &format!("hand{hand}vertical-{resolution}.png"),
                &format!("images/hand{hand}vertical-{resolution}.png"),
            );
        }
    }
}

fn hand_asset_cache_dir() -> PathBuf {
    if let Some(explicit) = env::var_os("INKPLAINER_HAND_ASSET_DIR") {
        return PathBuf::from(explicit);
    }

    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .unwrap_or_else(env::temp_dir);

    cargo_home
        .join("inkplainer-hand-assets")
        .join(UPSTREAM_COMMIT)
}

fn stage(out_dir: &Path, cache_dir: &Path, local_name: &str, upstream_path: &str) {
    let destination = out_dir.join(local_name);
    if valid_png(&destination) {
        return;
    }

    let cached = cache_dir.join(local_name);
    if valid_png(&cached) {
        fs::copy(&cached, &destination).expect("copy cached hand sprite into Cargo output");
        return;
    }

    let url = format!(
        "https://raw.githubusercontent.com/NadirWeb-App/Inkplainer-OS/{UPSTREAM_COMMIT}/{upstream_path}"
    );

    let temporary = cache_dir.join(format!(".{local_name}.download"));
    let _ = fs::remove_file(&temporary);

    let downloaded = download_with_curl(&url, &temporary)
        || download_with_powershell(&url, &temporary);

    if downloaded && valid_png(&temporary) {
        fs::rename(&temporary, &cached)
            .or_else(|_| {
                fs::copy(&temporary, &cached)?;
                fs::remove_file(&temporary)
            })
            .expect("move downloaded hand sprite into persistent cache");
        fs::copy(&cached, &destination).expect("copy downloaded hand sprite into Cargo output");
        return;
    }

    let _ = fs::remove_file(&temporary);

    if env::var_os("CI").is_some() {
        panic!(
            "Required pinned upstream hand sprite is unavailable. URL: {url}. Cache: {}. \
             Re-run after connectivity is restored or pre-populate INKPLAINER_HAND_ASSET_DIR.",
            cached.display()
        );
    }

    // Keep local source builds usable without network access. Official CI/release
    // builds intentionally fail above instead of shipping a missing hand sprite.
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
    println!(
        "cargo:warning=Could not obtain pinned hand sprite {url}; using transparent local fallback"
    );
}

fn valid_png(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if metadata.len() <= 64 {
        return false;
    }
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    bytes.starts_with(PNG_SIGNATURE)
}

fn download_with_curl(url: &str, destination: &Path) -> bool {
    for program in ["curl.exe", "curl"] {
        let result = Command::new(program)
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--retry",
                "3",
                "--retry-delay",
                "1",
                "--connect-timeout",
                "15",
                "--output",
            ])
            .arg(destination)
            .arg(url)
            .status();
        if result.map(|status| status.success()).unwrap_or(false) {
            return true;
        }
    }
    false
}

fn download_with_powershell(url: &str, destination: &Path) -> bool {
    let dest = destination.to_string_lossy().replace('\'', "''");
    let url = url.replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; \
         for ($i=0; $i -lt 3; $i++) {{ \
           try {{ Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile '{dest}'; exit 0 }} \
           catch {{ if ($i -eq 2) {{ throw }}; Start-Sleep -Seconds 1 }} \
         }}"
    );
    Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
