use clap::{Arg, ArgMatches, Command};
use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;
use std::fs::File;
use std::io::{self, stdin, BufWriter, IsTerminal, Read, Write};

use opencc_jieba_rs;
use opencc_jieba_rs::OpenCC;

const CONFIG_LIST: [&str; 16] = [
    "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "t2twp", "t2hk",
    "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t",
];

const BLUE: &str = "\x1B[1;34m";
const RESET: &str = "\x1B[0m";

pub fn read_input(
    input_file: Option<&str>,
    in_enc: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut input_str = String::new();

    // Use locked and buffered stdin
    let stdin = stdin();
    let mut handle = stdin.lock();

    match in_enc {
        "UTF-8" => {
            if let Some(file_name) = input_file {
                // Read file directly into a String
                File::open(file_name)?.read_to_string(&mut input_str)?;
            } else {
                // Terminal prompt only if input is from terminal
                if stdin.is_terminal() {
                    eprintln!("{BLUE}Input text to convert, <ctrl-z> or <ctrl-d> to submit:{RESET}");
                }

                // let stdin = stdin();
                // let mut handle = stdin.lock();
                let mut buffer = [0u8; 1024];

                while let Ok(n) = handle.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    input_str.push_str(&String::from_utf8_lossy(&buffer[..n]));
                }
            }
        }

        _ => {
            let mut bytes = Vec::new();

            if let Some(file_name) = input_file {
                File::open(file_name)?.read_to_end(&mut bytes)?;
            } else {
                if stdin.is_terminal() {
                    eprintln!("{BLUE}Input text to convert, <ctrl-z> or <ctrl-d> to submit:{RESET}");
                }

                // let stdin = stdin();
                // let mut handle = stdin.lock();
                let mut buffer = [0u8; 1024];

                while let Ok(n) = handle.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                }
            }

            let encoding = Encoding::for_label(in_enc.as_bytes()).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unsupported input encoding: {}", in_enc),
                )
            })?;

            let mut decoder = DecodeReaderBytesBuilder::new()
                .encoding(Some(encoding))
                .build(&*bytes);

            decoder.read_to_string(&mut input_str)?;
        }
    }

    Ok(input_str)
}

fn should_remove_bom(in_enc: &str, out_enc: &str) -> bool {
    in_enc.eq_ignore_ascii_case("utf-8") && !out_enc.eq_ignore_ascii_case("utf-8")
}

fn remove_utf8_bom_str_inplace(s: &mut String) {
    if s.starts_with('\u{FEFF}') {
        s.drain(..1); // Remove first char (BOM) without reallocation
    }
}

fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn write_output(
    output_file: Option<&str>,
    out_enc: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output: Box<dyn Write> = match output_file {
        Some(file_name) => Box::new(File::create(file_name)?),
        None => Box::new(io::stdout().lock()), // 🔒 important for proper redirection
    };

    let mut output_buf = BufWriter::new(output);

    match out_enc {
        "UTF-8" => {
            write!(output_buf, "{}", content)?;
            if output_file.is_none() && !content.ends_with('\n') {
                write!(output_buf, "\n")?;
            }
        }

        _ => {
            let encoding = Encoding::for_label(out_enc.as_bytes())
                .ok_or_else(|| format!("Unsupported output encoding: {}", out_enc))?;
            let (encoded_bytes, _, _) = encoding.encode(content);
            output_buf.write_all(&encoded_bytes)?;
            if output_file.is_none() && !content.ends_with('\n') {
                let (newline, _, _) = encoding.encode("\n");
                output_buf.write_all(&newline)?;
            }
        }
    }

    output_buf.flush()?; // 🚿 Always flush to make sure it’s written!
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("opencc-jieba")
        .about(format!(
            "{}OpenCC Jieba Rust: Command Line Open Chinese Converter{}",
            BLUE, RESET
        ))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("in_enc")
                .long("in-enc")
                .value_name("encoding")
                .default_value("UTF-8")
                .global(true)
                .help("Encoding for input: UTF-8|GB2312|GBK|gb18030|BIG5"),
        )
        .arg(
            Arg::new("out_enc")
                .long("out-enc")
                .value_name("encoding")
                .default_value("UTF-8")
                .global(true)
                .help("Encoding for output: UTF-8|GB2312|GBK|gb18030|BIG5"),
        )
        .subcommand(
            Command::new("convert")
                .about(format!(
                    "{}opencc-jieba convert: Convert Chinese Traditional/Simplified text using OpenCC{}",
                    BLUE,
                    RESET
                ))
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("file")
                        .help("Read original text from <file>."),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("file")
                        .help("Write converted text to <file>."),
                )
                .arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .value_name("conversion")
                        .help(
                            "Conversion configuration: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]",
                        )
                        .required(true),
                )
                .arg(
                    Arg::new("punct")
                        .short('p')
                        .long("punct")
                        .value_name("boolean")
                        .default_value("false")
                        .help("Punctuation conversion: [true|false]"),
                ),
        )
        .subcommand(
            Command::new("segment")
                .about(format!(
                    "{}opencc-jieba segment: Segment Chinese input text into words{}",
                    BLUE, RESET
                ))
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("file")
                        .help("Input file to segment")
                        .required(false),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("file")
                        .help("Write segmented result to file")
                        .required(false),
                )
                .arg(
                    Arg::new("delimiter")
                        .short('d')
                        .long("delim")
                        .value_name("character")
                        .help("Delimiter character for segmented text")
                        .required(false)
                        .default_value("/"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("convert", sub_matches)) => {
            handle_convert(sub_matches)?;
        }
        Some(("segment", sub_matches)) => {
            handle_segment(sub_matches)?;
        }
        _ => unreachable!("Clap ensures only valid subcommands are passed"),
    }

    fn handle_convert(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let input_file = matches.get_one::<String>("input").map(String::as_str);
        let output_file = matches.get_one::<String>("output").map(String::as_str);
        let config = matches.get_one::<String>("config").unwrap().as_str();
        if !CONFIG_LIST.contains(&config) {
            eprintln!("Invalid config: {}", config);
            eprintln!("Valid Config are: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]");
            return Ok(());
        }
        let punctuation = matches
            .get_one::<String>("punct")
            .map_or(false, |value| value == "true");

        let in_enc = matches.get_one::<String>("in_enc").unwrap().as_str();
        let out_enc = matches.get_one::<String>("out_enc").unwrap().as_str();

        let mut input_str = read_input(input_file, in_enc)?;
        if should_remove_bom(in_enc, out_enc) {
            remove_utf8_bom_str_inplace(&mut input_str)
        }

        let output_str = OpenCC::new().convert(&input_str, config, punctuation);
        write_output(output_file, out_enc, &output_str)?;

        eprintln!(
            "{BLUE}Conversion completed ({config}): {} -> {}{RESET}",
            input_file.unwrap_or(&"<stdin>".to_string()),
            output_file.unwrap_or(&"stdout".to_string())
        );

        Ok(())
    }

    fn handle_segment(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let input_file = matches.get_one::<String>("input").map(String::as_str);
        let output_file = matches.get_one::<String>("output").map(String::as_str);
        let delimiter = matches.get_one::<String>("delimiter").unwrap().as_str();
        let in_enc = matches.get_one::<String>("in_enc").unwrap().as_str();
        let out_enc = matches.get_one::<String>("out_enc").unwrap().as_str();

        let mut input_str = read_input(input_file, in_enc)?;
        
        if should_remove_bom(in_enc, out_enc) {
            remove_utf8_bom_str_inplace(&mut input_str)
        }
        
        if input_file.is_none() {
            input_str = normalize_line_endings(&input_str);
        }        
        
        let output_str = OpenCC::new().jieba_cut_and_join(&input_str, true, delimiter);
        write_output(output_file, out_enc, &output_str)?;

        eprintln!(
            "{BLUE}Segmentation completed ({delimiter}): {} -> {}{RESET}",
            input_file.unwrap_or(&"<stdin>".to_string()),
            output_file.unwrap_or(&"stdout".to_string())
        );

        Ok(())
    }

    Ok(())
}
