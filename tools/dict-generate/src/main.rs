use clap::{Arg, Command};
use opencc_jieba_rs::dictionary_lib::Dictionary;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use std::{fs, io};
use ureq::Agent;

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

    let dict_dir = Path::new("dicts");
    if !dict_dir.exists() {
        eprint!("{BLUE}Local 'dicts/' not found. Proceed with downloading dictionaries from GitHub? (Y/n): {RESET}");
        io::stdout().flush()?; // Ensure prompt is printed before read_line

        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        let answer = answer.trim().to_lowercase();

        if answer.is_empty() || answer == "y" || answer == "yes" {
            eprintln!("{BLUE}Downloading from GitHub...{RESET}");
            fetch_dicts_from_github(dict_dir)?;
        } else {
            eprintln!("{BLUE}Aborted by user. Exiting.{RESET}");
            return Ok(()); // or `std::process::exit(0);` if you want a hard exit
        }
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

    match dict_format {
        Some("zstd") => {
            let dictionary = Dictionary::from_dicts();
            Dictionary::save_compressed(&dictionary, output_file)?;
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

/// Download missing dict files from GitHub repo
fn fetch_dicts_from_github(dict_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dict_files = [
        "STCharacters.txt",
        "STPhrases.txt",
        "TSCharacters.txt",
        "TSPhrases.txt",
        "TWPhrases.txt",
        "TWPhrasesRev.txt",
        "TWVariants.txt",
        "TWVariantsRev.txt",
        "TWVariantsRevPhrases.txt",
        "HKVariants.txt",
        "HKVariantsRev.txt",
        "HKVariantsRevPhrases.txt",
        "JPShinjitaiCharacters.txt",
        "JPShinjitaiPhrases.txt",
        "JPVariants.txt",
        "JPVariantsRev.txt",
    ];

    fs::create_dir_all(dict_dir)?;

    let config = Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build();
    let agent: Agent = config.into();

    for filename in &dict_files {
        let url = format!(
            "https://raw.githubusercontent.com/laisuk/opencc-jieba-rs/master/dicts/{}",
            filename
        );

        let response = agent.get(&url).call()?;
        let mut content = String::new();
        response
            .into_body()
            .into_reader()
            .read_to_string(&mut content)?;

        let dest_path = dict_dir.join(filename);
        let mut file = File::create(dest_path)?;
        file.write_all(content.as_bytes())?;

        eprintln!("Downloaded: {}", filename);
    }

    Ok(())
}
