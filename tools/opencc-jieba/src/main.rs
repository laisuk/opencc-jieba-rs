use clap::builder::{StringValueParser, TypedValueParser, ValueParser};
use clap::{Arg, ArgMatches, Command};
use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;
use opencc_jieba_rs::{OpenCC, OpenccConfig};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, IsTerminal, Read, Write};
use std::path::Path;
use std::sync::OnceLock;

mod office_converter;
use office_converter::OfficeConverter;

const BLUE: &str = "\x1B[1;34m";
const RESET: &str = "\x1B[0m";

const PROMPT_CONVERT: &str = concat!(
    "\x1B[1;34m",
    "Input text to convert, <ctrl-z> or <ctrl-d> to submit:",
    "\x1B[0m"
);

const PROMPT_SEGMENT: &str = concat!(
    "\x1B[1;34m",
    "Input text to segment, <ctrl-z> or <ctrl-d> to submit:",
    "\x1B[0m"
);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("opencc-jieba")
        .about(format!(
            "{}OpenCC Jieba Rust: Command Line Open Chinese Converter{}",
            BLUE, RESET
        ))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("convert")
                .about(format!(
                    "{}opencc-jieba convert: Convert Chinese Traditional/Simplified text using OpenCC{}",
                    BLUE, RESET
                ))
                .args(common_args())
                .args(enc_args()),
        )
        .subcommand(
            Command::new("office")
                .about(format!(
                    "{}opencc-jieba office: Convert Office or EPUB documents using OpenCC{}",
                    BLUE, RESET
                ))
                .args(common_args())
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .value_name("ext")
                        .help(
                            "Force office document format <ext>: docx, xlsx, pptx, odt, ods, odp, epub",
                        ),
                )
                .arg(
                    Arg::new("keep_font")
                        .long("keep-font")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preserve original font styles"),
                )
                .arg(
                    Arg::new("auto_ext")
                        .long("auto-ext")
                        .action(clap::ArgAction::SetTrue)
                        .help("Infer format from file extension"),
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
                        .help("Delimiter character for segmented text (use \" \" for space)")
                        .required(false)
                        .default_value("/"),
                )
                .arg(
                    Arg::new("separator")
                        .short('s')
                        .long("separator")
                        .value_name("character")
                        .help("Separator character for segmented mode=tag (use \" \" for space)")
                        .required(false)
                        .default_value("/"),
                )
                .arg(
                    Arg::new("mode")
                        .short('m')
                        .long("mode")
                        .value_name("mode")
                        .value_parser(["cut", "search", "all", "tag"])
                        .default_value("cut")
                        .help("Segmentation mode: cut | search | all | tag"),
                )
                .arg(
                    Arg::new("no_hmm")
                        .long("no-hmm")
                        .action(clap::ArgAction::SetTrue)
                        .help("Disable HMM for segmentation and tagging"),
                )
                .args(enc_args()),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("convert", sub_matches)) => handle_convert(sub_matches)?,
        Some(("office", sub_matches)) => handle_office(sub_matches)?,
        Some(("segment", sub_matches)) => handle_segment(sub_matches)?,
        _ => unreachable!("Clap ensures only valid subcommands are passed"),
    }

    Ok(())
}

fn get_supported_configs() -> &'static str {
    static SUPPORTED: OnceLock<String> = OnceLock::new();
    SUPPORTED.get_or_init(|| {
        let mut s = String::with_capacity(128);
        for (i, cfg) in OpenccConfig::ALL.iter().enumerate() {
            if i > 0 {
                s.push_str(" | ");
            }
            s.push_str(cfg.as_str());
        }
        s
    })
}

fn config_value_parser() -> ValueParser {
    ValueParser::new(StringValueParser::new().try_map(|s| {
        OpenccConfig::try_from(s.as_str())
            .map(OpenccConfig::as_str)
            .map(str::to_owned)
            .map_err(|_| format!("\nSupported configs: {}", get_supported_configs()))
    }))
}

fn common_args() -> Vec<Arg> {
    vec![
        Arg::new("input")
            .short('i')
            .long("input")
            .value_name("file")
            .help("Input <file> (use stdin if omitted for non-office documents)"),
        Arg::new("output")
            .short('o')
            .long("output")
            .value_name("file")
            .help("Output <file> (use stdout if omitted for non-office documents)"),
        Arg::new("config")
            .short('c')
            .long("config")
            .required(true)
            .value_name("config")
            .value_parser(config_value_parser())
            .help(format!(
                "Conversion configuration ({})",
                get_supported_configs()
            )),
        Arg::new("punct")
            .short('p')
            .long("punct")
            .action(clap::ArgAction::SetTrue)
            .help("Enable punctuation conversion"),
    ]
}

