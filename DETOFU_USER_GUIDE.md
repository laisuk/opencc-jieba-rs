# DeTofu User Guide

DeTofu is an optional post-conversion display-compatibility pass for converted text that contains rare non-BMP CJK
extension characters. It is useful when those characters may render as tofu boxes on some systems, browsers, fonts,
document viewers, mobile devices, or e-book readers.

DeTofu is not an OpenCC dictionary conversion rule. It does not change OpenCC conversion dictionaries, phrase matching,
regional variant selection, script detection, or punctuation conversion. It is exposed through `OpenCC` as an optional
post-conversion add-on and should normally be applied after `convert()`:

```rust
use opencc_jieba_rs::{DetofuLevel, OpenCC};

fn main() {
    let cc = OpenCC::new();

    let converted = cc.convert(
        "儼驂騑於上路，訪風景於崇阿",
        "t2s",
        false,
    );

    let safe = cc.detofu(&converted, DetofuLevel::ExtB);

    assert_eq!(safe, "俨骖騑于上路，访风景于崇阿");
}
```

## Public APIs

* `DetofuLevel` selects the fallback threshold.
* `OpenCC` is the normal public entry point for DeTofu in an OpenCC conversion workflow.
* `DetofuMap` is the advanced reusable/customizable API for bindings, tools, and applications that need their own fallback map.

### OpenCC APIs

* `OpenCC::detofu(&self, input: &str, level: DetofuLevel) -> String` applies DeTofu through an `OpenCC` instance.
* `OpenCC::detofu_into(&self, input: &str, level: DetofuLevel, output: &mut String)` appends the DeTofu result to an existing output buffer for allocation reuse.
* `OpenCC::detofu_with_custom_file(&self, input, level, path)` applies DeTofu using the built-in fallback table plus a user-supplied fallback file.
* `OpenCC::detofu_with_custom_pairs(&self, input, level, pairs)` applies DeTofu using the built-in fallback table plus in-memory custom fallback pairs.

### Reusable `DetofuMap`

* `DetofuMap::builtin(level)` creates a reusable DeTofu map backed by the shared built-in fallback table.
* `DetofuMap::with_custom_pairs(...)` adds custom fallback pairs that override built-in mappings.
* `DetofuMap::with_custom_file(...)` loads additional fallback mappings from a UTF-8 text file.
* `DetofuMap::detofu(...)` applies a reusable map and returns a newly allocated `String`.
* `DetofuMap::detofu_into(...)` applies a reusable map and appends the result to an existing output buffer for allocation reuse.


## OpenCC Instance Usage

```rust,no_run
use opencc_jieba_rs::{DetofuLevel, OpenCC};

fn main() -> std::io::Result<()> {
    let cc = OpenCC::new();
    let input = "骖𬴂";

    let safe = cc.detofu(input, DetofuLevel::ExtB);
    let safe_from_file = cc.detofu_with_custom_file(input, DetofuLevel::ExtB, "custom_tofu.txt")?;
    let safe_from_pairs = cc.detofu_with_custom_pairs(input, DetofuLevel::ExtB, &[('𬴂', '騑')]);

    assert_eq!(safe, "骖騑");
    assert_eq!(safe_from_file, "骖騑");
    assert_eq!(safe_from_pairs, "骖騑");

    Ok(())
}
```

## Custom Pairs

```rust
use opencc_jieba_rs::{DetofuLevel, OpenCC};

fn main() -> std::io::Result<()> {
    let cc = OpenCC::new();

    let safe = cc.detofu_with_custom_pairs(
        "𣭲毛 骖𬴂",
        DetofuLevel::ExtB,
        &[('𣭲', '氄'), ('𬴂', '騑')],
    );

    assert_eq!(safe, "氄毛 骖騑");

    Ok(())
}
```

Each custom pair is `(tofu_char, fallback_char)`. Pairs are applied after built-in mappings, so if a pair key already
exists in the built-in map, the custom pair wins. If the same key appears more than once in the slice, the later pair
wins. Unlike custom fallback files, direct pairs do not carry an extension column, so they are always added to the
selected map. DeTofu preserves unmapped characters unchanged.

## Advanced Reusable Map Usage

```rust,no_run
use opencc_jieba_rs::{DetofuLevel, DetofuMap};

fn main() -> std::io::Result<()> {
    let file_map = DetofuMap::builtin(DetofuLevel::ExtB)
        .with_custom_file("custom_tofu.txt")?;

    let pair_map = DetofuMap::builtin(DetofuLevel::ExtB)
        .with_custom_pairs(&[('𣭲', '氄')]);

    assert_eq!(file_map.detofu("𣭲毛"), "氄毛");
    assert_eq!(pair_map.detofu("𣭲毛"), "氄毛");

    Ok(())
}
```

## Threshold Behavior

Threshold behavior is inclusive of later supported extensions:

* `DetofuLevel::ExtB` means ExtB and above.
* `DetofuLevel::ExtC` means ExtC and above.
* `DetofuLevel::ExtD` means ExtD and above.
* `DetofuLevel::ExtE` means ExtE and above.
* `DetofuLevel::ExtF` means ExtF and above.
* `DetofuLevel::ExtG` means ExtG and above.
* `DetofuLevel::ExtH` means ExtH and above.
* `DetofuLevel::ExtI` means ExtI only.

## Custom Fallback Files

```rust,no_run
use opencc_jieba_rs::{DetofuLevel, OpenCC};

fn main() -> std::io::Result<()> {
    let cc = OpenCC::new();

    let safe = cc.detofu_with_custom_file(
        "𣭲毛",
        DetofuLevel::ExtB,
        "custom_tofu.txt",
    )?;

    assert_eq!(safe, "氄毛");

    Ok(())
}
```

Example file:

```text
# tofu_char<TAB>fallback_char<TAB>extension

𣭲	氄	B
```

The extension column accepts either:

```text
B
C
D
...
```

or the legacy form:

```text
ExtB
ExtC
ExtD
...
```

> The built-in DeTofu table already contains many fallback mappings. Custom pairs and custom files are applied after
> loading the built-in table and override existing mappings when the same tofu-risk character is provided.
>
> Characters are only replaced when a matching built-in or custom fallback mapping exists. Unmapped characters are
> preserved unchanged, even when they belong to an enabled CJK extension block.

---

> Crate: [opencc-jieba-rs on crates.io](https://crates.io/crates/opencc-jieba-rs)  
> Docs: [docs.rs/opencc-jieba-rs](https://docs.rs/opencc-jieba-rs/latest/opencc_jieba_rs/)