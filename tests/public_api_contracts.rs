//! Consumer-facing contracts checked through crate-root exports only.
use opencc_jieba_rs::{DetofuMap, OpenCC, OpenccConfig};

#[test]
fn configurations_preserve_names_ffi_values_and_dispatch() {
    use OpenccConfig::*;
    type Convert = fn(&OpenCC, &str, bool) -> String;
    let cases: [(OpenccConfig, u32, &str, Convert); 20] = [
        (S2t, 1, "s2t", OpenCC::s2t),
        (S2tw, 2, "s2tw", OpenCC::s2tw),
        (S2twp, 3, "s2twp", OpenCC::s2twp),
        (S2hk, 4, "s2hk", OpenCC::s2hk),
        (T2s, 5, "t2s", OpenCC::t2s),
        (T2tw, 6, "t2tw", OpenCC::t2tw),
        (T2twp, 7, "t2twp", OpenCC::t2twp),
        (T2hk, 8, "t2hk", OpenCC::t2hk),
        (Tw2s, 9, "tw2s", OpenCC::tw2s),
        (Tw2sp, 10, "tw2sp", OpenCC::tw2sp),
        (Tw2t, 11, "tw2t", OpenCC::tw2t),
        (Tw2tp, 12, "tw2tp", OpenCC::tw2tp),
        (Hk2s, 13, "hk2s", OpenCC::hk2s),
        (Hk2t, 14, "hk2t", OpenCC::hk2t),
        (Jp2t, 15, "jp2t", OpenCC::jp2t),
        (T2jp, 16, "t2jp", OpenCC::t2jp),
        (S2hkp, 17, "s2hkp", OpenCC::s2hkp),
        (Hk2sp, 18, "hk2sp", OpenCC::hk2sp),
        (T2hkp, 19, "t2hkp", OpenCC::t2hkp),
        (Hk2tp, 20, "hk2tp", OpenCC::hk2tp),
    ];
    let cc = OpenCC::new();
    assert_eq!(OpenccConfig::ALL.len(), cases.len());
    for (config, number, name, direct) in cases {
        assert_eq!(
            OpenccConfig::ALL.iter().filter(|&&c| c == config).count(),
            1
        );
        assert_eq!(config.to_ffi(), number);
        assert_eq!(OpenccConfig::from_ffi(number), Some(config));
        assert_eq!(config.as_str(), name);
        assert_eq!(
            OpenccConfig::try_from(name.to_ascii_uppercase().as_str()),
            Ok(config)
        );
        for input in ["", "ASCII 123", "“软件鼠标，軟體滑鼠，舊字體廣國。”"] {
            for punctuation in [false, true] {
                let expected = direct(&cc, input, punctuation);
                assert_eq!(cc.convert_with_config(input, config, punctuation), expected);
                assert_eq!(cc.convert(input, name, punctuation), expected);
            }
        }
    }
    for invalid in ["", " s2t", "s2t ", "s2t.json", "unknown"] {
        assert_eq!(OpenccConfig::parse(invalid), None);
        assert!(!OpenccConfig::is_valid_config(invalid));
        assert_eq!(
            cc.convert("汉字", invalid, false),
            format!("Invalid config: {invalid}")
        );
    }
    for invalid in [0, 21, u32::MAX] {
        assert_eq!(OpenccConfig::from_ffi(invalid), None);
        assert!(!OpenccConfig::is_valid_config_ffi(invalid));
    }
}

#[test]
fn converters_can_be_shared_between_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OpenCC>();
    assert_send_sync::<DetofuMap>();
}

#[cfg(feature = "dictionary-build")]
#[test]
fn dictionary_builders_report_missing_sources() {
    // Use a child process so changing the working directory cannot race with
    // other tests that read the repository's dictionaries.
    const CHILD: &str = "OPENCC_TEST_MISSING_SOURCES";
    if std::env::var_os(CHILD).is_some() {
        use opencc_jieba_rs::dictionary_build::*;
        use opencc_jieba_rs::CustomDictFileSpec;
        let specs: [CustomDictFileSpec<&str>; 0] = [];
        for result in [
            write_json_pretty("pack.json"),
            write_zstd("pack.zst"),
            write_json_pretty_with_custom_dict_files("pack.json", &specs),
            write_zstd_with_custom_dict_files("pack.zst", &specs),
        ] {
            assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
        }
        return;
    }
    let directory =
        std::env::temp_dir().join(format!("opencc-missing-sources-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "dictionary_builders_report_missing_sources"])
        .env(CHILD, "1")
        .current_dir(&directory)
        .status();
    std::fs::remove_dir(&directory).unwrap();
    assert!(status.unwrap().success());
}
