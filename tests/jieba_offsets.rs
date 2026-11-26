use opencc_jieba_rs::{HmmModel, Jieba};

#[test]
fn search_connector_parts_keep_unicode_and_byte_offsets() {
    let mut jieba = Jieba::empty();
    let compound = "-甲ab__12.cd-𠀀";
    jieba.add_word(compound, Some(100), Some("nz"));
    let input = "𠀀🙂\r\n-甲ab__12.cd-𠀀";

    for hmm in [false, true] {
        let tokens: Vec<_> = jieba
            .cut_for_search(input, hmm)
            .into_iter()
            .map(|t| (t.word, t.start, t.end, t.byte_start, t.byte_end))
            .collect();
        assert_eq!(
            tokens,
            [
                ("𠀀", 0, 1, 0, 4),
                ("🙂", 1, 2, 4, 8),
                ("\r\n", 2, 4, 8, 10),
                ("甲ab", 5, 8, 11, 16),
                ("cd", 13, 15, 21, 23),
                (compound, 4, 17, 10, 28),
            ]
        );

        let tags = jieba.tag(input, hmm);
        let tagged = tags.last().unwrap();
        assert_eq!(
            (
                tagged.word,
                tagged.tag,
                tagged.start,
                tagged.end,
                tagged.byte_start,
                tagged.byte_end
            ),
            (compound, "nz", 4, 17, 10, 28)
        );
    }
}

#[test]
fn search_long_identifier_keeps_every_part_in_order() {
    let component = "Alpha-beta_42.";
    let repeats = 2048;
    let input = format!("{}omega", component.repeat(repeats));
    let jieba = Jieba::empty();
    let tokens: Vec<_> = jieba
        .cut_for_search(&input, true)
        .into_iter()
        .map(|t| (t.word, t.start, t.end, t.byte_start, t.byte_end))
        .collect();

    let mut expected = Vec::new();
    for i in 0..repeats {
        let start = i * component.len();
        expected.push(("Alpha", start, start + 5, start, start + 5));
        expected.push(("beta", start + 6, start + 10, start + 6, start + 10));
    }
    let end = input.len();
    expected.push(("omega", end - 5, end, end - 5, end));
    expected.push((input.as_str(), 0, end, 0, end));
    assert_eq!(tokens, expected);
}

#[test]
fn custom_hmm_offsets_cover_han_ascii_and_supplementary_characters() {
    let model = "\
0 -100 -100 -100
0 0 0 0
0 0 0 0
0 0 0 0
0 0 0 0
甲:0,乙:-100
甲:-100,乙:0
甲:-100,乙:-100
甲:-100,乙:-100
";
    let mut jieba = Jieba::empty();
    jieba.set_hmm_model(HmmModel::load(&mut model.as_bytes()).unwrap());
    let tokens: Vec<_> = jieba
        .cut("🙂𠀀甲乙api-v2\r\n", true)
        .into_iter()
        .map(|t| (t.word, t.start, t.end, t.byte_start, t.byte_end))
        .collect();
    assert_eq!(
        tokens,
        [
            ("🙂", 0, 1, 0, 4),
            ("𠀀", 1, 2, 4, 8),
            ("甲乙", 2, 4, 8, 14),
            ("api-v2", 4, 10, 14, 20),
            ("\r\n", 10, 12, 20, 22),
        ]
    );
}
