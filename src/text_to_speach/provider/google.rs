use std::str::FromStr;

use locale_codes::language::LanguageInfo;
use tts_rust::{languages::Languages, tts::GTTSClient};

use super::{TtsCapabilites, TtsClient, TtsClientBuilder, TtsError};

pub struct GTTSClientBuilder {
    volume: f32,
    language: Languages,
}

//this provider is only for tests, google max len is 100 chars, so it's useless

impl TtsClientBuilder<GTTSClient> for GTTSClientBuilder {
    fn capabilities() -> &'static [TtsCapabilites] {
        &[TtsCapabilites::LanguageChoice]
    }

    fn default() -> Self {
        Self {
            volume: 1.0,
            language: Languages::English,
        }
    }

    fn authorize(self) -> Self {
        self
    }

    fn with_voice(self, _voice: String) -> Self {
        self
    }

    fn for_language(mut self, language: &LanguageInfo) -> Self {
        self.language =
            Languages::from_str(language.short_code.to_owned().unwrap().as_str()).unwrap();
        self
    }

    fn build(self) -> GTTSClient {
        GTTSClient {
            volume: self.volume,
            language: self.language,
            tld: "com",
        }
    }

    fn set_speed(self, _speed: super::SpeechSpeed) -> Self {
        self
    }
}

impl TtsClient for GTTSClient {
    async fn speak_to_file(self, text: String, path: String) -> Result<(), TtsError> {
        let result = self.save_to_file(text.as_str(), path.as_str());
        result.map_err(TtsError::Unknown)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn google_test_it() {
        let client = GTTSClientBuilder::default()
            .for_language(locale_codes::language::lookup("it").unwrap())
            .build();

        client
            .speak_to_file("ciao ciao ciao".to_owned(), "test.it.mp3".to_owned())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn google_test_en() {
        let client = GTTSClientBuilder::default()
            .for_language(locale_codes::language::lookup("en").unwrap())
            .build();

        client
            .speak_to_file("hello hello hello".to_owned(), "test.en.mp3".to_owned())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn google_test_es() {
        let client = GTTSClientBuilder::default()
            .for_language(locale_codes::language::lookup("es").unwrap())
            .build();

        client
            .speak_to_file("hola hola hola".to_owned(), "test.es.mp3".to_owned())
            .await
            .unwrap();
    }
}
