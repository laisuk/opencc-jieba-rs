use opencc_jieba_rs::{CustomDictFileSpec, CustomDictMode, DictSlot};
use std::path::PathBuf;

pub fn parse_custom_dict_spec(
    arg: &str,
) -> Result<CustomDictFileSpec<PathBuf>, Box<dyn std::error::Error>> {
    let mut parts = arg.splitn(3, ':');

    let slot = parts.next().ok_or("Missing custom dict slot")?;
    let mode = parts.next().ok_or("Missing custom dict mode")?;
    let file = parts.next().ok_or("Missing custom dict file")?.trim();

    if file.is_empty() {
        return Err("Custom dictionary path cannot be empty".into());
    }

    let slot = DictSlot::from_name_ignore_ascii_case(slot)
        .ok_or_else(|| format!("Unknown custom dictionary slot: {slot}"))?;

    let mode = match mode.trim().to_ascii_lowercase().as_str() {
        "append" => CustomDictMode::Append,
        "override" => CustomDictMode::Override,
        other => return Err(format!("Unknown custom dict mode: {other}").into()),
    };

    let path = PathBuf::from(file);

    if !path.is_file() {
        return Err(format!("Custom dictionary file not found: {}", path.display()).into());
    }

    Ok(CustomDictFileSpec {
        slot,
        files: vec![path],
        mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_custom_dict_slots_case_insensitively() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/my_hk_dict.txt");

        let arg = format!(" jpscharactersrev : APPEND : {} ", path.display());

        let spec = parse_custom_dict_spec(&arg).unwrap();

        assert_eq!(spec.slot, DictSlot::JPSCharactersRev);
        assert_eq!(spec.mode, CustomDictMode::Append);
        assert_eq!(spec.files, [path]);
    }

    #[test]
    fn rejects_empty_custom_dictionary_paths() {
        assert!(parse_custom_dict_spec("STPhrases:append:   ").is_err());
    }

    // Ignored: Slot alias will be rejected upon v0.12.x
    #[test]
    #[ignore]
    fn rejects_physical_japanese_dictionary_filename_stems() {
        let error =
            parse_custom_dict_spec("JPShinjitaiCharactersRev:override:custom.txt").unwrap_err();

        assert!(error
            .to_string()
            .contains("Unknown custom dictionary slot: JPShinjitaiCharactersRev"));
    }
}
