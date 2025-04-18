use std::fs::File;
use std::io::{self, BufWriter, Read, Write};

use clap::{Arg, ArgMatches, Command};
use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;

use opencc_jieba_rs;
use opencc_jieba_rs::OpenCC;

const CONFIG_LIST: [&str; 16] = [
    "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "t2twp", "t2hk",
    "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t",
];

fn read_input(
    input_file: Option<&String>,
    in_enc: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut input: Box<dyn Read> = match input_file {
        Some(file_name) => Box::new(File::open(file_name)?),
        None => {
            println!("\x1B[1;34mInput text, <ctrl-z> or <ctrl-d> to submit:\x1B[0m");
            Box::new(io::stdin())
        }
    };

    let mut input_str = String::new();
    match in_enc {
        "UTF-8" => {
            if let Some(file_name) = input_file {
                File::open(file_name)?.read_to_string(&mut input_str)?;
            } else {
                let mut buffer = [0; 1024];
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
                File::open(file_name)?.read_to_end(&mut bytes)?;
            } else {
                let mut buffer = [0; 1024];
                while let Ok(n) = input.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                }
            }

            let encoding = Encoding::for_label(in_enc.as_bytes()).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::Other,
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

fn write_output(
    output_file: Option<&String>,
    out_enc: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output: Box<dyn Write> = match output_file {
        Some(file_name) => Box::new(File::create(file_name)?),
        None => Box::new(io::stdout()),
    };

    let mut output_buf = BufWriter::new(output);

    match out_enc {
        "UTF-8" => write!(output_buf, "{}", content)?,
        _ => {
            let encoding = Encoding::for_label(out_enc.as_bytes())
                .ok_or_else(|| format!("Unsupported output encoding: {}", out_enc))?;
            let encoded_bytes = encoding.encode(content).0;
            output_buf.write_all(&encoded_bytes)?;
        }
    }

    output_buf.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";
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

        let in_enc = matches.get_one::<String>("in_enc").unwrap().as_str();
        let out_enc = matches.get_one::<String>("out_enc").unwrap().as_str();

        let input_str = read_input(input_file, in_enc)?;
        let output_str = OpenCC::new().convert(&input_str, config, punctuation);
        write_output(output_file, out_enc, &output_str)?;

        println!(
            "\x1B[1;34mConversion completed ({config}): {} -> {}\x1B[0m",
            input_file.unwrap_or(&"<stdin>".to_string()),
            output_file.unwrap_or(&"stdout".to_string())
        );

        Ok(())
    }

    fn handle_segment(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let input_file = matches.get_one::<String>("input");
        let output_file = matches.get_one::<String>("output");
        let delimiter = matches.get_one::<String>("delimiter").unwrap();
        let in_enc = matches.get_one::<String>("in_enc").unwrap();
        let out_enc = matches.get_one::<String>("out_enc").unwrap();

        let input_str = read_input(input_file, in_enc)?;
        let output_vec = OpenCC::new().jieba.cut(&input_str, true);
        let output_str = output_vec.join(delimiter);
        write_output(output_file, out_enc, &output_str)?;

        println!(
            "\x1B[1;34mSegmentation completed ({delimiter}): {} -> {}\x1B[0m",
            input_file.unwrap_or(&"<stdin>".to_string()),
            output_file.unwrap_or(&"stdout".to_string())
        );

        Ok(())
    }

    Ok(())
}
