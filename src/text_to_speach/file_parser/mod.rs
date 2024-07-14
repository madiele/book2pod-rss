use std::fs;

use anyhow::{anyhow, Result};

pub(crate) trait FileParser {
    fn parse_bytes(input: &[u8]) -> Result<Vec<String>>;
}

pub(crate) struct UniversalFileParser {}

impl UniversalFileParser {
    pub fn parse_file(file_path: &str) -> Result<Vec<String>> {
        match file_path {
            path if path.ends_with(".txt") => {
                let bytes = fs::read(path)?;
                TxtParser::parse_bytes(bytes.as_slice())
            }
            ext => Err(anyhow!("extension is unsupported: {ext}")),
        }
    }
}

struct TxtParser;

impl FileParser for TxtParser {
    fn parse_bytes(input: &[u8]) -> Result<Vec<String>> {
        let file_content = String::from_utf8_lossy(input);
        let paragraphs: Vec<String> = file_content
            .split("\n\n")
            .filter_map(|s| match s.trim() {
                "" => None,
                trimed => Some(trimed.to_string()),
            })
            .collect();
        Ok(paragraphs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn txt() {
        let test_string: &[u8] = r#"
        this is a test
        paragraph 1
        paragraph 1
        paragraph 1
        paragraph 1


        paragraph 2
        paragraph 2
        paragraph 2
        paragraph 2



        paragraph 3
        paragraph 3
        paragraph 3
        paragraph 3




        paragraph 4
        paragraph 4
        paragraph 4
        paragraph 4
        "#
        .as_bytes();
        let result = TxtParser::parse_bytes(test_string).unwrap();

        assert_eq!(4, result.len());
    }

    #[test]
    fn universal() {
        let test_string = r#"
        this is a test
        paragraph 1
        paragraph 1
        paragraph 1
        paragraph 1


        paragraph 2
        paragraph 2
        paragraph 2
        paragraph 2



        paragraph 3
        paragraph 3
        paragraph 3
        paragraph 3




        paragraph 4
        paragraph 4
        paragraph 4
        paragraph 4
        "#;
        let file_path = format!("{}.txt", stringify!(universal));

        std::fs::write(&file_path, test_string).expect("Failed to write test file");

        let result = UniversalFileParser::parse_file(file_path.as_str());

        std::fs::remove_file(file_path).expect("Failed to delete test file");

        assert_eq!(result.unwrap().len(), 4);
    }
}
