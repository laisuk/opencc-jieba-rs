use super::*;
use crate::jieba_rs::Keyword;
use std::io::Cursor;
use std::sync::Barrier;

const TEXTS: [&str; 4] = [
    "南京市长江大桥旁，研究人员正在开发自然语言处理和机器学习软件。小明硕士毕业于中国科学院计算所。",
    "臺北市的研究團隊正在開發自然語言處理與機器學習軟體。香港銀行提供金融服務，使用者關注資料安全。",
    "OpenAI與研究团队使用Rust 1.75开发AI軟體，2026年在臺北和北京测试API-v2，准确率98.5%。𠀀🙂",
    "杭研硕士小明云端量子茶壶龘靐齉爩",
];

fn bits(keywords: Vec<Keyword>) -> Vec<(String, u64, String)> {
    keywords
        .into_iter()
        .map(|k| (k.keyword, k.weight.to_bits(), k.tag))
        .collect()
}

fn builtin_text() -> String {
    String::from_utf8(crate::zstd::decompress(include_bytes!("../data/idf.txt.zst")).unwrap())
        .unwrap()
}

fn custom(text: &str) -> TfIdf {
    TfIdf::new(
        Some(&mut Cursor::new(text)),
        KeywordExtractConfig::default(),
    )
}

fn small_jieba() -> Jieba {
    Jieba::with_dict(&mut Cursor::new(
        "alpha 100 n\nbeta 100 v\ngamma 100 n\ndelta 100 nz\n",
    ))
    .unwrap()
}

