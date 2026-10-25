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
`package.metadata.capi.revision` as a fourth component (including revision zero).
Cargo.lock retains the three-component package version. Windows resources and
`opencc_jieba_version_string()` report the combined version. The C regression
checks the four-component numeric format and ABI number `1`, so package version
and revision updates do not require changing the test. Rust unit tests check the
exact version against the build metadata. The release workflow accepts
four-component `v*` tags and packages both canonical headers.
