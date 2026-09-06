use opencc_jieba_rs::{OpenCC, OpenccConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use opencc_jieba_rs::{DetofuLevel, DetofuMap, OpenccError, UserDictEntry};
    use std::sync::Arc;

    #[test]
    fn zho_check_test() {
        let input = "你好，世界！龙马精神！着著";
        let expected_output = 2;
        let opencc = OpenCC::new();
        let actual_output = opencc.zho_check(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2t_test() {
        let input = "你好，世界！龙马精神！";
        let expected_output = "你好，世界！龍馬精神！";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2t(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2tw_test() {
        let input = "你好，这里世界！龙马精神！";
        let expected_output = "你好，這裡世界！龍馬精神！";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2tw(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2twp_test() {
        let input = "意大利项目";
        let expected_output = "義大利專案";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2twp(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn hong_kong_phrase_chain_tests() {
        let opencc = OpenCC::new();

        assert_eq!(OpenccConfig::from_ffi(17), Some(OpenccConfig::S2hkp));
        assert_eq!(OpenccConfig::from_ffi(18), Some(OpenccConfig::Hk2sp));
        assert_eq!(OpenccConfig::from_ffi(19), Some(OpenccConfig::T2hkp));
        assert_eq!(OpenccConfig::from_ffi(20), Some(OpenccConfig::Hk2tp));
        assert_eq!(OpenccConfig::S2hkp.as_str(), "s2hkp");
        assert_eq!(OpenccConfig::Hk2sp.as_str(), "hk2sp");
        assert_eq!(OpenccConfig::T2hkp.as_str(), "t2hkp");
        assert_eq!(OpenccConfig::Hk2tp.as_str(), "hk2tp");
        assert_eq!(opencc.s2hkp("鼠标", false), "滑鼠");
        assert_eq!(opencc.hk2sp("滑鼠", false), "鼠标");
        assert_eq!(opencc.t2hkp("鼠標", false), "滑鼠");
        assert_eq!(opencc.hk2tp("滑鼠", false), "鼠標");
        assert_eq!(opencc.convert("鼠标", "s2hkp", false), "滑鼠");
        assert_eq!(opencc.convert("滑鼠", "hk2sp", false), "鼠标");
        assert_eq!(opencc.convert("鼠標", "t2hkp", false), "滑鼠");
        assert_eq!(opencc.convert("滑鼠", "hk2tp", false), "鼠標");
        assert_eq!(
            opencc.convert_with_config("鼠标", OpenccConfig::S2hkp, false),
            "滑鼠"
        );
        assert_eq!(
            opencc.convert_with_config("滑鼠", OpenccConfig::Hk2sp, false),
            "鼠标"
        );
        assert_eq!(
            opencc.convert_with_config("鼠標", OpenccConfig::T2hkp, false),
            "滑鼠"
        );
        assert_eq!(
            opencc.convert_with_config("滑鼠", OpenccConfig::Hk2tp, false),
            "鼠標"
        );
    }

    #[test]
    fn t2s_test() {
        let input = "「數大」便是美，碧綠的山坡前幾千隻綿羊，挨成一片的雪絨，是美；";
        let expected_output = "“数大”便是美，碧绿的山坡前几千只绵羊，挨成一片的雪绒，是美；";
        let opencc = OpenCC::new();
        let actual_output = opencc.t2s(input, true);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn t2jp_test() {
        let input = "舊字體：廣國，讀賣。";
        let expected_output = "旧字体：広国，読売。";
        let opencc = OpenCC::new();
        let actual_output = opencc.t2jp(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn jp2t_test() {
        let input = "広国，読売。";
        let expected_output = "廣國，讀賣。";
        let opencc = OpenCC::new();
        let actual_output = opencc.jp2t(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn newly_unified_direct_apis_honor_punctuation() {
        let opencc = OpenCC::new();
        let simplified_quotes = "“測試”";
        let traditional_quotes = "「測試」";

        assert_eq!(opencc.t2tw(simplified_quotes, false), simplified_quotes);
        assert_eq!(opencc.t2tw(simplified_quotes, true), traditional_quotes);
        assert_eq!(opencc.tw2t(simplified_quotes, false), simplified_quotes);
        assert_eq!(opencc.tw2t(simplified_quotes, true), traditional_quotes);

        assert_eq!(opencc.t2hk(simplified_quotes, false), simplified_quotes);
        assert_eq!(opencc.t2hk(simplified_quotes, true), traditional_quotes);
        assert_eq!(opencc.hk2t(simplified_quotes, false), simplified_quotes);
        assert_eq!(opencc.hk2t(simplified_quotes, true), traditional_quotes);

        assert_eq!(opencc.t2jp(simplified_quotes, false), simplified_quotes);
        assert_eq!(opencc.t2jp(simplified_quotes, true), traditional_quotes);
        assert_eq!(opencc.jp2t(simplified_quotes, false), simplified_quotes);
        assert_eq!(opencc.jp2t(simplified_quotes, true), traditional_quotes);
    }

    #[test]
    fn test_jieba_cut() {
        let input = "「數大」便是美，碧綠的山坡前幾千隻綿羊，挨成一片的雪絨，是美；";
        let expected_output = "「/ 數大/ 」/ 便是/ 美/ ，/ 碧綠/ 的/ 山坡/ 前/ 幾千隻/ 綿羊/ ，/ 挨成/ 一片/ 的/ 雪絨/ ，/ 是/ 美/ ；";
        let opencc = OpenCC::new();
        let actual_output = opencc.jieba_cut(input, true).join("/ ");
        println!("{}", actual_output);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_jieba_cut_for_search() {
        let input = "「數大」便是美，碧綠的山坡前幾千隻綿羊，挨成一片的雪絨，是美；";
        let expected_output =
            "「/ 數大/ 」/ 便是/ 美/ ，/ 碧綠/ 的/ 山坡/ 前/ 幾千/ 千隻/ 幾千隻/ 綿羊/ ，/ 挨成/ 一片/ 的/ 雪絨/ ，/ 是/ 美/ ；";

        let opencc = OpenCC::new();
        let actual_output = opencc.jieba_cut_for_search(input, true).join("/ ");
        println!("{}", actual_output);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_jieba_cut_all() {
        let input = "「數大」便是美，碧綠的山坡前幾千隻綿羊，挨成一片的雪絨，是美；";
        let expected_output =
            "「/ 數/ 大/ 」/ 便/ 便是/ 是/ 美/ ，/ 碧/ 碧綠/ 綠/ 的/ 山/ 山坡/ 坡/ 前/ 幾/ 幾千/ 幾千隻/ 千/ 千隻/ 綿/ 綿羊/ 羊/ ，/ 挨/ 成/ 一/ 一片/ 片/ 的/ 雪/ 絨/ ，/ 是/ 美/ ；";

        let opencc = OpenCC::new();
        let actual_output = opencc.jieba_cut_all(input).join("/ ");
        println!("{}", actual_output);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_jieba_tag_hant() {
        let opencc = OpenCC::new();

        let text = "我喜歡學習Rust語言";
        let tagged = opencc.jieba_tag(text, true);

        println!("Input: {text}");
        println!("POS tagging result:");

        for (word, tag) in &tagged {
            println!("{word} / {tag}");
        }

        assert!(!tagged.is_empty());

        // sanity checks
        assert!(tagged.iter().any(|(w, _)| w == "我"));
        assert!(tagged.iter().any(|(w, _)| w == "喜歡"));
        assert!(tagged.iter().any(|(w, _)| w == "學習"));
        assert!(tagged.iter().any(|(w, _)| w == "Rust"));
        assert!(tagged.iter().any(|(w, _)| w == "語言"));
    }

    #[test]
    fn s2t_punct_test() {
        let input = "你好，世界！“龙马精神”！";
        let expected_output = "你好，世界！「龍馬精神」！";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2t(input, true);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_zho_check() {
        let input = "你好，世界！龙马精神！";
        let expected_output = 2;
        let opencc = OpenCC::new();
        let actual_output = opencc.zho_check(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_keyword_extract_textrank() {
        let input = include_str!("data/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_extract_textrank(input, 10);
        println!("TextRank: {:?}", output);
    }

    #[test]
    fn test_keyword_extract_textrank_pos() {
        let input = include_str!("data/OneDay.txt");
        let opencc = OpenCC::new();

        // Common content POS (nouns + verbs)
        let allowed_pos = ["n", "nr", "ns", "nt", "v"];

        let output = opencc.keyword_extract_textrank_pos(input, 10, Some(&allowed_pos));

        println!("TextRank (POS filtered): {:?}", output);

        // Basic sanity checks
        assert!(!output.is_empty());
        assert!(output.len() <= 10);
    }

    #[test]
    fn test_keyword_extract_textrank_pos_vs_all() {
        let input = include_str!("data/OneDay.txt");
        let opencc = OpenCC::new();

        let all = opencc.keyword_extract_textrank(input, 10);

        let filtered =
            opencc.keyword_extract_textrank_pos(input, 10, Some(&["n", "nr", "ns", "nt", "v"]));

        println!("All:      {:?}", all);
        println!("Filtered: {:?}", filtered);

        assert!(!filtered.is_empty());
        assert!(filtered.len() <= 10);

        // Usually filtered result differs (not guaranteed, but often true)
        assert_ne!(all, filtered);
    }

    #[test]
    fn test_keyword_weight_textrank() {
        let input = include_str!("data/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_weight_textrank(input, 10);
        println!("TextRank: {:?}", output);
    }

    #[test]
    fn test_keyword_extract_tfidf() {
        let input = include_str!("data/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_extract_tfidf(input, 10);
        println!("TF-IDF: {:?}", output);
    }

    #[test]
    fn test_keyword_extract_tfidf_pos() {
        let input = include_str!("data/OneDay.txt");
        let opencc = OpenCC::new();

        // Common content POS (nouns + verbs)
        let allowed_pos = ["n", "nr", "ns", "nt", "v"];

        let output = opencc.keyword_extract_tfidf_pos(input, 10, Some(&allowed_pos));

        println!("TF-IDF (POS filtered): {:?}", output);

        // Basic sanity checks
        assert!(!output.is_empty());
        assert!(output.len() <= 10);
    }

    #[test]
    fn test_keyword_extract_tfidf_pos_vs_all() {
        let input = include_str!("data/OneDay.txt");
        let opencc = OpenCC::new();

        let all = opencc.keyword_extract_tfidf(input, 10);

        let filtered =
            opencc.keyword_extract_tfidf_pos(input, 10, Some(&["n", "nr", "ns", "nt", "v"]));

        println!("All:      {:?}", all);
        println!("Filtered: {:?}", filtered);

        assert!(!filtered.is_empty());
        assert!(filtered.len() <= 10);

        // Usually filtered result differs (not guaranteed, but often true)
        assert_ne!(all, filtered);
    }

    #[test]
    fn test_keyword_weight_tfidf() {
        let input = include_str!("data/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_weight_tfidf(input, 10);
        println!("TF-IDF: {:?}", output);
    }

    #[test]
    fn test_add_word() {
        let input = "云计算";

        let mut opencc = OpenCC::new();

        let jieba = Arc::get_mut(&mut opencc.jieba).expect("Jieba instance is shared");

        jieba.add_word("云计算", Some(3), Some("n"));

        let result = jieba.cut(input, false);
        println!("{:?}", result);

        assert!(result.contains(&"云计算"));
    }

    #[test]
    fn test_user_dict_loading() {
        let input = "云计算和区块链技术在帕兰提尔学习";

        let opencc = OpenCC::try_new_with_user_dict_path("tests/dicts/user_dict.txt")
            .unwrap_or_else(|e| panic!("failed to initialize OpenCC with user dictionary: {e}"));

        let result = opencc.jieba.cut(input, false);

        println!("{:?}", result);

        assert_eq!(
            result,
            vec!["云计算", "和", "区块链", "技术", "在", "帕兰提尔", "学习"]
        );
    }

    #[test]
    fn test_load_multiple_user_dicts() {
        let input = "OpenAI和ChatGPT正在研究云计算和区块链技术";

        let mut opencc = OpenCC::new();

        opencc
            .load_user_dict("tests/dicts/user_dict.txt")
            .unwrap_or_else(|e| panic!("failed to load first user dictionary: {e}"));

        opencc
            .load_user_dict("tests/dicts/user_dict2.txt")
            .unwrap_or_else(|e| panic!("failed to load second user dictionary: {e}"));

        let result = opencc.jieba.cut(input, false);

        println!("{:?}", result);

        assert!(result.contains(&"云计算"));
        assert!(result.contains(&"区块链"));
        assert!(result.contains(&"OpenAI"));
        assert!(result.contains(&"ChatGPT"));

        assert_eq!(
            result,
            vec![
                "OpenAI",
                "和",
                "ChatGPT",
                "正在",
                "研究",
                "云计算",
                "和",
                "区块链",
                "技术"
            ]
        );
    }

    #[test]
    fn load_user_dict_entries_adds_words() {
        let mut opencc = OpenCC::new();

        let entries = [UserDictEntry {
            word: "火星詞".to_string(),
            freq: 100_000,
            tag: Some("n".to_string()),
        }];

        opencc.load_user_dict_entries(&entries).unwrap();

        assert_eq!(opencc.jieba_cut("火星詞", false), vec!["火星詞"]);
    }

    #[test]
    fn load_user_dict_entries_compose_across_calls() {
        let mut opencc = OpenCC::new();

        opencc
            .load_user_dict_entries(&[UserDictEntry {
                word: "火星詞".to_string(),
                freq: 100_000,
                tag: Some("n".to_string()),
            }])
            .unwrap();

        opencc
            .load_user_dict_entries(&[UserDictEntry {
                word: "木星詞".to_string(),
                freq: 100_000,
                tag: Some("n".to_string()),
            }])
            .unwrap();

        assert_eq!(opencc.jieba_cut("火星詞", false), vec!["火星詞"]);
        assert_eq!(opencc.jieba_cut("木星詞", false), vec!["木星詞"]);
    }

    #[test]
    fn load_user_dict_entries_rejects_empty_word() {
        let mut opencc = OpenCC::new();

        let result = opencc.load_user_dict_entries(&[UserDictEntry {
            word: String::new(),
            freq: 100_000,
            tag: None,
        }]);

        assert!(matches!(result, Err(OpenccError::UserDictParse(_))));
    }

    #[test]
    fn new_with_user_dict_entries_adds_words() {
        let entries = [UserDictEntry {
            word: "火星詞".to_string(),
            freq: 100_000,
            tag: Some("n".to_string()),
        }];

        let opencc = OpenCC::try_new_with_user_dict_entries(&entries).unwrap();

        assert_eq!(opencc.jieba_cut("火星詞", false), vec!["火星詞"]);
    }

    #[test]
    fn new_with_empty_user_dict_entries_succeeds() {
        let opencc = OpenCC::try_new_with_user_dict_entries(&[]).unwrap();

        assert_eq!(opencc.s2t("汉字", false), "漢字");
    }

    // Norm Compat Tests

    #[test]
    fn normalize_compat_golden() {
        let cc = OpenCC::new();

        assert_eq!(
            cc.normalize_compat("天龍八部書裡的喬峰是契丹人"),
            "天龍八部書裡的喬峰是契丹人"
        );
    }

    #[test]
    fn opencc_normalize_compat_then_convert_golden() {
        let cc = OpenCC::new();

        let normalized = cc.normalize_compat("天龍八部書裡的喬峰是契丹人");

        assert_eq!(normalized, "天龍八部書裡的喬峰是契丹人");

        assert_eq!(
            cc.convert(&normalized, "t2s", false),
            "天龙八部书里的乔峰是契丹人"
        );
    }

    #[test]
    fn opencc_normalize_unicode_compat_golden() {
        let cc = OpenCC::new();

        // Curated Unicode compatibility normalization only.
        assert_eq!(cc.normalize_unicode_compat("聼"), "聽");

        // CJK Compatibility Ideographs are intentionally not handled here.
        assert_eq!(cc.normalize_unicode_compat("金"), "金");
    }

    #[test]
    fn opencc_normalize_compat_extended_golden() {
        let cc = OpenCC::new();

        // Combines CJK Compatibility Ideographs with the curated
        // Unicode_Compatibility.txt mappings.
        assert_eq!(
            cc.normalize_compat_extended("天龍八部書裡的聼眾"),
            "天龍八部書裡的聽眾"
        );
    }

    #[test]
    fn opencc_normalize_compat_extended_then_convert_golden() {
        let cc = OpenCC::new();

        let normalized = cc.normalize_compat_extended("天龍八部書裡的聼眾");

        assert_eq!(normalized, "天龍八部書裡的聽眾");

        assert_eq!(cc.convert(&normalized, "t2s", false), "天龙八部书里的听众");
    }

    #[test]
    fn normalize_compat_extended_is_superset_of_basic_compat_golden() {
        let cc = OpenCC::new();

        let input = "天龍八部書裡的喬峰是契丹人";

        assert_eq!(
            cc.normalize_compat_extended(input),
            cc.normalize_compat(input)
        );
    }

    #[test]
    fn opencc_normalize_compat_extended_variant_forms_golden() {
        let cc = OpenCC::new();

        let input = "聼聼竒羙⽟䂖甁噐⾳";
        let normalized = cc.normalize_compat_extended(input);

        assert_eq!(normalized, "聽聽奇美玉石瓶器音");
        assert_eq!(cc.convert(&normalized, "t2s", false), "听听奇美玉石瓶器音");
    }

    #[test]
    fn opencc_normalize_compat_extended_improves_jieba_segmentation() {
        let cc = OpenCC::new();

        let input = "聼聼竒羙⽟䂖甁噐⾳";
        let normalized = cc.normalize_compat_extended(input);

        assert_eq!(normalized, "聽聽奇美玉石瓶器音");

        // Compatibility/variant forms are mostly unknown to Jieba,
        // so the raw input falls back to character-level segmentation.
        assert_eq!(
            cc.jieba_cut(input, true),
            vec!["聼", "聼", "竒", "羙", "⽟", "䂖", "甁", "噐", "⾳"]
        );

        // After extended compatibility normalization, Jieba can recognize
        // the canonical forms using its normal dictionary.
        assert_eq!(
            cc.jieba_cut(&normalized, true),
            vec!["聽聽", "奇美", "玉石", "瓶器音"]
        );
    }

    // DeTofu Tests

    #[test]
    fn test_opencc_detofu() {
        let cc = OpenCC::new();
        let input = "𠉂𪠟𫝈𫬐";

        assert_eq!(cc.detofu(input, DetofuLevel::ExtE), "𠉂𪠟𫝈㘔");
        assert_eq!(cc.detofu(input, DetofuLevel::ExtB), "㒓㓄㑮㘔");
    }

    #[test]
    fn test_opencc_t2s_detofu() {
        let cc = OpenCC::new();

        let output = cc.detofu(
            &cc.convert("儼驂騑於上路，訪風景於崇阿", "t2s", false),
            DetofuLevel::ExtB,
        );

        assert_eq!(output, "俨骖騑于上路，访风景于崇阿");
    }

    #[test]
    fn test_opencc_t2s_detofu_into() {
        let cc = OpenCC::new();
        let mut output = String::new();

        cc.detofu_into(
            &cc.convert("儼驂騑於上路，訪風景於崇阿", "t2s", false),
            DetofuLevel::ExtB,
            &mut output,
        );

        assert_eq!(output, "俨骖騑于上路，访风景于崇阿");
    }

    #[test]
    fn test_opencc_t2s_detofu_preserves_unmapped_character() {
        let cc = OpenCC::new();

        let converted = cc.convert("儼驂騑於上路，訪風景於崇阿，𱁬", "t2s", false);

        let output = cc.detofu(&converted, DetofuLevel::ExtB);

        assert_eq!(output, "俨骖騑于上路，访风景于崇阿，𱁬");
    }

    #[test]
    fn test_detofu_custom_pairs_override_builtin_mapping() {
        let input = "這隻小狗有𣭲毛";

        assert_eq!(
            DetofuMap::builtin(DetofuLevel::ExtB).detofu(input),
            "這隻小狗有氄毛"
        );

        let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𣭲', '氂')]);

        assert_eq!(map.detofu(input), "這隻小狗有氂毛");
    }

    #[test]
    fn detofu_with_custom_file_loads_user_mapping() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        path.push(format!("opencc_fmmseg_custom_tofu_{unique}.txt"));

        fs::write(&path, "𣭲\t氄\tB\n").unwrap();

        let cc = OpenCC::new();
        let result = cc
            .detofu_with_custom_file("𣭲毛", DetofuLevel::ExtB, &path)
            .unwrap();

        fs::remove_file(&path).ok();

        assert_eq!(result, "氄毛");
    }

    #[test]
    fn test_detofu_map_with_custom_pairs() {
        let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𣭲', '氄')]);

        assert_eq!(map.detofu("𣭲毛"), "氄毛");
    }

    #[test]
    fn test_detofu_map_with_custom_pairs_overrides_builtin() {
        let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𬴂', '騑')]);

        assert_eq!(map.detofu("骖𬴂"), "骖騑");
    }

    #[test]
    fn test_opencc_detofu_with_custom_pairs() {
        let cc = OpenCC::new();

        let output = cc.detofu_with_custom_pairs(
            "𣭲毛 骖𬴂",
            DetofuLevel::ExtB,
            &[('𣭲', '氄'), ('𬴂', '騑')],
        );

        assert_eq!(output, "氄毛 骖騑");
    }

    #[test]
    fn test_detofu_custom_pairs_later_wins() {
        let map =
            DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𣭲', '氂'), ('𣭲', '氄')]);

        assert_eq!(map.detofu("𣭲毛"), "氄毛");
    }

    #[test]
    fn test_detofu_custom_pairs_support_bmp_characters() {
        let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('A', 'B')]);

        assert_eq!(map.detofu("A𬴂"), "B騑");
    }

    #[test]
    fn builtin_detofu_replaces_known_st_mappings() {
        let map = DetofuMap::builtin(DetofuLevel::ExtB);

        assert_eq!(map.detofu("𠗣𧜗"), "㓆䘞");
    }

    #[test]
    fn test_opencc_s2t_detofu() {
        let cc = OpenCC::new();

        let converted = cc.convert("㓆䘞", "s2t", false);
        assert_eq!(converted, "𠗣𧜗");

        assert_eq!(cc.detofu(&converted, DetofuLevel::ExtB), "㓆䘞");
    }
}
