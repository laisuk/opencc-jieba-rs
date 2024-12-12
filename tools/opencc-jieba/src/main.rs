use std::fs::File;
use std::io::{self, BufWriter, Read, Write};

use clap::{Arg, Command};
use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;

use opencc_jieba_rs;
use opencc_jieba_rs::OpenCC;

const CONFIG_LIST: [&str; 16] = [
    "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "t2twp", "t2hk",
    "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";
    let matches = Command::new("OpenCC Rust")
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
        )
        .arg(
            Arg::new("in_enc")
                .long("in-enc")
                .value_name("encoding")
                .default_value("UTF-8")
                .help("Encoding for input"),
        )
        .arg(
            Arg::new("out_enc")
                .long("out-enc")
                .value_name("encoding")
                .default_value("UTF-8")
                .help("Encoding for output"),
        )
        .about(format!(
            "{}OpenCC Rust: Command Line Open Chinese Converter{}",
            BLUE, RESET
        ))
        .get_matches();

    let input_file = matches.get_one::<String>("input");
    let output_file = matches.get_one::<String>("output");
    let config = matches.get_one::<String>("config").unwrap().as_str();
    if !CONFIG_LIST.contains(&config) {
        println!("Invalid config: {}", config);
        println!("Valid Config are: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]");
        return Ok(());
    }
    let punctuation = matches
        .get_one::<String>("punct")
        .map_or(false, |value| value == "true");

    let mut input: Box<dyn Read> = match input_file {
        Some(file_name) => Box::new(File::open(file_name)?),
        None => {
            println!("{BLUE}Input text to convert, <ctrl-z> or <ctrl-d> to summit:{RESET}");
            Box::new(io::stdin())
        }
    };

    let output: Box<dyn Write> = match output_file {
        Some(file_name) => Box::new(File::create(file_name)?),
        None => Box::new(io::stdout()),
    };

    let mut output_buf = BufWriter::new(output);

    let mut input_str = String::new();
    let in_enc = matches.get_one::<String>("in_enc").unwrap().as_str();
    match in_enc {
        "UTF-8" => {
            if let Some(file_name) = input_file {
                // File input: read all data at once
                let mut file = File::open(file_name)?;
                file.read_to_string(&mut input_str)?;
            } else {
                // Console input: use buffered reading
                let mut buffer = [0; 1024]; // Buffer to hold chunks of data
                while let Ok(n) = input.read(&mut buffer) {
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
                // File input: read all bytes at once
                let mut file = File::open(file_name)?;
                file.read_to_end(&mut bytes)?;
            } else {
                // Console input: use buffered reading
                let mut buffer = [0; 1024]; // Buffer for chunked reading
                while let Ok(n) = input.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..n]); // Collect bytes in chunks
                }
            }
            let encoding = Encoding::for_label(in_enc.as_bytes()).ok_or_else(|| {
                let err_msg = format!("Unsupported input encoding: {}", in_enc);
                eprintln!("{}", &err_msg);
                io::Error::new(io::ErrorKind::Other, err_msg)
            })?;
            let mut decoder = DecodeReaderBytesBuilder::new()
                .encoding(Some(encoding))
                .build(&*bytes);
            decoder.read_to_string(&mut input_str)?;
        }
    }

    let opencc = OpenCC::new();

    let output_str = opencc.convert(&input_str, config, punctuation);

    let out_enc = matches.get_one::<String>("out_enc").unwrap().as_str();
    match out_enc {
        "UTF-8" => {
            write!(output_buf, "{}", output_str)?;
        }
        _ => match Encoding::for_label(out_enc.as_bytes()) {
            None => {
                return Err(format!("Unsupported output encoding: {}", out_enc).into());
            }
            Some(encoding) => {
                let encoded_bytes = encoding.encode(&output_str).0;
                output_buf.write_all(&encoded_bytes)?;
            }
        },
    }

    output_buf.flush()?; // Flush buffer to ensure all data is written

    if let Some(input_file) = input_file {
        println!(
            "{BLUE}Conversion completed ({config}): {} -> {}{RESET}",
            input_file,
            output_file.unwrap_or(&"stdout".to_string())
        );
    } else {
        println!(
            "{BLUE}Conversion completed ({config}): <stdin> -> {}{RESET}",
            output_file.unwrap_or(&"stdout".to_string())
        );
    }

    Ok(())
}
