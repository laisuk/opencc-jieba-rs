//! Release benchmark: cargo run --release --example jieba_join_bench -- [samples]
use opencc_jieba_rs::OpenCC;
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

fn main() {
    let samples: usize = std::env::args()
        .nth(1)
        .unwrap_or("15".into())
        .parse()
        .unwrap();
    assert!(samples > 0);
    let input = std::fs::read_to_string("benchmarks/ChangFeng_Simp.txt").unwrap();
    let cc = OpenCC::new();
    println!(
        "bytes={},chars={},rayon_threads={},samples={samples},warmups=3,delimiter=space",
        input.len(),
        input.chars().count(),
        rayon::current_num_threads()
    );
    println!("hmm,implementation,min_ms,median_ms,max_ms,alloc_calls,allocated_bytes");
    for hmm in [false, true] {
        let run = |original| {
            if original {
                cc.jieba_cut(black_box(&input), hmm).join(" ")
            } else {
                cc.jieba_cut_and_join(black_box(&input), hmm, " ")
            }
        };
        assert_eq!(run(true), run(false));
        for _ in 0..3 {
            black_box(run(true));
            black_box(run(false));
        }
        let mut times = [Vec::new(), Vec::new()];
        // Alternate order to limit systematic drift. Output destruction is timed.
        for sample in 0..samples {
            for index in if sample % 2 == 0 { [0, 1] } else { [1, 0] } {
                let start = Instant::now();
                black_box(run(index == 0));
                times[index].push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        for index in 0..2 {
            CALLS.store(0, Relaxed);
            BYTES.store(0, Relaxed);
            ENABLED.store(true, Relaxed);
            black_box(run(index == 0));
            ENABLED.store(false, Relaxed);
            times[index].sort_by(f64::total_cmp);
            println!(
                "{hmm},{},{:.3},{:.3},{:.3},{},{}",
                if index == 0 {
                    "original"
                } else {
                    "public_join"
                },
                times[index][0],
                times[index][samples / 2],
                times[index][samples - 1],
                CALLS.load(Relaxed),
                BYTES.load(Relaxed)
            );
        }
    }
}
