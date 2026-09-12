use opencc_jieba_rs::{
    CustomDictMode, CustomDictSpec, DetofuLevel, DictSlot, OpenCC, OpenccConfig, UserDictEntry,
};

fn main() {
    // ---------------------------------------------------------------------
    // Test 1: Basic conversion with the typed config API
    // ---------------------------------------------------------------------
    let opencc = OpenCC::new();

    let input = "意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

    println!("Text: {}", input);
    println!("Text Code: {}", opencc.zho_check(input));

    println!();
    println!("== Test 1: typed conversion ==");

    let traditional = opencc.convert_with_config(input, OpenccConfig::S2twp, true);

    println!("Converted: {}", traditional);
    println!("Converted Code: {}", opencc.zho_check(&traditional));

    // ---------------------------------------------------------------------
    // Test 2: String-config compatibility API
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 2: string config compatibility API ==");

    let by_name = opencc.convert(input, "s2twp", true);

    println!("Converted: {}", by_name);
    println!(
        "Same result: {}",
        if by_name == traditional {
            "PASS"
        } else {
            "FAIL"
        }
    );

    // Invalid string configs remain self-protected and report the error text.
    let invalid = opencc.convert(input, "what_is_this", false);
    println!("Invalid config returned: {}", invalid);

    // ---------------------------------------------------------------------
    // Test 3: Jieba user dictionary + custom conversion dictionary roundtrip
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 3: Jieba user dictionary + custom dictionary roundtrip ==");

    // Phrase mappings are applied to Jieba-tokenized text. Add the custom
    // source phrases to Jieba as well so they remain intact during conversion.
    let user_entries = [
        UserDictEntry {
            word: "帕兰蒂尔".to_string(),
            freq: 100_000,
            tag: None,
        },
        UserDictEntry {
            word: "柏蘭蒂爾".to_string(),
            freq: 100_000,
            tag: None,
        },
        UserDictEntry {
            word: "软件".to_string(),
            freq: 100_000,
            tag: None,
        },
        UserDictEntry {
            word: "軟體".to_string(),
            freq: 100_000,
            tag: None,
        },
    ];

    let mut custom_opencc = OpenCC::try_new_with_user_dict_entries(&user_entries)
        .expect("failed to initialize OpenCC with Jieba user dictionary entries");

    let custom_specs = [
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![
                ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
                ("软件".to_string(), "軟體".to_string()),
            ],
            mode: CustomDictMode::Append,
        },
        CustomDictSpec {
            slot: DictSlot::TSPhrases,
            pairs: vec![
                ("柏蘭蒂爾".to_string(), "帕兰蒂尔".to_string()),
                ("軟體".to_string(), "软件".to_string()),
            ],
            mode: CustomDictMode::Append,
        },
    ];

    custom_opencc
        .load_custom_dicts(&custom_specs)
        .expect("failed to apply custom conversion dictionaries");

    let source = "帕兰蒂尔是一家软件公司。";
    let custom_traditional = custom_opencc.convert_with_config(source, OpenccConfig::S2t, false);
    let custom_simplified =
        custom_opencc.convert_with_config(&custom_traditional, OpenccConfig::T2s, false);

    println!("Source:      {}", source);
    println!("S2T custom:  {}", custom_traditional);
    println!("T2S custom:  {}", custom_simplified);
    println!(
        "Roundtrip:   {}",
        if custom_simplified == source {
            "PASS"
        } else {
            "FAIL"
        }
    );

    // ---------------------------------------------------------------------
    // Test 4: Compatibility normalization
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 4: compatibility normalization ==");

    let compat_source = "天龍八部書";
    let compat_normalized = opencc.normalize_compat(compat_source);

    println!("Source:       {}", compat_source);
    println!("Norm compat:  {}", compat_normalized);
    println!(
        "Result:       {}",
        if compat_normalized == "天龍八部書" {
            "PASS"
        } else {
            "FAIL"
        }
    );

    // ---------------------------------------------------------------------
    // Test 5: Extended normalization -> conversion
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 5: extended normalization -> T2S ==");

    let extended_source = "天龍八部書裡的聼眾‧聼聼竒羙⽟䂖甁噐⾳";

    let normalized = opencc.normalize_compat_extended(extended_source);
    let simplified = opencc.convert_with_config(&normalized, OpenccConfig::T2s, false);

    let norm_ok = normalized == "天龍八部書裡的聽眾·聽聽奇美玉石瓶器音";
    let t2s_ok = simplified == "天龙八部书里的听众·听听奇美玉石瓶器音";

    println!("Source:         {}", extended_source);
    println!("Norm extended:  {}", normalized);
    println!("T2S:            {}", simplified);
    println!(
        "Pipeline:       {}",
        if norm_ok && t2s_ok { "PASS" } else { "FAIL" }
    );

    // ---------------------------------------------------------------------
    // Test 6: DeTofu post-processing
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 6: DeTofu ExtB ==");

    let detofu_source = "骖𬴂";
    let detofued = opencc.detofu(detofu_source, DetofuLevel::ExtB);

    println!("Source:      {}", detofu_source);
    println!("DeTofu:      {}", detofued);
    println!(
        "Result:      {}",
        if detofued == "骖騑" { "PASS" } else { "FAIL" }
    );

    // ---------------------------------------------------------------------
    // Test 7: Recommended full compatibility pipeline
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 7: normalize -> convert -> DeTofu ==");

    let pipeline_source = "天龍八部書裡的聼眾，儼驂騑於上路。";

    let pipeline_normalized = opencc.normalize_compat_extended(pipeline_source);

    let pipeline_converted =
        opencc.convert_with_config(&pipeline_normalized, OpenccConfig::T2s, false);

    let pipeline_display = opencc.detofu(&pipeline_converted, DetofuLevel::ExtB);

    println!("Source:      {}", pipeline_source);
    println!("Normalized:  {}", pipeline_normalized);
    println!("Converted:   {}", pipeline_converted);
    println!("Display:     {}", pipeline_display);

    println!();
    println!("All tests completed.");
}
