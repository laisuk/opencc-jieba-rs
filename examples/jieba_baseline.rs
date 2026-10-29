//! Phase-1 benchmark only. Run with Rust 1.75 in release mode; see benchmarks/jieba/README.md.
#![allow(dead_code, unused_imports)]
// This executable is not an additional copy of the private unit-test suite.
#![cfg(not(test))]

// Compile the unchanged private modules into this example, without exposing APIs
// or modifying library code. The package build.rs supplies the same HMM tables.
#[path = "../src/jieba_rs/mod.rs"]
mod jieba_rs;
#[path = "../src/rustc_hash/mod.rs"]
mod rustc_hash;
#[path = "../src/zstd/mod.rs"]
mod zstd;
mod opencc {
    pub(crate) const DICT_HANS_HANT_ZSTD: &[u8] =
        include_bytes!("../src/dictionary_lib/dicts/dict_hans_hant.txt.zst");
}

use jieba_rs::{Jieba, KeywordExtract, TextRank, TfIdf};
use opencc_jieba_rs::{OpenCC, UserDictEntry};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant};

struct CountingAllocator;
static COUNT: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
fn record(bytes: usize) {
    if COUNT.load(Relaxed) {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(bytes as u64, Relaxed);
    }
}
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        System.alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        System.alloc_zeroed(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        System.realloc(ptr, layout, size)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const SIMP: &str = "南京市长江大桥旁，研究人员正在开发自然语言处理和机器学习软件。小明硕士毕业于中国科学院计算所。";
const TRAD: &str = "臺北市的研究團隊正在開發自然語言處理與機器學習軟體。香港銀行提供金融服務，使用者關注資料安全。";
const MIXED: &str =
    "OpenAI與研究团队使用Rust 1.75开发AI軟體，2026年在臺北和北京测试API-v2，准确率98.5%。𠀀🙂";
const OOV: &str = "杭研硕士小明云端量子茶壶龘靐齉爩";
const SAMPLES: usize = 7;
const TARGET: Duration = Duration::from_millis(40);

fn allocations(mut f: impl FnMut()) -> (u64, u64) {
    ALLOCS.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    COUNT.store(true, Relaxed);
    f();
    COUNT.store(false, Relaxed);
    (ALLOCS.load(Relaxed), BYTES.load(Relaxed))
}

fn row(name: &str, bytes: usize, iterations: usize, mut ns: Vec<f64>, alloc: (u64, u64)) {
    ns.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "{name},{bytes},{iterations},{:.1},{:.1},{:.1},{},{}",
        ns[0],
        ns[ns.len() / 2],
        ns[ns.len() - 1],
        alloc.0,
        alloc.1
    );
}

fn bench(name: &str, bytes: usize, mut f: impl FnMut()) {
    // Warm once, calibrate a batch, then keep the batch size fixed across samples.
    f();
    let mut iterations = 1;
    loop {
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        if start.elapsed() >= TARGET || iterations >= 1 << 22 {
            break;
        }
        iterations *= 2;
    }
    let mut times = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        times.push(start.elapsed().as_nanos() as f64 / iterations as f64);
    }
    let alloc = allocations(&mut f);
    row(name, bytes, iterations, times, alloc);
}

// Fresh state per operation. Setup and final-state destruction are excluded;
// any clone/drop performed INSIDE the operation (e.g. transactional APIs) counts.
fn prepared<T>(name: &str, bytes: usize, mut setup: impl FnMut() -> T, mut f: impl FnMut(&mut T)) {
    let mut times = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let mut state = setup();
        let start = Instant::now();
        f(&mut state);
        times.push(start.elapsed().as_nanos() as f64);
        black_box(&state);
    }
    let mut state = setup();
    let alloc = allocations(|| f(&mut state));
    row(name, bytes, 1, times, alloc);
}

fn once(name: &str, mut f: impl FnMut()) {
    // Cold latency is timed without enabled allocation counters. Allocation
    // counts are omitted because replaying would no longer be a cold operation.
    let start = Instant::now();
    f();
    row(name, 0, 1, vec![start.elapsed().as_nanos() as f64], (0, 0));
}

