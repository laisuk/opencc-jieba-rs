// build.rs

#[cfg(target_os = "windows")]
fn read_capi_revision() -> u16 {
    use std::{env, fs, path::Path};
    use toml::Value;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing");
    let toml_path = Path::new(&manifest_dir).join("Cargo.toml");

    let text = fs::read_to_string(&toml_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", toml_path.display()));
    let value: Value = text
        .parse::<Value>()
        .unwrap_or_else(|e| panic!("Failed to parse Cargo.toml as TOML: {e}"));

    let rev_i64 = value
        .get("package")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("capi"))
        .and_then(|v| v.get("revision"))
        .and_then(|v| v.as_integer())
        .unwrap_or(0);

    if !(0..=(u16::MAX as i64)).contains(&rev_i64) {
        panic!(
            "[package.metadata.capi] revision out of range for u16: {rev_i64} (must be 0..={})",
            u16::MAX
        );
    }

    rev_i64 as u16
}

#[cfg(target_os = "windows")]
fn pack_win_ver(major: u16, minor: u16, patch: u16, revision: u16) -> u64 {
    ((major as u64) << 48) | ((minor as u64) << 32) | ((patch as u64) << 16) | (revision as u64)
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=Cargo.toml");

        use std::env;
        use winres::{VersionInfo, WindowsResource};

        let major: u16 = env::var("CARGO_PKG_VERSION_MAJOR")
            .unwrap()
            .parse()
            .unwrap();
        let minor: u16 = env::var("CARGO_PKG_VERSION_MINOR")
            .unwrap()
            .parse()
            .unwrap();
        let patch: u16 = env::var("CARGO_PKG_VERSION_PATCH")
            .unwrap()
            .parse()
            .unwrap();
        let revision: u16 = read_capi_revision();

        let packed_u64 = pack_win_ver(major, minor, patch, revision);

        // String table (Explorer-friendly; may hide trailing .0, but it’s correct)
        let ver_str_dots = format!("{major}.{minor}.{patch}.{revision}");
        // Numeric raw (VS_FIXEDFILEINFO uses comma-separated parts)
        let ver_str_commas = ver_str_dots.replace('.', ",");

        let this_pkg_name =
            env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "opencc_jieba_capi".into());

        let authors = env::var("CARGO_PKG_AUTHORS").unwrap_or_else(|_| "Laisuk".into());
        let desc = env::var("CARGO_PKG_DESCRIPTION").unwrap_or_else(|_| {
            "Opencc-Jieba Rust C API (Simplified/Traditional Chinese Converter)".into()
        });

        let mut res = WindowsResource::new();

        // ✅ Authoritative numeric versions (VS_FIXEDFILEINFO)
        res.set_version_info(VersionInfo::FILEVERSION, packed_u64);
        res.set_version_info(VersionInfo::PRODUCTVERSION, packed_u64);

        // ✅ String table versions (Explorer “Details” page)
        res.set("FileVersion", &ver_str_dots);
        res.set("ProductVersion", &ver_str_dots);

        // ✅ Optional: also explicitly set the "*Raw" strings
        // (helps some tooling that reads string table instead of FIXEDFILEINFO)
        res.set("FileVersionRaw", &ver_str_commas);
        res.set("ProductVersionRaw", &ver_str_commas);

        // Other metadata
        res.set("FileDescription", &desc);
        res.set("ProductName", "Opencc-Jieba Rust C API");
        res.set("CompanyName", &authors);
        res.set("OriginalFilename", "opencc_jieba_capi.dll");
        res.set("InternalName", &this_pkg_name);
        res.set("LegalCopyright", "© Laisuk. MIT License");
        res.set("Comments", "Built with Rust and Opencc-Jieba libraries.");

        res.compile().expect("Failed to embed Windows resources");
    }
}
