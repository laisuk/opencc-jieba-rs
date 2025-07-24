#[cfg(test)]
mod tests {
    use opencc_jieba_rs::OpenCC;

    #[test]
    fn test_split_inclusive_refined() {
        let instance = OpenCC::new();
        let text_cjk = "你好，世界！";
        // "你好，" (0..9), "世界！" (9..18)
        assert_eq!(
            instance.split_string_ranges(text_cjk, true),
            vec![0..9, 9..18]
        );

        let text_mixed = "Hello,World!Rust.";
        // "Hello," (0..6), "World!" (6..12), "Rust." (12..17)
        assert_eq!(
            instance.split_string_ranges(text_mixed, true),
            vec![0..6, 6..12, 12..17]
        );

        let text_no_delimiter = "HelloWorld";
        assert_eq!(
            instance.split_string_ranges(text_no_delimiter, true),
            vec![0..10]
        );

        let text_starts_with_delimiter = "，Hello"; // Assuming '，' is 3 bytes
        assert_eq!(
            instance.split_string_ranges(text_starts_with_delimiter, true),
            vec![0..3, 3..8]
        ); // "，Hello"
    }

    #[test]
    fn test_split_exclusive_refined() {
        let instance = OpenCC::new();
        let text_cjk = "你好，世界！";
        // "你好" (0..6), "，" (6..9), "世界" (9..15), "！" (15..18)
        assert_eq!(
            instance.split_string_ranges(text_cjk, false),
            vec![0..6, 6..9, 9..15, 15..18]
        );

        let text_mixed = "Hello,World!Rust.";
        // "Hello" (0..5), "," (5..6), "World" (6..11), "!" (11..12), "Rust" (12..16), "." (16..17)
        assert_eq!(
            instance.split_string_ranges(text_mixed, false),
            vec![0..5, 5..6, 6..11, 11..12, 12..16, 16..17]
        );

        let text_no_delimiter = "HelloWorld";
        assert_eq!(
            instance.split_string_ranges(text_no_delimiter, false),
            vec![0..10]
        );

        let text_starts_with_delimiter = "，Hello";
        // "，" (0..3), "Hello" (3..8)
        assert_eq!(
            instance.split_string_ranges(text_starts_with_delimiter, false),
            vec![0..3, 3..8]
        );

        let text_consecutive_delimiters = "Hello,,World";
        // "Hello" (0..5), "," (5..6), "," (6..7), "World" (7..12)
        assert_eq!(
            instance.split_string_ranges(text_consecutive_delimiters, false),
            vec![0..5, 5..6, 6..7, 7..12]
        );

        let text_trailing_delimiter = "Hello!";
        // "Hello" (0..5), "!" (5..6)
        assert_eq!(
            instance.split_string_ranges(text_trailing_delimiter, false),
            vec![0..5, 5..6]
        );

        let text_only_delimiters = ",,,";
        // "," (0..1), "," (1..2), "," (2..3)
        assert_eq!(
            instance.split_string_ranges(text_only_delimiters, false),
            vec![0..1, 1..2, 2..3]
        );
    }
}
