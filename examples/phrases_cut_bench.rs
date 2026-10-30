//! Release benchmark: cargo run --release --example phrases_cut_bench -- [samples]
use opencc_jieba_rs::OpenCC;
use rayon::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

struct CountingAllocator;
static ENABLED: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
fn record(size: usize) {
    if ENABLED.load(Relaxed) {
        CALLS.fetch_add(1, Relaxed);
        BYTES.fetch_add(size, Relaxed);
    }
}
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        System.alloc_zeroed(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        System.realloc(ptr, layout, size)
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// Experiments use the same tokenizer, inclusive ranges, and byte threshold as
// phrases_cut_impl. No library changes are needed to compare Vec<String>s.
fn candidate(cc: &OpenCC, input: &str, hmm: bool, indexed: bool) -> Vec<String> {
    let ranges = cc.split_string_ranges(input, true);
    let words = |range: std::ops::Range<usize>| {
        cc.jieba
            .cut(&input[range], hmm)
            .into_iter()
            .map(|token| token.word.to_owned())
    };
    if input.len() < 1000 {
        let mut output = Vec::new();
        for range in ranges {
            output.extend(words(range));
        }
        return output;
    }
    if indexed {
        let chunks: Vec<Vec<String>> = ranges
            .into_par_iter()
            .map(|range| words(range).collect())
            .collect();
        let mut output = Vec::with_capacity(chunks.iter().map(Vec::len).sum());
        for mut chunk in chunks {
            output.append(&mut chunk);
        }
        output
    } else {
        let chunks: std::collections::LinkedList<Vec<String>> = ranges
            .into_par_iter()
            .fold(Vec::new, |mut output, range| {
                output.extend(words(range));
                output
            })
            .collect();
        let mut output = Vec::with_capacity(chunks.iter().map(Vec::len).sum());
        for mut chunk in chunks {
            output.append(&mut chunk);
        }
        output
    }
}

fn validate(cc: &OpenCC) {
    let inputs = [
        String::new(),
        " \r\n\t".into(),
        "，。！？".into(),
        " 南京市长江大桥，杭研小明𠀀🙂\0\r\n ".into(),
        "a".repeat(999),
        "a".repeat(1000),
        "研究生命起源".repeat(200),
        "， \n\t！".repeat(300),
        " 南京市长江大桥，杭研小明🙂𠀀\r\n".repeat(100),
    ];
    for workers in [1, 4, 8] {
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .unwrap()
            .install(|| {
                for input in &inputs {
                    for hmm in [false, true] {
                        let expected = cc.jieba_cut(input, hmm);
                        assert_eq!(expected, candidate(cc, input, hmm, false));
                        assert_eq!(expected, candidate(cc, input, hmm, true));
                    }
                }
            });
    }
}

fn main() {
    let samples: usize = std::env::args()
        .nth(1)
        .unwrap_or("15".into())
        .parse()
        .unwrap();
    assert!(samples > 0);
    let input = std::fs::read_to_string("benchmarks/ChangFeng_Simp.txt").unwrap();
    let cc = OpenCC::new();
    validate(&cc);
    println!(
        "bytes={},chars={},rayon_threads={},samples={samples},warmups=3",
        input.len(),
        input.chars().count(),
        rayon::current_num_threads()
    );
    println!("hmm,implementation,min_ms,median_ms,max_ms,alloc_calls,allocated_bytes");
    for hmm in [false, true] {
        let run = |index| match index {
            0 => cc.jieba_cut(black_box(&input), hmm),
            1 => candidate(&cc, black_box(&input), hmm, false),
            _ => candidate(&cc, black_box(&input), hmm, true),
        };
        assert_eq!(run(0), run(1));
        assert_eq!(run(0), run(2));
        for _ in 0..3 {
            for index in 0..3 {
                black_box(run(index));
            }
        }
        let mut times = [Vec::new(), Vec::new(), Vec::new()];
        // Rotate order to limit systematic drift. Output destruction is timed.
        for sample in 0..samples {
            for step in 0..3 {
                let index = (sample + step) % 3;
                let start = Instant::now();
                black_box(run(index));
                times[index].push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        for (index, samples_times) in times.iter_mut().enumerate() {
            CALLS.store(0, Relaxed);
            BYTES.store(0, Relaxed);
            ENABLED.store(true, Relaxed);
            black_box(run(index));
            ENABLED.store(false, Relaxed);

            samples_times.sort_by(f64::total_cmp);

            println!(
                "{hmm},{},{:.3},{:.3},{:.3},{},{}",
                ["original", "fold_extend", "indexed_chunks"][index],
                samples_times[0],
                samples_times[samples / 2],
                samples_times[samples - 1],
                CALLS.load(Relaxed),
                BYTES.load(Relaxed)
            );
        }
    }
}
