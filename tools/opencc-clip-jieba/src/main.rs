extern crate copypasta;

use clap::{
    builder::{StringValueParser, TypedValueParser, ValueParser},
    Arg, ArgAction, Command,
};
use copypasta::{ClipboardContext, ClipboardProvider};
use opencc_jieba_rs::{find_max_utf8_length, OpenCC, OpenccConfig};

fn config_value_parser() -> ValueParser {
    ValueParser::new(StringValueParser::new().try_map(|s| {
        if s.eq_ignore_ascii_case("auto") {
            Ok(String::from("auto"))
        } else {
            OpenccConfig::try_from(s.as_str())
                .map(OpenccConfig::as_str)
                .map(str::to_owned)
                .map_err(|_| format!("invalid config: {s}"))
        }
    }))
}

#[inline]
fn parse_config_or_auto(s: &str) -> Option<OpenccConfig> {
    if s.eq_ignore_ascii_case("auto") {
        None
    } else {
        OpenccConfig::parse(s)
    }
}

#[inline]
fn is_japanese_config(config: OpenccConfig) -> bool {
    matches!(config, OpenccConfig::T2jp | OpenccConfig::Jp2t)
}

fn main() {
    const RED: &str = "\x1B[1;31m";
    const GREEN: &str = "\x1B[1;32m";
    const YELLOW: &str = "\x1B[1;33m";
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";

    let matches = Command::new("opencc-clip-jieba")
        .about("Clipboard Simplified ⇄ Traditional Chinese converter using opencc-jieba-rs")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_parser(config_value_parser())
                .default_value("auto")
                .help("Conversion configuration (default: auto)"),
        )
        .arg(
            Arg::new("punct")
                .short('p')
                .long("punct")
                .action(ArgAction::SetTrue)
                .help("Enable punctuation conversion"),
        )
        .after_help(
            "Examples:
  opencc-clip-jieba                 # auto, punctuation OFF
  opencc-clip-jieba -c s2t          # force s2t
  opencc-clip-jieba -c s2t --punct  # force s2t, punctuation ON
  opencc-clip-jieba -p              # auto with punctuation ON",
        )
        .get_matches();

    let cfg_str = matches.get_one::<String>("config").unwrap().as_str();
    let mut conversion_type = parse_config_or_auto(cfg_str);
    let use_punctuation = matches.get_flag("punct");

    // Clipboard context
    let mut ctx: ClipboardContext = match ClipboardContext::new() {
        Ok(context) => context,
        Err(err) => {
            eprintln!("{}Error creating clipboard context: {}{}", RED, err, RESET);
            return;
        }
    };

    match ctx.get_contents() {
        Ok(contents) => {
            let opencc = OpenCC::new();
            let input_code = opencc.zho_check(&contents);

            if conversion_type.is_none() {
                conversion_type = match input_code {
                    1 => Some(OpenccConfig::T2s), // Traditional → Simplified
                    2 => Some(OpenccConfig::S2t), // Simplified → Traditional
                    _ => None,
                };
            }

            let (display_input_code, display_output_code) =
                if input_code == 0 || conversion_type.is_some_and(is_japanese_config) {
                    ("Non-zho 其它", "Non-zho 其它")
                } else if conversion_type
                    .map(OpenccConfig::as_str)
                    .is_some_and(|cfg| cfg.starts_with('s'))
                {
                    ("Simplified Chinese 简体", "Traditional Chinese 繁体")
                } else if conversion_type
                    .map(OpenccConfig::as_str)
                    .is_some_and(|cfg| cfg.ends_with('s') || cfg.ends_with("sp"))
                {
                    ("Traditional Chinese 繁体", "Simplified Chinese 简体")
                } else {
                    ("Traditional Chinese 繁体", "Traditional Chinese 繁体")
                };

            let output = if let Some(config) = conversion_type {
                opencc.convert_with_config(&contents, config, use_punctuation)
            } else {
                contents.clone()
            };

            let (display_input, display_output, ellipsis) = if contents.len() > 600 {
                let contents_max_utf8_length = find_max_utf8_length(&contents, 600);
                let output_max_utf8_length = find_max_utf8_length(&output, 600);
                (
                    &contents[..contents_max_utf8_length],
                    &output[..output_max_utf8_length],
                    "...",
                )
            } else {
                (contents.as_str(), output.as_str(), "")
            };

            eprintln!(
                "opencc-clip-jieba Simplified/Traditional Chinese Text Converter © 2026 laisuk Lai"
            );
            eprintln!(
                "Config: {}{}, punct: {}{}",
                BLUE,
                conversion_type.map(OpenccConfig::as_str).unwrap_or("auto"),
                use_punctuation,
                RESET
            );
            eprintln!(
                "{}Clipboard Input ({}):\n{}{}{}\n",
                GREEN, display_input_code, YELLOW, display_input, ellipsis
            );
            eprintln!(
                "{}Converted Output ({}):\n{}{}{}{}",
                GREEN, display_output_code, YELLOW, display_output, ellipsis, RESET
            );

            if let Err(err) = ctx.set_contents(output) {
                eprintln!("{}Error setting clipboard: {}{}", RED, err, RESET);
            } else {
                let input_length = contents.chars().count();
                eprintln!(
                    "{}(Output set to clipboard: {} chars){}",
                    BLUE,
                    format_thousand(input_length),
                    RESET
                );
            }
        }
        Err(err) => {
            eprintln!("{}No text in clipboard: {}{}", RED, err, RESET)
        }
    }
}

pub fn format_thousand(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);

    let bytes = s.as_bytes();
    let len = bytes.len();

    for (i, &b) in bytes.iter().enumerate() {
        out.push(b as char);

        let remaining = len - i - 1;
        if remaining > 0 && remaining % 3 == 0 {
            out.push(',');
        }
    }
    out
}
