use opencc_jieba_rs::{OpenCC, OpenccConfig};

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(opencc.t2hkp("鼠標"), "滑鼠");
        assert_eq!(opencc.hk2tp("滑鼠"), "鼠標");
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
        let actual_output = opencc.t2jp(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn jp2t_test() {
        let input = "広国，読売。";
        let expected_output = "廣國，讀賣。";
        let opencc = OpenCC::new();
        let actual_output = opencc.jp2t(input);
        assert_eq!(actual_output, expected_output);
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
        let input = include_str!("../src/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_extract_textrank(input, 10);
        println!("TextRank: {:?}", output);
    }

    #[test]
    fn test_keyword_extract_textrank_pos() {
        let input = include_str!("../src/OneDay.txt");
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
        let input = include_str!("../src/OneDay.txt");
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
        let input = include_str!("../src/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_weight_textrank(input, 10);
        println!("TextRank: {:?}", output);
    }

    #[test]
    fn test_keyword_extract_tfidf() {
        let input = include_str!("../src/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_extract_tfidf(input, 10);
        println!("TF-IDF: {:?}", output);
    }

    #[test]
    fn test_keyword_extract_tfidf_pos() {
        let input = include_str!("../src/OneDay.txt");
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
        let input = include_str!("../src/OneDay.txt");
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
        let input = include_str!("../src/OneDay.txt");
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
}
