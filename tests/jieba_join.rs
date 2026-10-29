use opencc_jieba_rs::OpenCC;

fn check(cc: &OpenCC, input: &str) {
    for hmm in [false, true] {
        let tokens = cc.jieba_cut(input, hmm);
        for delimiter in [" ", "/", "|", "", "🙂分隔"] {
            assert_eq!(
                cc.jieba_cut_and_join(input, hmm, delimiter),
                tokens.join(delimiter),
                "bytes={}, hmm={hmm}, delimiter={delimiter:?}",
                input.len()
            );
        }
    }
}

#[test]
fn join_matches_flattened_tokens() {
    let cc = OpenCC::new();
    for input in [
        "",
        " ",
        "\r\n\t",
        "，。！？",
        "南京市长江大桥",
        "  南京，\n\n北京！ ",
        "Rust API-v2 98.5% 𠀀🙂\0杭研硕士小明",
        "a|b/c",
    ] {
        check(&cc, input);
    }
    // Exercise both sides of the byte threshold, a single large chunk,
    // many punctuation-only chunks, and mixed Unicode chunk boundaries.
    for input in [
        "a".repeat(999),
        "a".repeat(1000),
        "研究生命起源".repeat(200),
        "， \n\t！".repeat(300),
        " 南京市长江大桥，杭研小明🙂𠀀\r\n".repeat(100),
    ] {
        check(&cc, &input);
    }
}

#[test]
fn parallel_join_preserves_order_across_thread_counts() {
    let cc = OpenCC::new();
    let input = "，南京市长江大桥！ \n杭研小明𠀀🙂".repeat(100);
    for threads in [1, 4] {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| check(&cc, &input));
    }
}

#[test]
#[ignore = "requires local benchmarks/ChangFeng_Simp.txt; run explicitly in release mode"]
fn novel_join_matches_flattened_tokens() {
    let input = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/benchmarks/ChangFeng_Simp.txt"
    ))
    .expect("provide the real-world novel benchmark file");
    check(&OpenCC::new(), &input);
}
