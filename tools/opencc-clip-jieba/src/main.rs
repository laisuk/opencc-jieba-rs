extern crate copypasta;

use std::collections::HashSet;
use std::env;

use copypasta::ClipboardContext;
use copypasta::ClipboardProvider;
use once_cell::sync::Lazy;
use opencc_jieba_rs::{find_max_utf8_length, format_thousand, OpenCC};

pub static CONFIG_LIST: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "t2twp", "t2hk",
        "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t",
    ]
    .iter()
    .cloned()
    .collect()
});

fn main() {
    const RED: &str = "\x1B[1;31m";
    const GREEN: &str = "\x1B[1;32m";
    const YELLOW: &str = "\x1B[1;33m";
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";

    let mut config;
    let mut punct = false;
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        config = args[1].clone();
        if config == "help" {
            println!("Opencc-Clip-Jieba Zho Converter version 1.0.0 Copyright (c) 2024 Bryan Lai");
            println!("Usage: opencc-clip-jieba [s2t|t2s|s2tw|tw2s|s2twp|tw2sp|s2hk|hk2s|t2tw|tw2t|t2twp|tw2t|tw2tp|t2hk|hk2t|jp2t|t2jp|auto|help] [punct]\n");
            return;
        }
        if !CONFIG_LIST.contains(&*config) {
            config = "auto".to_string()
        }
        punct = matches!(args.last(), Some(s) if s == "punct");

    } else {
        config = "auto".to_string()
    }
    // Create a new clipboard context
    let mut ctx: ClipboardContext = ClipboardContext::new().unwrap();
    // Attempt to read text from the clipboard
    match ctx.get_contents() {
        Ok(contents) => {
            // If successful, print the text to the console
            let display_input;
            let display_output;
            let display_input_code;
            let display_output_code;
            let etc;
            let output;
            let opencc = OpenCC::new();
            let input_code = opencc.zho_check(contents.as_str());

            if config == "auto" {
                match input_code {
                    1 => config = "t2s".to_string(),
                    2 => config = "s2t".to_string(),
                    _ => config = "none".to_string(),
                }
            }

            let input_length = contents.chars().collect::<Vec<_>>().len();

            if input_code == 0 || config == "t2jp" || config == "jp2t" {
                display_input_code = "Non-zho 其它";
                display_output_code = "Non-zho 其它";
            } else if config.starts_with('s') {
                display_input_code = "Simplified Chinese 简体";
                display_output_code = "Traditional Chinese 繁体";
            } else if config.ends_with('s') || config.ends_with('p') {
                display_input_code = "Traditional Chinese 繁体";
                display_output_code = "Simplified Chinese 简体";
            } else {
                display_input_code = "Traditional Chinese 繁体";
                display_output_code = "Traditional Chinese 繁体";
            }

            if CONFIG_LIST.contains(&config.as_str()) {
                output = opencc.convert(&contents, &config, punct);
            } else {
                output = contents.clone();
            }

            if contents.len() > 600 {
                let contents_max_utf8_length = find_max_utf8_length(&contents, 600);
                display_input = &contents[..contents_max_utf8_length];
                etc = "...";
                let output_max_utf8_length = find_max_utf8_length(&output, 600);
                display_output = &output[..output_max_utf8_length];
            } else {
                display_input = &contents;
                etc = "";
                display_output = &output;
            }

            println!("Opencc-Clip-Jieba Zho Converter version 1.0.0 Copyright (c) 2024 Bryan Lai");
            println!("Config: {}{}, {}{}", BLUE, config, punct, RESET);
            println!(
                "{}Clipboard Input ({}):{}\n{}{}{}{}\n",
                GREEN, &display_input_code, RESET, YELLOW, &display_input, etc, RESET
            );
            println!(
                "{}Converted Output ({}):{}\n{}{}{}{}",
                GREEN, &display_output_code, RESET, YELLOW, &display_output, etc, RESET
            );

            match ctx.set_contents(output) {
                Ok(..) => {
                    println!(
                        "{}(Output set to clipboard: {} chars){}",
                        BLUE,
                        format_thousand(input_length as i32),
                        RESET
                    )
                }
                Err(err) => {
                    eprintln!("{}Error set clipboard: {}{}", RED, err, RESET)
                }
            }
        }
        Err(err) => {
            // If an error occurs, print the error message
            eprintln!("{}No text in clipboard: {}{}", RED, err, RESET)
        }
    }
}
