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

fn main() {
    let cc = OpenCC::new();
    let novel = std::fs::read_to_string("benchmarks/ChangFeng_Simp.txt").unwrap();
    let mut large = String::new();
    while large.len() < 3_000_000 {
        large.push_str(&novel);
    }
    let mut end = 3_000_000;
    while !large.is_char_boundary(end) {
        end -= 1;
    }
    large.truncate(end);
    println!("case,bytes,iterations,ns_per_call,mib_per_s,alloc_calls,allocated_bytes");
    for (name, text, n) in [
        ("short", "汉字转换测试", 10000),
        ("no_match", "Hello world 123!", 10000),
        ("novel_3mb", large.as_str(), 5),
    ] {
        for (op, f) in [
            ("s2twp", OpenCC::s2twp as fn(&OpenCC, &str, bool) -> String),
            ("t2twp", OpenCC::t2twp),
        ] {
            for _ in 0..3 {
                black_box(f(&cc, text, true));
            }
            CALLS.store(0, Relaxed);
            BYTES.store(0, Relaxed);
            ENABLED.store(true, Relaxed);
            black_box(f(&cc, text, true));
            ENABLED.store(false, Relaxed);
            let calls = CALLS.load(Relaxed);
            let bytes = BYTES.load(Relaxed);
            let start = Instant::now();
            for _ in 0..n {
                black_box(f(&cc, black_box(text), true));
            }
            let ns = start.elapsed().as_nanos() as f64 / n as f64;
            println!(
                "{name}_{op},{},{n},{ns:.0},{:.2},{calls},{bytes}",
                text.len(),
                text.len() as f64 / (ns / 1e9) / 1048576.0
            );
        }
    }
}
