//! Phase-2A allocator-request probe; see benchmarks/jieba/PHASE2A.md.
#![allow(dead_code, unused_imports)]
#![cfg(not(test))]

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

use jieba_rs::TfIdf;
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::io::Cursor;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering::Relaxed};

struct Allocator;
static LIVE: AtomicI64 = AtomicI64::new(0);
static PEAK: AtomicI64 = AtomicI64::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);
static REQUESTED: AtomicU64 = AtomicU64::new(0);
fn allocated(bytes: usize) {
    CALLS.fetch_add(1, Relaxed);
    REQUESTED.fetch_add(bytes as u64, Relaxed);
    let live = LIVE.fetch_add(bytes as i64, Relaxed) + bytes as i64;
    PEAK.fetch_max(live, Relaxed);
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as i64, Relaxed);
        System.dealloc(ptr, layout);
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, size);
        if !new_ptr.is_null() {
            LIVE.fetch_sub(layout.size() as i64, Relaxed);
            allocated(size);
        }
        new_ptr
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn probe<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let live = LIVE.load(Relaxed);
    let calls = CALLS.load(Relaxed);
    let requested = REQUESTED.load(Relaxed);
    PEAK.store(live, Relaxed);
    let result = f();
    let retained = LIVE.load(Relaxed) - live;
    let peak = PEAK.load(Relaxed) - live;
    let calls = CALLS.load(Relaxed) - calls;
    let requested = REQUESTED.load(Relaxed) - requested;
    println!("{name},{calls},{requested},{retained},{peak}");
    result
}

fn main() {
    // Prime stdout before the first snapshot. No Jieba/OpenCC instance or model
    // is created: these rows isolate IDF/config allocations from segmentation.
    println!("case,alloc_calls,requested_bytes,retained_delta_bytes,peak_increment_bytes");
    let start = LIVE.load(Relaxed);
    let first = probe("first_default", TfIdf::default);
    let second = probe("second_default", TfIdf::default);
    probe("drop_first", || drop(first));
    probe("drop_second", || drop(second));
    println!(
        "resident_after_dropping_defaults,0,0,{},0",
        LIVE.load(Relaxed) - start
    );
    probe("sixteen_warm_defaults_drop", || {
        for _ in 0..16 {
            black_box(TfIdf::default());
        }
    });
    let mut customized = TfIdf::default();
    probe("customize_default_first_load", || {
        customized
            .load_dict(&mut Cursor::new("phase2a_custom 8\n"))
            .unwrap();
    });
    probe("customize_default_second_load", || {
        customized
            .load_dict(&mut Cursor::new("phase2a_custom 9\n"))
            .unwrap();
    });
    probe("drop_customized", || drop(customized));
}
