//! Core APIs remain usable when keyword extraction is excluded.
#![cfg(not(feature = "keywords"))]

use opencc_jieba_rs::OpenCC;

#[test]
fn conversion_segmentation_and_pos_without_keywords() {
    let mut cc = OpenCC::new();
    assert_eq!(cc.s2t("汉字", false), "漢字");
    cc.add_word("星际量子茶壶", Some(100_000), Some("n"))
        .unwrap();
    assert_eq!(cc.jieba_cut("星际量子茶壶", false), vec!["星际量子茶壶"]);
    assert_eq!(
        cc.jieba_tag("星际量子茶壶", false),
        vec![("星际量子茶壶".into(), "n".into())]
    );
}
