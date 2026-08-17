use opencc_jieba_rs::{
    CustomDictFileSpec, CustomDictMode, CustomDictSpec, DictSlot, OpenCC,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dict_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!(
            "opencc-jieba-rs-custom-dict-{name}-{nonce}.txt"
        ))
    }

    #[test]
    fn custom_pairs_append_character_mapping() {
        let mut opencc = OpenCC::new();

        opencc
            .load_custom_dicts(&[CustomDictSpec {
                slot: DictSlot::STCharacters,
                pairs: vec![("龙".to_string(), "龍龍".to_string())],
                mode: CustomDictMode::Append,
            }])
            .unwrap();

        assert_eq!(opencc.s2t("龙", false), "龍龍");
    }

    #[test]
    fn custom_pairs_append_is_last_wins() {
        let mut opencc = OpenCC::new();

        opencc
            .load_custom_dicts(&[
                CustomDictSpec {
                    slot: DictSlot::STCharacters,
                    pairs: vec![("龙".to_string(), "甲".to_string())],
                    mode: CustomDictMode::Append,
                },
                CustomDictSpec {
                    slot: DictSlot::STCharacters,
                    pairs: vec![("龙".to_string(), "乙".to_string())],
                    mode: CustomDictMode::Append,
                },
            ])
            .unwrap();

        assert_eq!(opencc.s2t("龙", false), "乙");
    }

    #[test]
    fn custom_pairs_override_clears_target_slot() {
        let mut opencc = OpenCC::new();

        assert_eq!(opencc.s2t("汉龙", false), "漢龍");

        opencc
            .load_custom_dicts(&[CustomDictSpec {
                slot: DictSlot::STCharacters,
                pairs: vec![("龙".to_string(), "龍龍".to_string())],
                mode: CustomDictMode::Override,
            }])
            .unwrap();

        // STCharacters was replaced, so the built-in 汉 -> 漢 character
        // mapping is no longer available while the custom 龙 mapping remains.
        assert_eq!(opencc.s2t("汉龙", false), "汉龍龍");
    }

    #[test]
    fn custom_pairs_invalid_entry_is_transactional() {
        let mut opencc = OpenCC::new();

        let result = opencc.load_custom_dicts(&[
            CustomDictSpec {
                slot: DictSlot::STCharacters,
                pairs: vec![("龙".to_string(), "甲".to_string())],
                mode: CustomDictMode::Append,
            },
            CustomDictSpec {
                slot: DictSlot::STCharacters,
                pairs: vec![("".to_string(), "乙".to_string())],
                mode: CustomDictMode::Append,
            },
        ]);

        assert!(result.is_err());

        // The valid first spec must not have been applied before validation
        // failed on the later spec.
        assert_eq!(opencc.s2t("龙", false), "龍");
    }

    #[test]
    fn custom_file_append_character_mapping() {
        let path = temp_dict_path("append");
        fs::write(&path, "龙\t龍龍\n").unwrap();

        let mut opencc = OpenCC::new();

        let result = opencc.load_custom_dict_files(&[CustomDictFileSpec {
            slot: DictSlot::STCharacters,
            files: vec![path.clone()],
            mode: CustomDictMode::Append,
        }]);

        let _ = fs::remove_file(&path);

        result.unwrap();
        assert_eq!(opencc.s2t("龙", false), "龍龍");
    }

    #[test]
    fn custom_file_multiple_files_are_applied_in_order() {
        let path1 = temp_dict_path("order-1");
        let path2 = temp_dict_path("order-2");

        fs::write(&path1, "龙\t甲\n").unwrap();
        fs::write(&path2, "龙\t乙\n").unwrap();

        let mut opencc = OpenCC::new();

        let result = opencc.load_custom_dict_files(&[CustomDictFileSpec {
            slot: DictSlot::STCharacters,
            files: vec![path1.clone(), path2.clone()],
            mode: CustomDictMode::Append,
        }]);

        let _ = fs::remove_file(&path1);
        let _ = fs::remove_file(&path2);

        result.unwrap();
        assert_eq!(opencc.s2t("龙", false), "乙");
    }

    #[test]
    fn custom_file_parser_supports_comments_bom_and_multiple_values() {
        let path = temp_dict_path("parser");

        fs::write(
            &path,
            "\u{FEFF}# custom dictionary\n\n龙\t龍龍 備選值\n",
        )
        .unwrap();

        let mut opencc = OpenCC::new();

        let result = opencc.load_custom_dict_files(&[CustomDictFileSpec {
            slot: DictSlot::STCharacters,
            files: vec![path.clone()],
            mode: CustomDictMode::Append,
        }]);

        let _ = fs::remove_file(&path);

        result.unwrap();
        assert_eq!(opencc.s2t("龙", false), "龍龍");
    }

    #[test]
    fn custom_file_parse_failure_is_transactional() {
        let valid_path = temp_dict_path("transaction-valid");
        let invalid_path = temp_dict_path("transaction-invalid");

        fs::write(&valid_path, "龙\t甲\n").unwrap();
        fs::write(&invalid_path, "missing-tab-separator\n").unwrap();

        let mut opencc = OpenCC::new();

        let result = opencc.load_custom_dict_files(&[
            CustomDictFileSpec {
                slot: DictSlot::STCharacters,
                files: vec![valid_path.clone()],
                mode: CustomDictMode::Append,
            },
            CustomDictFileSpec {
                slot: DictSlot::STCharacters,
                files: vec![invalid_path.clone()],
                mode: CustomDictMode::Append,
            },
        ]);

        let _ = fs::remove_file(&valid_path);
        let _ = fs::remove_file(&invalid_path);

        assert!(result.is_err());

        // All files are parsed before load_custom_dicts() is called, so the
        // earlier valid file must not partially mutate the converter.
        assert_eq!(opencc.s2t("龙", false), "龍");
    }

    #[test]
    fn custom_phrase_works_when_jieba_preserves_the_domain_term() {
        let jieba_path = temp_dict_path("jieba");
        fs::write(&jieba_path, "帕兰蒂尔 100000 nz\n").unwrap();

        let mut opencc = OpenCC::new();

        let result = opencc.load_user_dict(&jieba_path);
        let _ = fs::remove_file(&jieba_path);
        result.unwrap();

        opencc
            .load_custom_dicts(&[CustomDictSpec {
                slot: DictSlot::STPhrases,
                pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
                mode: CustomDictMode::Append,
            }])
            .unwrap();

        assert_eq!(
            opencc.jieba.cut("帕兰蒂尔", false),
            vec!["帕兰蒂尔"]
        );
        assert_eq!(opencc.s2t("帕兰蒂尔", false), "柏蘭蒂爾");
    }
}
