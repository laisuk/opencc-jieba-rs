use clap::{Arg, Command};
use opencc_jieba_rs::dictionary_build;
use opencc_tool_common::parse_custom_dict_spec;
use std::path::{Path, PathBuf};
use std::{env, io};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";

    let matches = Command::new("Dictionary Generator")
        .about(format!(
            "{BLUE}Dict Generator: Command Line Dictionary Generator for opencc-jieba-rs{RESET}"
        ))
        .after_help(
            "Examples:\n\
             \n\
             dict-generate --format zstd --output dictionary.json.zst\n\
             dict-generate -D STPhrases:append:my_st_phrases.txt\n\
             dict-generate -D HKPhrases:override:my_hk_phrases.txt -o hk-custom.json.zst\n",
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("format")
                .default_value("zstd")
                .value_parser(["zstd", "json"])
                .help("Dictionary format: [zstd|json]"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("filename")
                .help("Write generated dictionary to <filename>. If not specified, a default filename is used."),
        )
        .arg(
            Arg::new("custom-dict")
                .short('D')
                .long("custom-dict")
                .value_name("SLOT:MODE:FILE")
                .action(clap::ArgAction::Append)
                .help(
                    "Custom conversion dictionary, e.g. \
                     STPhrases:append:my_st_phrases.txt \
                     (slot names are ASCII case-insensitive)",
                ),
        )
        .get_matches();

    let dict_dir = Path::new("dicts");
    if !dict_dir.exists() {
        eprintln!(
            "{BLUE}Error:{RESET} required directory {BLUE}./dicts/{RESET} not found.\n\
             Please provide OpenCC dictionary files in the {BLUE}dicts/{RESET} folder.\n\
             See: https://github.com/laisuk/opencc-jieba-rs/tree/master/dicts\n\
             {BLUE}Exiting.{RESET}"
        );
        std::process::exit(2);
    }

    let dict_format = matches.get_one::<String>("format").map(String::as_str);

    let default_output = match dict_format {
        Some("zstd") => "dictionary.json.zst",
        Some("json") => "dictionary.json",
        _ => "dictionary.unknown",
    };

    let output_file = matches
        .get_one::<String>("output")
        .map(String::as_str)
        .unwrap_or(default_output);

    let output_path = to_abs_path(output_file)?;

    let custom_specs = matches
        .get_many::<String>("custom-dict")
        .map(|values| {
            values
                .map(|value| parse_custom_dict_spec(value))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    match dict_format {
        Some("zstd") => {
            if custom_specs.is_empty() {
                dictionary_build::write_zstd(output_file)?;
            } else {
                dictionary_build::write_zstd_with_custom_dict_files(output_file, &custom_specs)?;
            }

            eprintln!(
                "{BLUE}Dictionary saved in ZSTD format at: {}{RESET}",
                output_path.display()
            );
        }
        Some("json") => {
            if custom_specs.is_empty() {
                dictionary_build::write_json_pretty(output_file)?;
            } else {
                dictionary_build::write_json_pretty_with_custom_dict_files(
                    output_file,
                    &custom_specs,
                )?;
            }

            eprintln!(
                "{BLUE}Dictionary saved in JSON format at: {}{RESET}",
                output_path.display()
            );
        }
        other => {
            let format_str = other.unwrap_or("unknown");
            eprintln!("{BLUE}Unsupported format: {format_str}{RESET}");
            std::process::exit(2);
        }
    }

    Ok(())
}

fn to_abs_path(p: impl AsRef<Path>) -> io::Result<PathBuf> {
    let p = p.as_ref();
    if p.is_absolute() {
        Ok(p.to_owned())
    } else {
        Ok(env::current_dir()?.join(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencc_jieba_rs::{CustomDictMode, DictSlot};
    use opencc_tool_common::parse_custom_dict_spec;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dict_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        env::temp_dir().join(format!("opencc-jieba-rs-dict-generate-{name}-{nonce}.txt"))
    }

    #[test]
    fn parses_custom_dict_slots_case_insensitively() {
        let path = temp_dict_path("slot");
        fs::write(&path, "帕兰蒂尔\t柏蘭蒂爾\n").unwrap();

        let arg = format!(" stphrases : APPEND : {} ", path.display());
        let spec = parse_custom_dict_spec(&arg).unwrap();

        let _ = fs::remove_file(&path);

        assert_eq!(spec.slot, DictSlot::STPhrases);
        assert_eq!(spec.mode, CustomDictMode::Append);
        assert_eq!(spec.files, [path]);
    }

    #[test]
    fn parses_override_mode() {
        let path = temp_dict_path("override");
        fs::write(&path, "龙\t龍龍\n").unwrap();

        let arg = format!("STCharacters:override:{}", path.display());
        let spec = parse_custom_dict_spec(&arg).unwrap();

        let _ = fs::remove_file(&path);

        assert_eq!(spec.slot, DictSlot::STCharacters);
        assert_eq!(spec.mode, CustomDictMode::Override);
    }

    #[test]
    fn rejects_empty_custom_dictionary_paths() {
        assert!(parse_custom_dict_spec("STPhrases:append:   ").is_err());
    }

    #[test]
    fn rejects_unknown_custom_dictionary_slots() {
        let path = temp_dict_path("unknown-slot");
        fs::write(&path, "甲\t乙\n").unwrap();

        let arg = format!("UnknownSlot:append:{}", path.display());
        let result = parse_custom_dict_spec(&arg);

        let _ = fs::remove_file(&path);

        assert!(result.is_err());
    }
}
