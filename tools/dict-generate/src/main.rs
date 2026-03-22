use clap::{Arg, Command};
use opencc_jieba_rs::dictionary_lib::Dictionary;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{env, io};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";

    let matches = Command::new("Dictionary Generator")
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("format")
                .default_value("zstd")
                .help("Dictionary format: [zstd|json]"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("filename")
                .help("Write generated dictionary to <filename>. If not specified, a default filename is used."),
        )
        .about(format!(
            "{BLUE}Dict Generator: Command Line Dictionary Generator for opencc-jieba-rs{RESET}"
        ))
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
        .map(|s| s.as_str())
        .unwrap_or(default_output);

    let output_path = to_abs_path(output_file)?;

    match dict_format {
        Some("zstd") => {
            let dictionary = Dictionary::from_dicts();
            Dictionary::save_json_compressed(&dictionary, output_file)?;
            eprintln!(
                "{BLUE}Dictionary saved in ZSTD format at: {}{RESET}",
                output_path.display()
            );
        }
        Some("json") => {
            let dictionary = Dictionary::from_dicts();
            let file = File::create(output_file)?;
            serde_json::to_writer_pretty(file, &dictionary)?;
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
