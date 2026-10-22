These regression programs compile against the canonical headers and a freshly
built C API library. The C++ helper requires C++17. Assertions must be enabled.

On Linux, from the repository root:

```sh
cargo build -p opencc_jieba_capi --locked
cc -std=c11 -Wall -Wextra -Werror -I capi/include capi/tests/add_word.c -L target/debug -lopencc_jieba_capi -o target/debug/add_word_c
c++ -std=c++17 -Wall -Wextra -Werror -I capi/include capi/tests/add_word.cpp -L target/debug -lopencc_jieba_capi -o target/debug/add_word_cpp
LD_LIBRARY_PATH=target/debug target/debug/add_word_c
LD_LIBRARY_PATH=target/debug target/debug/add_word_cpp
```

The C API release version combines Cargo's three-component package version with
`package.metadata.capi.revision`, currently `0.8.1.1`. Cargo.lock therefore retains
`0.8.1`. Windows resources and `opencc_jieba_version_string()` report the combined
version. The release workflow accepts four-component `v*` tags and packages both
canonical headers; use `v0.8.1.1` for this C API release. ABI number remains `1`.
