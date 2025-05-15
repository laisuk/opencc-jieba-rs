use clap::{Arg, Command};
use opencc_jieba_rs::dictionary_lib::Dictionary;
use std::fs::File;
use std::io;
use std::io::BufWriter;
use zstd::Encoder;

pub fn save_compressed(
    dictionary: &Dictionary,
    path: &str,
) -> Result<(), io::Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut encoder = Encoder::new(writer, 19)?;
    serde_json::to_writer(&mut encoder, dictionary)?;
    encoder.finish()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BLUE: &str = "\x1B[1;34m"; // Bold Blue
    const RESET: &str = "\x1B[0m"; // Reset color

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

    match dict_format {
        Some("zstd") => {
            let dictionary = Dictionary::from_dicts();
            save_compressed(&dictionary, output_file)?;
            eprintln!("{BLUE}Dictionary saved in ZSTD format at: {output_file}{RESET}");
        }
        Some("json") => {
            let dictionary = Dictionary::from_dicts();
            let file = File::create(output_file)?;
            serde_json::to_writer_pretty(file, &dictionary)?;
            eprintln!("{BLUE}Dictionary saved in JSON format at: {output_file}{RESET}");
        }
        other => {
            let format_str = other.unwrap_or("unknown");
            eprintln!("{BLUE}Unsupported format: {format_str}{RESET}");
        }
    }

    Ok(())
}
