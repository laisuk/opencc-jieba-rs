use opencc_jieba_rs::{format_thousand, OpenCC, dictionary_lib};

#[cfg(test)]
mod tests {
    use std::fs;
    use super::*;

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
    fn serialize_to_json_test() {
        let input = "你好，世界！龙马精神！";
        let expected_output = 2;
        let opencc = OpenCC::new();
        let actual_output = opencc.zho_check(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    #[ignore]
    fn test_serialize_to_json() {
        // Define the filename for testing
        let filename = "test_dictionary.json";
        // let opencc = OpenCC::new();
        let dictionary = dictionary_lib::Dictionary::new();
        // Serialize to JSON and write to file
        dictionary.serialize_to_json(filename).unwrap();

        // Read the contents of the file
        let file_contents = fs::read_to_string(filename).unwrap();

        // Verify that the JSON contains the expected data
        let expected_json = 1350164;
        assert_eq!(file_contents.trim().len(), expected_json);

        // Clean up: Delete the test file
        fs::remove_file(filename).unwrap();
    }
}
