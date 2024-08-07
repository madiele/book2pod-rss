use std::{
    collections::{HashMap, HashSet},
    default, fs,
    io::{Cursor, Read, Seek},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use epub::doc::EpubDoc;
use xml::{attribute::OwnedAttribute, reader::XmlEvent};

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
    fn extract_text_for_chapters(
        &mut self,
        from_id: String,
        to_id: Option<String>,
    ) -> Result<String>;

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

    fn extract_text_for_chapters(
        &mut self,
        from_id: String,
        to_id: Option<String>,
    ) -> Result<String> {
        let (from_uri, from_tag, to_tag, to_uri) = extract_positions(from_id, to_id)?;

        let mut table_of_contents = self.get_table_of_contents()?;
        table_of_contents.sort_by(|a, b| a.order.cmp(&b.order));

        let content_to_read =
            filter_page_to_iterate_over(table_of_contents.iter(), &from_uri, &to_uri);

        let mut final_string = "".to_owned();
        let mut skip_text = from_tag.is_some();
        let mut scanned_pages: HashSet<usize> = HashSet::default();
        for content in content_to_read {
            let content_split = content.id.splitn(2, '#').collect::<Vec<&str>>();
            let content_uri = &PathBuf::from(
                content_split
                    .first()
                    .ok_or(anyhow!("id is not correct {}", content.id))?,
            );
            let page = self
                .doc
                .resource_uri_to_chapter(content_uri)
                .ok_or(anyhow!("no chapter found"))?;

            if scanned_pages.contains(&page) {
                continue;
            } else {
                scanned_pages.insert(page);
            }

            self.doc.set_current_page(page);
            let (content, _mime) = self
                .doc
                .get_current_str()
                .ok_or(anyhow!("chapter has no content!"))?;

            let page_xml = xml::reader::EventReader::new(content.as_bytes());
            let mut element_stack: Vec<EpubElement> = vec![];
            for xml_event in page_xml {
                match xml_event {
                    Ok(XmlEvent::Characters(c)) => {
                        if !skip_text
                            && !element_stack.iter().any(|e| {
                                matches!(
                                    e.name.as_str(),
                                    "img"
                                        | "media"
                                        | "script"
                                        | "video"
                                        | "audio"
                                        | "object"
                                        | "embed"
                                        | "iframe"
                                        | "source"
                                        | "track"
                                        | "svg"
                                )
                            })
                        {
                            final_string.push_str(format!("{c}\n").as_str())
                        }
                    }
                    Ok(XmlEvent::StartElement {
                        name,
                        attributes,
                        namespace: _,
                    }) => {
                        if attributes.iter().any(|e| {
                            e.name.local_name.as_str() == "id"
                                && Some(e.value.clone()) == from_tag
                                && *content_uri == from_uri
                        }) {
                            skip_text = false;
                        }
                        if attributes.iter().any(|e| {
                            e.name.local_name.as_str() == "id"
                                && Some(e.value.clone()) == to_tag
                                && Some(content_uri) == to_uri.as_ref()
                        }) {
                            return Ok(final_string);
                        }
                        element_stack.push(EpubElement {
                            name: name.local_name,
                            attributes,
                        })
                    }
                    Ok(XmlEvent::EndElement { name }) => {
                        for i in 0..element_stack.len() - 1 {
                            if element_stack.get(i).unwrap().name == name.local_name {
                                element_stack.remove(i);
                                break;
                            }
                        }
                    }
                    Ok(_) => (),
                    Err(err) => return Err(anyhow!(err)),
                }
            }
        }

        Ok(final_string)
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

fn filter_page_to_iterate_over<'a>(
    iterator: std::slice::Iter<'a, Content>,
    from_uri: &PathBuf,
    to_uri: &Option<PathBuf>,
) -> Vec<&'a Content> {
    let content_to_read = iterator
        .skip_while(|e| {
            e.id.splitn(2, '#')
                .collect::<Vec<&str>>()
                .first()
                .is_some_and(|s| *s != from_uri.to_string_lossy())
        })
        .take_while(|e| {
            to_uri.is_none() || to_uri.clone().is_some_and(|u| e.id != u.to_string_lossy())
        })
        .collect::<Vec<&Content>>();
    content_to_read
}

fn extract_positions(
    from_id: String,
    to_id: Option<String>,
) -> Result<(PathBuf, Option<String>, Option<String>, Option<PathBuf>), anyhow::Error> {
    let from_split = from_id.splitn(2, '#').collect::<Vec<&str>>();
    let from_uri = &PathBuf::from(
        from_split
            .first()
            .ok_or(anyhow!("id is not correct {from_id}"))?,
    );
    let from_tag = from_split.get(1).map(|x| x.to_string());
    let mut to_tag = None;
    let mut to_uri = None;
    if let Some(id) = to_id {
        let to_split = id.splitn(2, '#').collect::<Vec<&str>>();
        to_uri = Some(PathBuf::from(
            to_split
                .first()
                .ok_or(anyhow!("id is not correct {from_id}"))?,
        ));
        to_tag = to_split.get(1).map(|x| x.to_string());
    };
    Ok((from_uri.to_owned(), from_tag, to_tag, to_uri))
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
    fn extract_text_from_content_from_tag_to_end() {
        let mut file = File::open("test.epub").unwrap();
        let mut input = vec![];
        file.read_to_end(&mut input).unwrap();
        let input = Cursor::new(input.as_slice());
        let mut reader = EpubParserV2::from_reader(input).unwrap();
        let toc = reader.get_table_of_contents().unwrap();

        let content_to_read = reader
            .extract_text_for_chapters(toc[18].id.clone(), None)
            .unwrap();
        assert!(content_to_read.starts_with("Uncopyright"));
    }

    #[test]
    fn extract_text_from_content_from_tag_to_tag() {
        let mut file = File::open("test.epub").unwrap();
        let mut input = vec![];
        file.read_to_end(&mut input).unwrap();
        let input = Cursor::new(input.as_slice());
        let mut reader = EpubParserV2::from_reader(input).unwrap();
        let toc = reader.get_table_of_contents().unwrap();

        let content_to_read = reader
            .extract_text_for_chapters(toc[13].id.clone(), Some(toc[14].id.clone()))
            .unwrap();
        assert!(content_to_read.starts_with("Hours of Childhood"));
        assert!(!content_to_read.contains("An Appeal to Woman"));
    }

    #[test]
    fn get_metadata() {
        let mut file = File::open("test.epub").unwrap();
        let mut input = vec![];
        file.read_to_end(&mut input).unwrap();
        let input = Cursor::new(input.as_slice());
        let mut reader = EpubParserV2::from_reader(input).unwrap();

        let metadata = reader.get_metadata();

        assert_eq!(metadata.authors, vec!["Sarah Louisa Forten Purvis"]);
        assert_eq!(
            metadata.description,
            Some("A collection of poems by Sarah Louisa Forten Purvis.".to_string())
        );
        assert_eq!(metadata.lang, Some("en-US".to_string()));
        assert_eq!(metadata.publisher, Some("Standard Ebooks".to_string()));
        assert_eq!(metadata.title, Some("Poetry".to_string()));
    }

    #[test]
    fn get_cover() {
        let mut file = File::open("test.epub").unwrap();
        let mut input = vec![];
        file.read_to_end(&mut input).unwrap();
        let input = Cursor::new(input.as_slice());
        let mut reader = EpubParserV2::from_reader(input).unwrap();

        let cover = reader.get_cover().unwrap();
        assert_eq!(cover.content.len(), 348151);
        assert_eq!(cover.mime, "image/jpeg");
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
