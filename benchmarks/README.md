# Development benchmarks

These tools are a standalone Cargo package, excluded from the library workspace.
Only the public API demonstration remains in `examples/`, so normal library tests
also work without default features.

Run from the repository root:

```sh
cargo run --locked --release --manifest-path benchmarks/Cargo.toml --target-dir target --bin jieba_join_bench -- 15
```

Available binaries: `allocation_bench`, `jieba_join_bench`, `phrases_cut_bench`,
`jieba_baseline`, and `tfidf_memory_probe`. The last two require the default
`keywords` feature and compile private modules for measurement. The build script
reuses the library HMM table generator. Run commands from the repository root so
relative input paths resolve correctly.

See the individual reports and `jieba/` for measurement details.