fn cold() {
    let mut core = None;
    once("cold.core.new", || {
        core = Some(Jieba::new());
    });
    let core = core.unwrap();
    once("cold.core.cut_hmm", || {
        black_box(core.cut(OOV, true));
    });
    // Empty dictionary guarantees compound POS model access on Chinese OOV.
    let empty = Jieba::empty();
    once("cold.core.pos_model", || {
        black_box(empty.tag(OOV, true));
    });
    let mut tfidf = None;
    once("cold.core.tfidf_init", || {
        tfidf = Some(TfIdf::default());
    });
    once("cold.core.tfidf_extract", || {
        black_box(
            tfidf
                .as_ref()
                .unwrap()
                .extract_keywords(&core, MIXED, 10, vec![]),
        );
    });
    let mut public = None;
    once("cold.public.new", || {
        public = Some(OpenCC::new());
    });
    let public = public.unwrap();
    once("cold.public.cut", || {
        black_box(public.jieba_cut(MIXED, true));
    });
    once("cold.public.tfidf", || {
        black_box(public.keyword_weight_tfidf(MIXED, 10));
    });
    once("cold.public.textrank", || {
        black_box(public.keyword_weight_textrank(MIXED, 10));
    });
}

fn main() {
    if std::env::args().any(|arg| arg == "--cold") {
        cold();
        return;
    }
    println!("case,input_bytes,iterations_per_sample,min_ns,median_ns,max_ns,alloc_calls,allocated_bytes");
    // Separate processes reset OnceLocks, TLS and the Rayon pool. Process launch
    // itself is not timed; OS file/page caches are NOT flushed.
    for sample in 1..=5 {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--cold")
            .output()
            .unwrap();
        assert!(output.status.success(), "cold child failed");
        print!(
            "{}",
            String::from_utf8(output.stdout)
                .unwrap()
                .replace("cold.", &format!("cold{sample}."))
        );
    }

    let core = Jieba::new();
    let public = OpenCC::new();
    let tfidf = TfIdf::default();
    let textrank = TextRank::default();
    bench("init.core.new_drop", 0, || {
        black_box(Jieba::new());
    });
    bench("init.public.new_drop", 0, || {
        black_box(OpenCC::new());
    });
    bench("init.core.tfidf_drop", 0, || {
        black_box(TfIdf::default());
    });
    bench("init.core.textrank_drop", 0, || {
        black_box(TextRank::default());
    });
    bench("init.core.clone_drop", 0, || {
        black_box(core.clone());
    });
    let decoded = zstd::decompress(opencc::DICT_HANS_HANT_ZSTD).unwrap();
    bench("init.core.decode_drop", decoded.len(), || {
        black_box(zstd::decompress(opencc::DICT_HANS_HANT_ZSTD).unwrap());
    });
    bench("init.core.parse_drop", decoded.len(), || {
        black_box(Jieba::with_dict(&mut Cursor::new(&decoded)).unwrap());
    });

    for (script, short) in [("simp", SIMP), ("trad", TRAD), ("mixed", MIXED)] {
        for (length, input) in [("short", short.to_owned()), ("long", short.repeat(256))] {
            let label = format!("{script}.{length}");
            for hmm in [false, true] {
                bench(&format!("core.cut.{label}.hmm_{hmm}"), input.len(), || {
                    black_box(core.cut(black_box(&input), hmm));
                });
                bench(&format!("core.tag.{label}.hmm_{hmm}"), input.len(), || {
                    black_box(core.tag(black_box(&input), hmm));
                });
                bench(
                    &format!("public.cut.{label}.hmm_{hmm}"),
                    input.len(),
                    || {
                        black_box(public.jieba_cut(black_box(&input), hmm));
                    },
                );
            }
            bench(&format!("core.tfidf.{label}"), input.len(), || {
                black_box(tfidf.extract_keywords(&core, black_box(&input), 10, vec![]));
            });
            bench(&format!("core.textrank.{label}"), input.len(), || {
                black_box(textrank.extract_keywords(&core, black_box(&input), 10, vec![]));
            });
            bench(&format!("public.tfidf.{label}"), input.len(), || {
                black_box(public.keyword_weight_tfidf(black_box(&input), 10));
            });
            bench(&format!("public.textrank.{label}"), input.len(), || {
                black_box(public.keyword_weight_textrank(black_box(&input), 10));
            });
            bench(&format!("public.tag.{label}"), input.len(), || {
                black_box(public.jieba_tag(black_box(&input), true));
            });
            bench(&format!("public.convert_s2t.{label}"), input.len(), || {
                black_box(public.convert(black_box(&input), "s2t", false));
            });
        }
    }
    for hmm in [false, true] {
        let text = OOV.repeat(256);
        bench(&format!("core.cut.oov.hmm_{hmm}"), text.len(), || {
            black_box(core.cut(black_box(&text), hmm));
        });
        bench(&format!("core.tag.oov.hmm_{hmm}"), text.len(), || {
            black_box(core.tag(black_box(&text), hmm));
        });
    }
    for top_k in [0, 1, 100] {
        bench(&format!("core.tfidf.top_{top_k}"), MIXED.len(), || {
            black_box(tfidf.extract_keywords(&core, MIXED, top_k, vec![]));
        });
        bench(&format!("core.textrank.top_{top_k}"), MIXED.len(), || {
            black_box(textrank.extract_keywords(&core, MIXED, top_k, vec![]));
        });
    }

    // let dict: String = (0..1000).map(|i| format!("维护词条{i} 100 nz\n")).collect();
    use std::fmt::Write;

    let mut dict = String::new();

    for i in 0..1000 {
        writeln!(dict, "维护词条{i} 100 nz").unwrap();
    }
    let entries: Vec<_> = (0..1000)
        .map(|i| UserDictEntry {
            word: format!("维护词条{i}"),
            freq: 100,
            tag: Some("nz".into()),
        })
        .collect();
    prepared("dict.core.load_1000_empty", dict.len(), Jieba::empty, |j| {
        j.load_dict(&mut Cursor::new(dict.as_bytes())).unwrap();
    });
    prepared(
        "dict.core.load_1000_existing",
        dict.len(),
        || core.clone(),
        |j| {
            j.load_dict(&mut Cursor::new(dict.as_bytes())).unwrap();
        },
    );
    prepared("dict.public.load_1000", dict.len(), OpenCC::new, |j| {
        j.load_user_dict_entries(&entries).unwrap();
    });
    prepared("dict.public.load_empty", 0, OpenCC::new, |j| {
        j.load_user_dict_entries(&[]).unwrap();
    });
    let mut seeded = core.clone();
    seeded.load_dict(&mut Cursor::new(dict.as_bytes())).unwrap();
    prepared(
        "dict.core.load_1000_duplicates",
        dict.len(),
        || seeded.clone(),
        |j| {
            j.load_dict(&mut Cursor::new(dict.as_bytes())).unwrap();
        },
    );
    prepared(
        "dict.core.add_explicit",
        0,
        || core.clone(),
        |j| {
            black_box(j.add_word("星际量子茶壶", Some(100), Some("nz")));
        },
    );
    prepared(
        "dict.core.add_inferred",
        0,
        || core.clone(),
        |j| {
            black_box(j.add_word("星际量子茶壶", None, Some("nz")));
        },
    );
    prepared(
        "dict.public.add_shared",
        0,
        || {
            let cc = OpenCC::new();
            let snapshot = cc.jieba.clone();
            (cc, snapshot)
        },
        |(j, _snapshot)| {
            j.add_word("星际量子茶壶", Some(100), Some("nz")).unwrap();
        },
    );
    // Construct outside timing to ensure the tokenizer Arc is uniquely owned.
    prepared("dict.public.add_unique", 0, OpenCC::new, |j| {
        j.add_word("星际量子茶壶", Some(100), Some("nz")).unwrap();
    });
    bench("dict.core.suggest_freq", 0, || {
        black_box(core.suggest_freq("星际量子茶壶"));
    });
}
