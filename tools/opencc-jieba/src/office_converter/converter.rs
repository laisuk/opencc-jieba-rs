use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use regex::Regex;
use tempfile::tempdir;
use zip::{
    write::{ExtendedFileOptions, FileOptions},
    CompressionMethod, ZipArchive, ZipWriter,
};

use opencc_jieba_rs::OpenCC;

pub struct ConversionResult {
    pub success: bool,
    pub message: String,
}

pub struct OfficeConverter;

impl OfficeConverter {
    pub fn convert(
        input_path: &str,
        output_path: &str,
        format: &str,
        helper: &OpenCC,
        config: &str,
        punctuation: bool,
        keep_font: bool,
    ) -> io::Result<ConversionResult> {
        let temp_dir = tempdir()?;
        let temp_path = temp_dir.path();

        let file = File::open(input_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let raw_name = file.name().replace('\\', "/");
            let rel_path = Path::new(&raw_name);

            if rel_path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            }) {
                continue;
            }

            let out_path = temp_path.join(rel_path);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut out_file = File::create(&out_path)?;
            io::copy(&mut file, &mut out_file)?;
        }

        for xml_file in get_target_xml_paths(format, temp_path) {
            if !xml_file.exists() {
                continue;
            }
            let mut content = String::new();
            File::open(&xml_file)?.read_to_string(&mut content)?;

            let mut font_map = HashMap::new();
            if keep_font {
                mask_font(&mut content, format, &mut font_map);
            }

            let mut converted = helper.convert(&content, config, punctuation);
            if keep_font {
                for (marker, original) in font_map {
                    converted = converted.replace(&marker, &original);
                }
            }

            File::create(&xml_file)?.write_all(converted.as_bytes())?;
        }

        if Path::new(output_path).exists() {
            fs::remove_file(output_path)?;
        }

        let zip_file = File::create(output_path)?;
        let mut zip_writer = ZipWriter::new(zip_file);

        for entry in walkdir::WalkDir::new(temp_path) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let mut buffer = Vec::new();
                File::open(path)?.read_to_end(&mut buffer)?;

                let relative_path = path
                    .strip_prefix(temp_path)
                    .map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("strip_prefix failed: {}", e))
                    })?
                    .to_string_lossy()
                    .replace('\\', "/");

                let is_mimetype = relative_path == "mimetype";
                let method = if is_mimetype {
                    CompressionMethod::Stored
                } else {
                    CompressionMethod::Deflated
                };
                let options: FileOptions<'_, ExtendedFileOptions> =
                    FileOptions::default().compression_method(method);

                zip_writer.start_file(&relative_path, options)?;
                zip_writer.write_all(&buffer)?;
            }
        }

        zip_writer.finish()?;

        Ok(ConversionResult {
            success: true,
            message: "✅ Conversion completed.".to_string(),
        })
    }
}

fn get_target_xml_paths(format: &str, base_dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    match format {
        "docx" => result.push(base_dir.join("word/document.xml")),
        "xlsx" => result.push(base_dir.join("xl/sharedStrings.xml")),
        "pptx" => {
            for entry in walkdir::WalkDir::new(base_dir.join("ppt")) {
                let path = entry.unwrap().path().to_path_buf();
                let name = path.file_name().unwrap().to_string_lossy();
                let path_str = path.to_string_lossy();
                if name.contains("slide") || path_str.contains("notesSlide") {
                    result.push(path);
                }
            }
        }
        "odt" | "ods" | "odp" => result.push(base_dir.join("content.xml")),
        "epub" => {
            for entry in walkdir::WalkDir::new(base_dir) {
                let path = entry.unwrap().path().to_path_buf();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "xhtml" | "opf" | "ncx" | "html") {
                    result.push(path);
                }
            }
        }
        _ => {}
    }
    result
}

fn mask_font(xml: &mut String, format: &str, font_map: &mut HashMap<String, String>) {
    let pattern = match format {
        "docx" => r#"(w:(?:eastAsia|ascii|hAnsi|cs)=")(.*?)(")"#,
        "xlsx" => r#"(val=")(.*?)(")"#,
        "pptx" => r#"(typeface=")(.*?)(")"#,
        "odt" | "ods" | "odp" => {
            r#"((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=['"])([^'"]+)(['"])"#
        }
        "epub" => r#"(font-family\s*:\s*)([^;"']+)"#,
        _ => return,
    };
    let re = Regex::new(pattern).unwrap();
    let mut counter = 0;
    let mut result_str = String::new();
    let mut last_end = 0;
    for caps in re.captures_iter(xml) {
        let marker = format!("__F_O_N_T_{}__", counter);
        counter += 1;
        font_map.insert(marker.clone(), caps[2].to_string());
        let mat = caps.get(0).unwrap();
        result_str.push_str(&xml[last_end..mat.start()]);
        result_str.push_str(&caps[1]);
        result_str.push_str(&marker);
        if caps.len() > 3 {
            result_str.push_str(&caps[3]);
        }
        last_end = mat.end();
    }
    result_str.push_str(&xml[last_end..]);
    *xml = result_str;
}
