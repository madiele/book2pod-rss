use core::panic;

use reqwest::header;
use tokio;

use super::{TtsCapabilites, TtsClient, TtsClientBuilder};

pub struct OpenAiTtsClient {
    api_key: String,
    voice: String,
}

impl TtsClient for OpenAiTtsClient {
    async fn speak_to_file(self, text: String, path: String) {
        let url = "https://api.openai.com/v1/audio/speech";
        let client = reqwest::Client::new();
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", self.api_key)).unwrap(),
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let response = client
            .post(url)
            .headers(headers)
            .json(&serde_json::json!({
                "model": "tts-1",
                "input": text,
                "voice": self.voice
            }))
            .send()
            .await
            .unwrap();

        if response.status().is_success() {
            let bytes = response.bytes().await.unwrap();
            tokio::fs::write(path.to_owned(), bytes).await.unwrap();
            println!("Speech saved to {}", path);
        } else {
            let error_msg = response.text().await.unwrap();
            println!("Failed to retrieve speech: {}", error_msg);
        }
    }
}

pub struct OpenAiTtsClientBuilder {
    api_key: Option<String>,
    voice: Option<String>,
}

impl TtsClientBuilder for OpenAiTtsClientBuilder {
    fn capabilities() -> &'static [TtsCapabilites] {
        &[TtsCapabilites::VoiceChoice, TtsCapabilites::RequiresAuth]
    }

    fn default() -> Self {
        Self {
            api_key: None,
            voice: None,
        }
    }

    fn authorize(self) -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            ..self
        }
    }

    fn with_voice(self, voice: String) -> Self {
        Self {
            voice: Some(voice),
            ..self
        }
    }

    fn for_language(self, _language: &locale_codes::language::LanguageInfo) -> Self {
        panic!()
    }

    fn build(self) -> OpenAiTtsClient {
        OpenAiTtsClient {
            api_key: self.api_key.expect("API key is required"),
            voice: self.voice.expect("Voice is required"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn openai_it_test() {
        let client = OpenAiTtsClientBuilder::default()
            .authorize()
            .with_voice("alloy".to_owned())
            .build();

        client
            .speak_to_file("sono un bot, ciao".to_owned(), "test.it.mp3".to_owned())
            .await;
    }
}