fn enc_args() -> Vec<Arg> {
    vec![
        Arg::new("in_enc")
            .long("in-enc")
            .value_name("encoding")
            .default_value("UTF-8")
            .global(true)
            .help("Encoding for input: UTF-8|GB2312|GBK|gb18030|BIG5"),
        Arg::new("out_enc")
            .long("out-enc")
            .value_name("encoding")
            .default_value("UTF-8")
            .global(true)
            .help("Encoding for output: UTF-8|GB2312|GBK|gb18030|BIG5"),
    ]
}

fn handle_convert(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = matches.get_one::<String>("input");
    let output_file = matches.get_one::<String>("output");
    let config = matches.get_one::<String>("config").unwrap();
    let in_enc = matches.get_one::<String>("in_enc").unwrap();
    let out_enc = matches.get_one::<String>("out_enc").unwrap();
    let punctuation = matches.get_flag("punct");

    let is_console = input_file.is_none();
    let mut input: Box<dyn Read> = match input_file {
        Some(file_name) => Box::new(BufReader::new(File::open(file_name)?)),
        None => {
            if io::stdin().is_terminal() {
                eprintln!("{PROMPT_CONVERT}");
            }
            Box::new(BufReader::new(io::stdin().lock()))
        }
    };

    let mut buffer = read_input(&mut *input, is_console)?;
    if should_remove_bom(in_enc, out_enc) {
        remove_utf8_bom(&mut buffer);
    }

    let input_str = decode_input(&buffer, in_enc)?;
    let output_str = OpenCC::new().convert(&input_str, config, punctuation);

    let (is_console_output, mut output) = open_output(output_file)?;

    let final_output = if is_console_output && !output_str.ends_with('\n') {
        format!("{output_str}\n")
    } else {
        output_str
    };

    encode_and_write_output(&final_output, out_enc, &mut *output)?;
    output.flush()?;

    Ok(())
}

fn handle_office(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let office_extensions: HashSet<&'static str> =
        ["docx", "xlsx", "pptx", "odt", "ods", "odp", "epub"].into();

    let input_file = matches
        .get_one::<String>("input")
        .ok_or("❌  Input file is required for office mode")?;

    let output_file = matches.get_one::<String>("output");
    let config = matches.get_one::<String>("config").unwrap();
    let punctuation = matches.get_flag("punct");
    let keep_font = matches.get_flag("keep_font");
    let auto_ext = matches.get_flag("auto_ext");
    let format = matches.get_one::<String>("format").map(String::as_str);

    let office_format = match format {
        Some(f) => f.to_lowercase(),
        None => {
            if auto_ext {
                let ext = Path::new(input_file)
                    .extension()
                    .and_then(|e| e.to_str())
                    .ok_or("❌  Cannot infer file extension")?;
                if office_extensions.contains(ext) {
                    ext.to_string()
                } else {
                    return Err(format!("❌  Unsupported Office extension: .{ext}").into());
                }
            } else {
                return Err("❌  Please provide --format or use --auto-ext".into());
            }
        }
    };

    let helper = OpenCC::new();

    let final_output = match output_file {
        Some(path) => {
            if auto_ext
                && Path::new(path).extension().is_none()
                && office_extensions.contains(office_format.as_str())
            {
                format!("{path}.{}", office_format)
            } else {
                path.to_string()
            }
        }
        None => {
            let input_path = Path::new(input_file);
            let file_stem = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("converted");
            let ext = office_format.as_str();
            let parent = input_path.parent().unwrap_or_else(|| ".".as_ref());

            let file_stem_converted = helper.convert(file_stem, config, punctuation);
            let final_stem = if auto_ext {
                format!("{file_stem_converted}_converted")
            } else {
                format!("{file_stem}_converted")
            };

            parent
                .join(format!("{final_stem}.{ext}"))
                .to_string_lossy()
                .to_string()
        }
    };

    match OfficeConverter::convert(
        input_file,
        &final_output,
        &office_format,
        &helper,
        config,
        punctuation,
        keep_font,
    ) {
        Ok(result) if result.success => {
            eprintln!("{}\n📁  Output saved to: {}", result.message, final_output);
        }
        Ok(result) => {
            eprintln!("❌  Office document conversion failed: {}", result.message);
        }
        Err(e) => {
            eprintln!("❌  Error: {}", e);
        }
    }

    Ok(())
}

