extern crate clipboard;

use std::env;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use opencc_jieba_rs::{find_max_utf8_length, format_thousand, OpenCC};

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
        let config_vector = vec![
            "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "t2twp",
            "t2hk", "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t",
        ];
        if !config_vector.contains(&config.as_str()) {
            config = "auto".to_string()
        }
        if args.len() > 2 {
            if args[2] == "punct" {
                punct = true
            }
        }
    } else {
        config = "auto".to_string()
    }
    // Create a new clipboard context
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    // Attempt to read text from the clipboard
    match ctx.get_contents() {
        Ok(contents) => {
            // If successful, print the text to the console
            let output;
            let opencc = OpenCC::new();
            let input_code = opencc.zho_check(contents.as_str());
            if config == "auto" {
                match input_code {
                    1 => config = "t2s".to_string(),
                    2 => config = "s2tw".to_string(),
                    _ => config = "none".to_string(),
                }
            }

            let display_input;
            let display_output;
            let display_input_code;
            let display_output_code;
            let etc;
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

            if config == "s2t" {
                output = opencc.s2t(contents.as_str(), punct)
            } else if config == "s2tw" {
                output = opencc.s2tw(contents.as_str(), punct)
            } else if config == "s2twp" {
                output = opencc.s2twp(contents.as_str(), punct)
            } else if config == "s2hk" {
                output = opencc.s2hk(contents.as_str(), punct)
            } else if config == "t2s" {
                output = opencc.t2s(contents.as_str(), punct)
            } else if config == "t2tw" {
                output = opencc.t2tw(contents.as_str(), punct)
            } else if config == "t2twp" {
                output = opencc.t2twp(contents.as_str(), punct)
            } else if config == "t2hk" {
                output = opencc.t2hk(contents.as_str(), punct)
            } else if config == "tw2s" {
                output = opencc.tw2s(contents.as_str(), punct)
            } else if config == "tw2sp" {
                output = opencc.tw2sp(contents.as_str(), punct)
            } else if config == "tw2t" {
                output = opencc.tw2t(contents.as_str(), punct)
            } else if config == "tw2tp" {
                output = opencc.tw2tp(contents.as_str(), punct)
            } else if config == "hk2s" {
                output = opencc.hk2s(contents.as_str(), punct)
            } else if config == "hk2t" {
                output = opencc.hk2t(contents.as_str(), punct)
            } else if config == "t2jp" {
                output = opencc.t2jp(contents.as_str())
            } else if config == "jp2t" {
                output = opencc.jp2t(contents.as_str())
            } else {
                output = contents.clone()
            }

            if contents.len() > 600 {
                let max_utf8_length = find_max_utf8_length(&contents, 600);
                display_input = &contents[..max_utf8_length];
                etc = "...";
                display_output = &output[..max_utf8_length];
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
                        "{}(Output set to clipboard: {} chars.){}",
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
