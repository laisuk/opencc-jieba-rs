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

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let revision = read_capi_revision();
    println!(
        "cargo:rustc-env=OPENCC_CLI_VERSION={}.{}",
        env!("CARGO_PKG_VERSION"),
        revision
    );
    
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");

        use std::env;
        use winres::{VersionInfo, WindowsResource};

        // --- Version from Cargo (SemVer: major.minor.patch) ---
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
        let revision: u16 = 0; // implicit 4th digit

        // Pack into Windows FIXEDFILEINFO format
        let packed: u64 = ((major as u64) << 48)
            | ((minor as u64) << 32)
            | ((patch as u64) << 16)
            | (revision as u64);

        // String forms
        let ver_dots = format!("{major}.{minor}.{patch}.{revision}");
        let ver_commas = format!("{major},{minor},{patch},{revision}");

        // Other Cargo metadata
        let name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "dict-generate".into());
        let authors = env::var("CARGO_PKG_AUTHORS").unwrap_or_else(|_| "Laisuk".into());
        let desc = env::var("CARGO_PKG_DESCRIPTION")
            .unwrap_or_else(|_| "Opencc-Jieba dictionary generator tool".into());

        let mut res = WindowsResource::new();
        res.set_icon("assets/icon.ico");

        // --- Authoritative numeric versions ---
        res.set_version_info(VersionInfo::FILEVERSION, packed);
        res.set_version_info(VersionInfo::PRODUCTVERSION, packed);

        // --- Explorer-readable strings ---
        res.set("FileVersion", &ver_dots);
        res.set("ProductVersion", &ver_dots);
        res.set("FileVersionRaw", &ver_commas);
        res.set("ProductVersionRaw", &ver_commas);

        // --- Rich metadata ---
        res.set("FileDescription", &desc);
        res.set("ProductName", "Opencc-Jieba-Rs Dict Generator");
        res.set("CompanyName", &authors);
        res.set("LegalCopyright", "© Laisuk. MIT License");
        res.set("OriginalFilename", "dict-generate.exe");
        res.set("InternalName", &name);
        res.set("Comments", "Generate dictionary for Opencc-Jieba-Rs.");

        // Optional extra tags (some scanners treat these as "complete" PE info)
        res.set(
            "LegalTrademarks",
            "OpenCC is a trademark of BYVoid and contributors.",
        );

        res.compile().expect("Failed to embed Windows resources");
    }
}