fn handle_segment(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = matches.get_one::<String>("input");
    let output_file = matches.get_one::<String>("output");
    let delimiter = matches.get_one::<String>("delimiter").unwrap();
    let separator = matches.get_one::<String>("separator").unwrap();
    let mode = matches.get_one::<String>("mode").unwrap();
    let in_enc = matches.get_one::<String>("in_enc").unwrap();
    let out_enc = matches.get_one::<String>("out_enc").unwrap();
    let hmm = !matches.get_flag("no_hmm");

    let is_console = input_file.is_none();
    let mut input: Box<dyn Read> = match input_file {
        Some(file_name) => Box::new(BufReader::new(File::open(file_name)?)),
        None => {
            if io::stdin().is_terminal() {
                eprintln!("{PROMPT_SEGMENT}");
            }
            Box::new(BufReader::new(io::stdin().lock()))
        }
    };

    let mut buffer = read_input(&mut *input, is_console)?;
    if should_remove_bom(in_enc, out_enc) {
        remove_utf8_bom(&mut buffer);
    }

    let mut input_str = decode_input(&buffer, in_enc)?;
    if is_console {
        input_str = normalize_line_endings(&input_str);
        // Remove trailing submit newline from interactive console input
        input_str = input_str.trim_end_matches('\n').to_string();
    }

    let opencc = OpenCC::new();

    let output_str = match mode.as_str() {
        "search" => opencc.jieba_cut_for_search(&input_str, hmm).join(delimiter),
        "all" => opencc.jieba_cut_all(&input_str).join(delimiter),
        "tag" => {
            let pairs = opencc.jieba_tag(&input_str, hmm);
            let mut out = String::new();

            for (i, (w, t)) in pairs.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(delimiter);
                }
                out.push_str(&w);
                out.push_str(&separator);
                out.push_str(&t);
            }

            out
        }
        _ => opencc.jieba_cut(&input_str, hmm).join(delimiter),
    };

    let (is_console_output, mut output) = open_output(output_file)?;

    let final_output = if is_console_output && !output_str.ends_with('\n') {
        format!("{output_str}\n")
    } else {
        output_str
    };

    encode_and_write_output(&final_output, out_enc, &mut *output)?;
    output.flush()?;

    Ok(())
}

fn read_input(input: &mut dyn Read, is_console: bool) -> io::Result<Vec<u8>> {
    if is_console {
        let mut reader = BufReader::new(input);
        let mut text = String::new();
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            text.push_str(&line);
        }

        Ok(text.into_bytes())
    } else {
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
}

fn decode_input(buffer: &[u8], enc: &str) -> io::Result<String> {
    if enc.eq_ignore_ascii_case("UTF-8") {
        return Ok(String::from_utf8_lossy(buffer).into_owned());
    }

    let encoding = Encoding::for_label(enc.as_bytes()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported encoding: {enc}"),
        )
    })?;

    let mut reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(buffer);

    let mut decoded = String::new();
    reader.read_to_string(&mut decoded)?;
    Ok(decoded)
}

fn open_output(output_file: Option<&String>) -> io::Result<(bool, Box<dyn Write>)> {
    let is_console_output = output_file.is_none();

    let output: Box<dyn Write> = match output_file {
        Some(file_name) => Box::new(BufWriter::new(File::create(file_name)?)),
        None => Box::new(BufWriter::new(io::stdout().lock())),
    };

    Ok((is_console_output, output))
}

fn encode_and_write_output(output_str: &str, enc: &str, output: &mut dyn Write) -> io::Result<()> {
    if enc.eq_ignore_ascii_case("UTF-8") {
        output.write_all(output_str.as_bytes())?;
        return Ok(());
    }

    let encoding = Encoding::for_label(enc.as_bytes()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported output encoding: {enc}"),
        )
    })?;

    let (encoded, _, _) = encoding.encode(output_str);
    output.write_all(&encoded)?;
    Ok(())
}

fn should_remove_bom(in_enc: &str, out_enc: &str) -> bool {
    in_enc.eq_ignore_ascii_case("UTF-8") && !out_enc.eq_ignore_ascii_case("UTF-8")
}

fn remove_utf8_bom(input: &mut Vec<u8>) {
    if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        input.drain(..3);
    }
}

fn normalize_line_endings(s: &str) -> String {
    if !s.contains('\r') {
        return s.to_string(); // fast path
    }

    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\r' {
            if matches!(chars.peek(), Some('\n')) {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }

    out
}
