# opencc-jieba-rs

![Build Status](https://github.com/laisuk/opencc-jieba-rs/actions/workflows/rust.yml/badge.svg)
![Crates.io](https://img.shields.io/crates/v/opencc-jieba-rs)
![License](https://img.shields.io/github/license/laisuk/opencc-jieba-rs)

A Rust-based Chinese text converter that performs word segmentation using **Jieba**, powered by **OpenCC lexicons**. This project aims to provide high-performance and accurate **Simplified ↔ Traditional Chinese** (zh-Hans ↔ zh-Hant) conversion.

## Features

- 📦 Simple CLI tool for converting between Simplified and Traditional Chinese.
- 🔍 Lexicon-driven segmentation using OpenCC dictionaries.
- ⚡ Utilizes Jieba's probabilistic models for more accurate word segmentation, improving the quality of Simplified ↔ Traditional Chinese conversion.
- 🛠️ Designed to be easily embedded as a Rust library or used standalone.


## Installation

```bash
git clone https://github.com/laisuk/opencc-jieba-rs
cd opencc-jieba-rs
cargo build --release --workspace
```

The CLI tool will be located at:

```
target/release/opencc-jieba
```

## Usage: convert

```
opencc-jieba convert: Convert Chinese Traditional/Simplified text using OpenCC

(Windows)
Usage: opencc-jieba.exe convert [OPTIONS] --config <conversion>
(Linux / macOS)
Usage: opencc-jieba convert [OPTIONS] --config <conversion>

Options:
  -i, --input <file>         Read original text from <file>.
      --in-enc <encoding>    Encoding for input: UTF-8|GB2312|GBK|gb18030|BIG5 [default: UTF-8]
  -o, --output <file>        Write converted text to <file>.
      --out-enc <encoding>   Encoding for output: UTF-8|GB2312|GBK|gb18030|BIG5 [default: UTF-8]
  -c, --config <conversion>  Conversion configuration: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]
  -p, --punct <boolean>      Punctuation conversion: [true|false] [default: false]
  -h, --help                 Print help

```

## Usage: segment

```
opencc-jieba segment: Segment Chinese input text into words

Usage: opencc-jieba segment [OPTIONS]

Options:
  -i, --input <file>        Input file to segment
      --in-enc <encoding>   Encoding for input: UTF-8|GB2312|GBK|gb18030|BIG5 [default: UTF-8]
  -o, --output <file>       Write segmented result to file
      --out-enc <encoding>  Encoding for output: UTF-8|GB2312|GBK|gb18030|BIG5 [default: UTF-8]
  -d, --delim <character>   Delimiter character for segmented text [default: /]
  -h, --help                Print help
```

### Example

```bash
# Convert Simplified Chinese to Traditional Chinese
opencc-jieba convert -i input.txt -o output.txt --config s2t

# Convert Traditional Chinese (Taiwan Standard) to Simplified Chinese
opencc-jieba convert -i input.txt -o output.txt --config tw2s

# Segment text file contents then output to new file
opencc-jieba segment -i input.txt -o output.txt --delim ","
```

- Supported conversions:
  - `s2t` – Simplified to Traditional
  - `s2tw` – Simplified to Traditional Taiwan
  - `s2twp` – Simplified to Traditional Taiwan with idioms
  - `t2s` – Traditional to Simplified
  - `tw2s` – Traditional Taiwan to Simplified
  - `tw2sp` – Traditional Taiwan to Simplified with idioms
  - etc

### Lexicons

By default, it uses OpenCC's built-in lexicon paths. You can also provide your own lexicon folder as the fourth argument.

## Library Usage

You can also use `opencc-jieba-rs` as a library:

```rust
use opencc_jieba_rs::OpenCC;

fn main() {
  let input = "这是一个测试";
  let opencc = OpenCC::new();
  let output = opencc.convert(input, "s2t", false);
  println!("{}", output); // -> "這是一個測試"
}
```

## C API Usage (`opencc_jieba_capi`)

You can also use `opencc-jieba-rs` via a C API for integration with C/C++ projects.

### Example

```c
#include <stdio.h>
#include "opencc_jieba_capi.h"

int main(int argc, char **argv) {
    void *opencc = opencc_jieba_new();
    const char *config = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    printf("Text: %s\n", text);
    int code = opencc_jieba_zho_check(opencc, text);
    printf("Text Code: %d\n", code);
    char *result = opencc_jieba_convert(opencc, text, config, true);
    code = opencc_jieba_zho_check(opencc, result);
    printf("Converted: %s\n", result);
    printf("Converted Code: %d\n", code);
    if (result != NULL) {
        opencc_jieba_free_string(result);
    }
    if (opencc != NULL) {
        opencc_jieba_delete(opencc);
    }

    return 0;
}
```

### Output
```
Text: 意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。
Text Code: 2
Converted: 義大利鄰國法蘭西羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。
Converted Code: 1
```
### Notes

- `opencc_jieba_new()` initializes the engine.
- `opencc_jieba_convert(...)` performs the conversion with the specified config (e.g., `s2t`, `t2s`, `s2twp`).
- `opencc_jieba_free_string(...)` must be called to free the returned string.
- `opencc_jieba_delete(...)` must be called to free OpenCC instance.
- `opencc_jieba_zho_check(...)` to detect zh-Hant (1), zh-Hans (2), others (0).


## Project Structure

- `src/lib.rs` – Main library with segmentation logic.
- `capi/opencc-jieba-capi` C API source and demo.
- `tools/opencc-jieba/src/main.rs` – CLI tool (`opencc-cs`) implementation.
- `dicts/` – OpenCC text lexicons which converted into JSON format.

## Dictionary compression (ZStd)
[ZStandard](https://github.com/facebook/zstd) - `zstd`: A fast lossless compression algorithm, targeting real-time compression scenarios at zlib-level and better compression ratios.
```
zstd -19 src/dictionary_lib/dicts/dictionary.json -o src/dictionary_lib/dicts/dictionary.json.zst
zstd -19 src/dictionary_lib/dicts/dict_hans_hant.txt -o src/dictionary_lib/dict_hans_hant.txt.zst
```

## Credits

- [OpenCC](https://github.com/BYVoid/OpenCC) – Lexicon source.
- [jieba-rs](https://github.com/messense/jieba-rs) - Jieba tokenization.

## License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.


## Contributing
Contributions are welcome! Please open issues or submit pull requests for improvements or bug fixes.


