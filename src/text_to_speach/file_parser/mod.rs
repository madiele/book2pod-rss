use std::{
    fs,
    io::{Cursor, Read, Seek},
    os::windows::fs::FileTimesExt,
    path::PathBuf,
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
    id: String,
    order: usize,
    name: String,
}

struct Cover {
    mime: String,
    content: Vec<u8>,
}

struct Metadata {
    authors: Vec<String>,
    title: Option<String>,
    publisher: Option<String>,
    description: Option<String>,
    lang: Option<String>,
}

trait FileParserV2<R>
where
    R: Read,
{
    fn from_reader(input: R) -> Result<Self>
    where
        Self: Sized;

    fn get_table_of_contents(&mut self) -> Result<Vec<Content>>;
    fn extract_text_from_content(&mut self, id: String) -> Result<String>;

    fn get_cover(&mut self) -> Option<Cover>;

    fn get_metadata(&mut self) -> Metadata;
}

#[derive(Debug)]
struct EpubParserV2<R>
where
    R: Read + Seek,
{
    doc: EpubDoc<R>,
}

impl<'a> FileParserV2<Cursor<&'a [u8]>> for EpubParserV2<Cursor<&'a [u8]>> {
    fn from_reader(input: Cursor<&'a [u8]>) -> Result<Self> {
        Ok(Self {
            doc: EpubDoc::from_reader(input)?,
        })
    }

    fn get_table_of_contents(&mut self) -> Result<Vec<Content>> {
        Ok(self
            .doc
            .toc
            .iter()
            .map(|a| Content {
                id: a.content.to_string_lossy().to_string(),
                order: a.play_order,
                name: a.label.clone(),
            })
            .collect())
    }

    fn extract_text_from_content(&mut self, id: String) -> Result<String> {
        self.doc.set_current_page(
            self.doc
                .resource_uri_to_chapter(&PathBuf::from(id))
                .ok_or(anyhow!("no chapter found"))?,
        );
        let (content, _mime) = self
            .doc
            .get_current_str()
            .ok_or(anyhow!("chapter has no content!"))?;
        Ok(content)
    }

    fn get_cover(&mut self) -> Option<Cover> {
        let (content, mime) = self.doc.get_cover()?;
        Some(Cover { mime, content })
    }

    fn get_metadata(&mut self) -> Metadata {
        let metadata = &self.doc.metadata;
        let title = metadata.get("title");
        let authors = metadata.get("creator");
        let publisher = metadata.get("publisher");
        let desc = metadata.get("description");
        let lang = metadata.get("language");

        Metadata {
            publisher: publisher.and_then(|x| x.first().cloned()),
            authors: authors.cloned().unwrap_or_default(),
            lang: lang.and_then(|l| l.first().cloned()),
            title: title.map(|a| a.first().cloned()).unwrap_or_default(),
            description: desc.map(|a| a.first().cloned()).unwrap_or_default(),
        }
    }
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
        let input = Cursor::new(input.as_slice());
        let result = EpubParserV2::from_reader(input).unwrap();
        panic!("{result:?}");
    }

    #[test]
    fn toc() {
        let mut file = File::open("test.epub").unwrap();
        let mut input = vec![];
        file.read_to_end(&mut input).unwrap();
        let input = Cursor::new(input.as_slice());
        let mut reader = EpubParserV2::from_reader(input).unwrap();
        let toc = reader.get_table_of_contents().unwrap();

        assert_eq!(toc[0].id, "epub\\text/titlepage.xhtml");
        assert_eq!(toc[0].name, "Titlepage");
        assert_eq!(toc[0].order, 1);

        assert_eq!(toc[1].id, "epub\\text/imprint.xhtml");
        assert_eq!(toc[1].name, "Imprint");
        assert_eq!(toc[1].order, 2);

        assert_eq!(toc[2].id, "epub\\text/poetry.xhtml#the-grave-of-the-slave");
        assert_eq!(toc[2].name, "The Grave of the Slave");
        assert_eq!(toc[2].order, 3);

        //.....

        assert_eq!(toc.len(), 19);
    }

    #[test]
    fn extract_text_from_content() {
        let mut file = File::open("test.epub").unwrap();
        let mut input = vec![];
        file.read_to_end(&mut input).unwrap();
        let input = Cursor::new(input.as_slice());
        let mut reader = EpubParserV2::from_reader(input).unwrap();
        let toc = reader.get_table_of_contents().unwrap();

        let srt = reader.extract_text_from_content(toc[0].id.clone()).unwrap();
        todo!()
    }

    #[test]
    fn get_metadata() {
        let mut file = File::open("test.epub").unwrap();
        let mut input = vec![];
        file.read_to_end(&mut input).unwrap();
        let input = Cursor::new(input.as_slice());
        let mut reader = EpubParserV2::from_reader(input).unwrap();

        let srt = reader.get_metadata();

        assert_eq!(srt.authors, vec!["Sarah Louisa Forten Purvis"]);
        assert_eq!(
            srt.description,
            Some("A collection of poems by Sarah Louisa Forten Purvis.".to_string())
        );
        assert_eq!(srt.lang, Some("en-US".to_string()));
        assert_eq!(srt.publisher, Some("Standard Ebooks".to_string()));
        assert_eq!(srt.title, Some("Poetry".to_string()));
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
