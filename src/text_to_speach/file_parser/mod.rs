use std::{
    fs,
    io::{Cursor, Read},
};

use anyhow::{anyhow, Result};
use epub::doc::EpubDoc;
use xml::{attribute::OwnedAttribute, name::OwnedName, namespace::Namespace, reader::XmlEvent};

trait FileParser<R>
where
    R: Read,
{
    fn parse_bytes(input: R) -> Result<Vec<String>>;
}

struct Content {
    Id: String,
    Order: usize,
    Name: String,
    CharCount: usize,
}

struct Cover {
    Name: String,
    Extension: String,
    Content: Vec<u8>,
}

trait FileParserV2<R>
where
    R: Read,
{
    fn from_reader(input: R) -> Self;
    fn get_table_of_contets(self) -> Result<Vec<Content>>;
    fn extract_text_from_content(self, id: String) -> Result<String>;
    fn get_cover(self) -> Option<Cover>;
    fn set_cover(self, input: Cover) -> Result<()>;
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

impl FileParser<&[u8]> for TxtParser {
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

#[derive(Default)]
struct EpubElement {
    name: String,
    attributes: Vec<OwnedAttribute>,
}

impl FileParser<&[u8]> for EpubParser {
    fn parse_bytes(input: &[u8]) -> Result<Vec<String>> {
        let mut res: Vec<String> = vec![];
        let mut doc = EpubDoc::from_reader(Cursor::new(input))?;
        loop {
            let page = doc.get_current_str().ok_or(anyhow!("can't read page"))?.0;
            let page_xml = xml::reader::EventReader::new(page.as_bytes());
            let mut final_string: String = "".to_owned();
            let mut elementStack: Vec<EpubElement> = vec![];
            for xml_event in page_xml {
                match xml_event {
                    Ok(XmlEvent::Characters(c)) => final_string.push_str(format!("{c}\n").as_str()),
                    Ok(XmlEvent::StartElement {
                        name,
                        attributes,
                        namespace: _,
                    }) => elementStack.push(
                        (EpubElement {
                            name: name.local_name,
                            attributes,
                        }),
                    ),
                    Ok(XmlEvent::EndElement { name }) => {
                        for i in 0..elementStack.len() - 1 {
                            if elementStack.get(i).unwrap().name == name.local_name {
                                elementStack.remove(i);
                                break;
                            }
                        }
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
    use std::{fs::File, io::Read};

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
        let mut file = File::open("test.epub").unwrap();
        let mut input = vec![];
        file.read_to_end(&mut input).unwrap();
        let result = EpubParser::parse_bytes(input.as_slice()).unwrap();
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
