use opencc_jieba_rs::{dictionary_lib, format_thousand, OpenCC};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn zho_check_test() {
        let input = "你好，世界！龙马精神！";
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
        let actual_output = opencc.jieba.cut(input, true).join("/ ");
        println!("{}", actual_output);
        assert_eq!(actual_output, expected_output);
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
    fn format_thousand_test() {
        let input = 1234567890;
        let expected_output = "1,234,567,890";
        let actual_output = format_thousand(input);
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
    #[ignore]
    // In case there are new update to dictionaries contents,
    // run this test to generate new dictionary.json
    fn test_serialize_to_json() {
        // Define the filename for testing
        let filename = "dictionary.json";
        // let opencc = OpenCC::new();
        let dictionary = dictionary_lib::Dictionary::from_dicts();
        // Serialize to JSON and write to file
        dictionary.serialize_to_json(filename).unwrap();

        // Read the contents of the file
        let file_contents = fs::read_to_string(filename).unwrap();

        // Verify that the JSON contains the expected data
        let expected_json = 1351418;
        assert_eq!(file_contents.trim().len(), expected_json);

        // Clean up: Delete the test file
        // fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_keyword_extract_textrank() {
        let input = include_str!("../src/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_extract_textrank(input, 10);
        println!("TextRank: {:?}", output);
    }

    #[test]
    fn test_keyword_extract_tfidf() {
        let input = include_str!("../src/OneDay.txt");
        let opencc = OpenCC::new();
        let output = opencc.keyword_extract_tfidf(input, 10);
        println!("TF-IDF: {:?}", output);
    }
}
