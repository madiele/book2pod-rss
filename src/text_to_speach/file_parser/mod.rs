use std::{any, error::Error, fs, path};

use anyhow::{anyhow, Result};
use epub::doc::EpubDoc;
use xml::reader::XmlEvent;

trait FileParser {
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
            path if path.ends_with(".epub") => {
                let bytes = fs::read(path)?;
                EpubParser::parse_bytes(bytes.as_slice())
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

struct EpubParser;

impl FileParser for EpubParser {
    fn parse_bytes(input: &[u8]) -> Result<Vec<String>> {
        //TODO need to refactor as we cant use bytes
        //TODO need to clean up the resulting xml from all the tags
        let mut res: Vec<String> = vec![];
        let mut doc = EpubDoc::new("test.epub").unwrap();
        loop {
            let read = doc.get_current_str().ok_or(anyhow!("can't read page"))?.0;
            let parsd = xml::reader::EventReader::new(read.as_bytes());
            let mut final_string: String = "".to_owned();
            for event in parsd {
                match event {
                    Ok(XmlEvent::Characters(c)) => {
                        final_string.push_str(format!("{} ", c).as_str())
                    }
                    Ok(_) => (),
                    Err(err) => return Err(anyhow!(err)),
                }
            }
            res.push(final_string);
            if !doc.go_next() {
                break;
            }
        }
        Ok(res)
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
    fn epub() {
        let result = EpubParser::parse_bytes("".as_bytes());
        panic!("{result:?}");
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