// Frozen result fingerprints are captured from 21d67d4 before changing its
// implementation. Include order, word/tag bytes, and exact f64 weight bits.
fn fingerprint(items: &[(String, u64, String)]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for (word, weight, tag) in items {
        for byte in (word.len() as u64)
            .to_le_bytes()
            .into_iter()
            .chain(word.bytes())
            .chain(weight.to_le_bytes())
            .chain((tag.len() as u64).to_le_bytes())
            .chain(tag.bytes())
        {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    hash
}

#[test]
fn builtin_matches_independently_loaded_idf_and_baseline_results() {
    let jieba = Jieba::new();
    let mut shared = TfIdf::default();
    // Baseline's BufRead loading path independently parses the same bytes.
    let mut independent = custom(&builtin_text());
    assert_eq!(
        shared.data.median_idf.to_bits(),
        independent.data.median_idf.to_bits()
    );
    assert_eq!(shared.data.idf_dict.len(), independent.data.idf_dict.len());
    for (word, value) in shared.data.idf_dict.iter() {
        assert_eq!(
            value.to_bits(),
            independent.data.idf_dict[word].to_bits(),
            "{word}"
        );
    }
    let mut snapshots = Vec::new();
    for hmm in [false, true] {
        *shared.config_mut() = KeywordExtractConfig::builder().use_hmm(hmm).build();
        *independent.config_mut() = shared.config().clone();
        for text in TEXTS {
            for top_k in [0, 1, 5, 100] {
                for allowed in [
                    vec![],
                    vec!["n".to_owned(), "v".to_owned(), "eng".to_owned()],
                ] {
                    let expected =
                        bits(independent.extract_keywords(&jieba, text, top_k, allowed.clone()));
                    let actual = bits(shared.extract_keywords(&jieba, text, top_k, allowed));
                    assert_eq!(actual, expected);
                    snapshots.push(fingerprint(&actual));
                }
            }
        }
    }
    assert_eq!(
        snapshots,
        vec![
            14695981039346656037,
            14695981039346656037,
            6477159600997857382,
            1457761374937938900,
            11289714792899811600,
            8018754530098106330,
            4937961171344006089,
            13465046762909509050,
            14695981039346656037,
            14695981039346656037,
            14994981703853690624,
            1317971975628895999,
            14498102320680334617,
            17348593776917521155,
            10922536898959028519,
            3165218383700049852,
            14695981039346656037,
            14695981039346656037,
            1710473771339932909,
            17400080400368549419,
            6744908069997980697,
            18280821400989701616,
            13761940663983229679,
            1015211730555719324,
            14695981039346656037,
            14695981039346656037,
            2802850097350524637,
            2802850097350524637,
            5401311377119180119,
            5401311377119180119,
            5401311377119180119,
            5401311377119180119,
            14695981039346656037,
            14695981039346656037,
            1412146477852310886,
            1457761374937938900,
            2981332398544718336,
            8018754530098106330,
            15059695833183440938,
            13465046762909509050,
            14695981039346656037,
            14695981039346656037,
            14994981703853690624,
            1317971975628895999,
            14498102320680334617,
            17348593776917521155,
            10922536898959028519,
            3165218383700049852,
            14695981039346656037,
            14695981039346656037,
            2249975208559882613,
            4441786782217203852,
            10069700636516515018,
            16397396207126959871,
            12771588009793017259,
            132578651099423752,
            14695981039346656037,
            14695981039346656037,
            9603788837310650171,
            2802850097350524637,
            5558443323799925671,
            5401311377119180119,
            16044829514788345667,
            5401311377119180119
        ]
    );
}

#[test]
fn custom_duplicates_lower_median_ties_and_merge_semantics() {
    let jieba = small_jieba();
    let mut extractor = custom("alpha 2\nbeta 8\nalpha 10\ngamma 4\nignored\nbad invalid\n");
    assert_eq!(extractor.data.median_idf, 4.0); // lower median of [2,8,10,4], duplicates included
    assert_eq!(extractor.data.idf_dict.len(), 3);
    assert_eq!(
        bits(extractor.extract_keywords(&jieba, "alpha beta gamma delta alpha", 100, vec![])),
        vec![
            ("alpha".into(), 4.0f64.to_bits(), "n".into()),
            ("beta".into(), 1.6f64.to_bits(), "v".into()),
            ("delta".into(), 0.8f64.to_bits(), "nz".into()),
            ("gamma".into(), 0.8f64.to_bits(), "n".into()),
        ]
    );
    extractor
        .load_dict(&mut Cursor::new("beta 12\nbeta 6\ndelta 14\n"))
        .unwrap();
    assert_eq!(extractor.data.median_idf, 12.0); // median of this load, NOT the merged map
    assert_eq!(extractor.data.idf_dict["beta"], 6.0);
    assert_eq!(extractor.data.idf_dict["alpha"], 10.0);
    assert_eq!(extractor.data.idf_dict["delta"], 14.0);
    assert_eq!(
        extractor.extract_keywords(&jieba, "alpha beta gamma delta", 0, vec![]),
        vec![]
    );
}

#[test]
fn custom_nonfinite_values_and_signed_zero_keep_their_bits() {
    let extractor = custom("neg -inf\nzero -0.0\npos inf\nnan NaN\n");
    assert_eq!(extractor.data.median_idf.to_bits(), (-0.0f64).to_bits());
    assert_eq!(
        extractor.data.idf_dict["zero"].to_bits(),
        (-0.0f64).to_bits()
    );
    assert_eq!(extractor.data.idf_dict["neg"], f64::NEG_INFINITY);
    assert_eq!(extractor.data.idf_dict["pos"], f64::INFINITY);
    assert!(extractor.data.idf_dict["nan"].is_nan());
}

#[test]
fn default_custom_load_and_configuration_do_not_affect_other_extractors() {
    let jieba = small_jieba();
    let untouched = TfIdf::default();
    let before = bits(untouched.extract_keywords(&jieba, "alpha beta gamma delta", 10, vec![]));
    let mut changed = TfIdf::default();
    changed
        .load_dict(&mut Cursor::new("alpha 99\nalpha 7\nbeta 15\n"))
        .unwrap();
    assert_eq!(changed.data.idf_dict["alpha"], 7.0);
    assert_eq!(changed.data.median_idf, 15.0);
    *changed.config_mut() = KeywordExtractConfig::builder()
        .add_stop_word("alpha")
        .build();
    assert!(changed
        .extract_keywords(&jieba, "alpha beta gamma delta", 10, vec![])
        .iter()
        .all(|k| k.keyword != "alpha"));
    changed.load_dict(&mut Cursor::new("beta 3\n")).unwrap();
    assert_eq!(changed.data.median_idf, 3.0);
    assert_eq!(changed.data.idf_dict["alpha"], 7.0);
    assert_eq!(
        before,
        bits(untouched.extract_keywords(&jieba, "alpha beta gamma delta", 10, vec![]))
    );
    assert_eq!(
        before,
        bits(TfIdf::default().extract_keywords(&jieba, "alpha beta gamma delta", 10, vec![]))
    );
    let empty = TfIdf::new(None::<&mut io::Empty>, KeywordExtractConfig::default());
    assert!(empty.data.idf_dict.is_empty());
    assert_eq!(empty.data.median_idf, 0.0);
}

#[test]
fn default_tables_are_shared_and_only_the_customized_extractor_detaches() {
    let first = TfIdf::default();
    let mut second = TfIdf::default();
    assert!(std::ptr::eq(first.data.as_ref(), second.data.as_ref()));
    assert!(matches!(second.data, Cow::Borrowed(_)));
    *second.config_mut() = KeywordExtractConfig::builder()
        .min_keyword_length(3)
        .build();
    assert!(std::ptr::eq(first.data.as_ref(), second.data.as_ref()));
    assert_eq!(first.config().min_keyword_length(), 2);
    second
        .load_dict(&mut Cursor::new("phase2a_custom 8\n"))
        .unwrap();
    assert!(matches!(second.data, Cow::Owned(_)));
    assert!(!first.data.idf_dict.contains_key("phase2a_custom"));
    let key_ptr = second
        .data
        .idf_dict
        .keys()
        .find(|key| key.as_str() == "phase2a_custom")
        .unwrap()
        .as_ptr();
    second
        .load_dict(&mut Cursor::new("phase2a_custom 9\n"))
        .unwrap();
    assert_eq!(
        key_ptr,
        second
            .data
            .idf_dict
            .keys()
            .find(|key| key.as_str() == "phase2a_custom")
            .unwrap()
            .as_ptr()
    );
    assert_eq!(second.data.median_idf, 9.0);
    assert!(std::ptr::eq(
        first.data.as_ref(),
        TfIdf::default().data.as_ref()
    ));
}

struct FailingReader(Cursor<&'static [u8]>);
impl io::Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        unreachable!()
    }
}
impl BufRead for FailingReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.0.position() as usize == self.0.get_ref().len() {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "injected read failure",
            ))
        } else {
            self.0.fill_buf()
        }
    }
    fn consume(&mut self, amount: usize) {
        self.0.consume(amount);
    }
}

