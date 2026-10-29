# Jieba internal-module vendor bundle (source: jieba-rs 0.11.0)

Copy `src/jieba_rs/` into your parent `src/` and copy `build.rs` to the **package root** next to Cargo.toml. If you
already have a build.rs, merge the `generate_hmm_data()` function and rerun directive into it; do not overwrite your
Windows resource tasks.

Parent integration checklist:

1. Declare `mod jieba_rs;` (or `pub mod jieba_rs;`) in parent `src/lib.rs`. Change `use jieba_rs::...` in parent code to
   `use crate::jieba_rs::...`.
2. Confirm vendored hash module path `crate::rustc_hash::FxBuildHasher` matches your project; adjust imports if needed.
3. Keep `bytecount` in parent Cargo.toml; remove `jieba-rs`, `include-flate`, `ordered-float`, and `jieba-macros` *only
   if nothing else uses them*.
4. The parent `crate::zstd::decompress(&[u8]) -> Result<Vec<u8>, _>` must exist (as shown in your opencc.rs).
5. This version always includes TF-IDF and TextRank and uses lazy zstd decode for IDF and POS model. No bundled
   `dict.txt`; `Jieba::new()` is empty. `OpenCC::with_dict` path is unaffected.
6. Run `cargo +1.75.0 check`, `cargo +1.75.0 test`, `cargo clippy`, plus segmentation/POS/keyword regressions. This
   bundle is not integration-tested against your parent repo, which was not provided.
7. Upstream doc examples and test fixtures may need adjustments after moving to an internal module.

Upstream Jieba source LICENSE is included for attribution. This is a best-effort migration scaffold, not a verified
drop-in replacement.