#[test]
fn failed_and_empty_loads_preserve_baseline_partial_update_and_panic_behavior() {
    for mut extractor in [custom("alpha 2\nbeta 4\n"), TfIdf::default()] {
        let median = extractor.data.median_idf.to_bits();
        let error = extractor
            .load_dict(&mut FailingReader(Cursor::new(b"alpha 99\n")))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(extractor.data.idf_dict["alpha"], 99.0); // loading was not transactional
        assert_eq!(extractor.data.median_idf.to_bits(), median);
        for input in ["", "ignored\ninvalid not-a-float\n"] {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                extractor.load_dict(&mut Cursor::new(input)).unwrap();
            }));
            assert!(result.is_err()); // preserve baseline empty median selection panic
            assert_eq!(extractor.data.median_idf.to_bits(), median);
            assert_eq!(extractor.data.idf_dict["alpha"], 99.0);
        }
    }
}

#[test]
fn default_initialization_and_repeated_extraction_are_thread_safe() {
    let jieba = small_jieba();
    let barrier = Barrier::new(8);
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for _ in 0..8 {
            let jieba = &jieba;
            let barrier = &barrier;
            threads.push(scope.spawn(move || {
                barrier.wait();
                let extractor = TfIdf::default();
                let first =
                    bits(extractor.extract_keywords(jieba, "alpha beta gamma delta", 10, vec![]));
                for _ in 0..8 {
                    assert_eq!(
                        first,
                        bits(extractor.extract_keywords(
                            jieba,
                            "alpha beta gamma delta",
                            10,
                            vec![]
                        ))
                    );
                }
                first
            }));
        }
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert!(results.windows(2).all(|w| w[0] == w[1]));
    });
    // Also share a single immutable extractor, not just its default table.
    let extractor = TfIdf::default();
    let expected = bits(extractor.extract_keywords(&jieba, "alpha beta gamma delta", 10, vec![]));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let extractor = &extractor;
            let jieba = &jieba;
            let expected = &expected;
            scope.spawn(move || {
                for _ in 0..8 {
                    assert_eq!(
                        &bits(extractor.extract_keywords(
                            jieba,
                            "alpha beta gamma delta",
                            10,
                            vec![]
                        )),
                        expected
                    );
                }
            });
        }
    });
}

#[test]
fn concurrent_public_calls_across_instances_match_exactly() {
    let first = crate::OpenCC::new();
    let second = crate::OpenCC::new();
    let expected = bits(first.keyword_weight_tfidf(TEXTS[2], 10));
    let expected_words: Vec<_> = expected.iter().map(|(word, _, _)| word.clone()).collect();
    let expected_filtered = bits(first.keyword_weight_tfidf_pos(TEXTS[2], 10, Some(&["n", "v"])));
    std::thread::scope(|scope| {
        for cc in [&first, &first, &second, &second] {
            let expected = &expected;
            let expected_words = &expected_words;
            let expected_filtered = &expected_filtered;
            scope.spawn(move || {
                for _ in 0..3 {
                    assert_eq!(&bits(cc.keyword_weight_tfidf(TEXTS[2], 10)), expected);
                    assert_eq!(&cc.keyword_extract_tfidf(TEXTS[2], 10), expected_words);
                    assert_eq!(
                        &bits(cc.keyword_weight_tfidf_pos(TEXTS[2], 10, Some(&["n", "v"]))),
                        expected_filtered
                    );
                }
            });
        }
    });
}
