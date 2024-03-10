use core::panic;

use provider::TtsError;
use reqwest::header;
use tokio;

use crate::text_to_speach::provider;

use super::{TtsCapabilites, TtsClient, TtsClientBuilder};

pub struct OpenAiTtsClient {
    api_key: String,
    voice: String,
    speed: f32,
}

impl TtsClient for OpenAiTtsClient {
    async fn speak_to_file(self, text: String, path: String) -> Result<(), TtsError> {
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
                "voice": self.voice,
                "speed": self.speed,
            }))
            .send()
            .await
            .map_err(|e| TtsError::ConnectionFailure(e.to_string()))?;

        if response.status().is_success() {
            let bytes = response
                .bytes()
                .await
                .map_err(|e| TtsError::NoContent(e.to_string()))?;

            tokio::fs::write(path.to_owned(), bytes)
                .await
                .map_err(|e| TtsError::WriteToFileFailure(e.to_string()))?;
        } else {
            if 401 == response.status() {
                return Err(TtsError::Unauthorized(
                    response.text().await.unwrap_or("".to_owned()),
                ));
            }
            return Err(TtsError::Unknown(
                response.text().await.unwrap_or("".to_owned()),
            ));
        }
        Ok(())
    }
}

pub struct OpenAiTtsClientBuilder {
    api_key: Option<String>,
    voice: Option<String>,
    speed: Option<f32>,
}

impl TtsClientBuilder for OpenAiTtsClientBuilder {
    fn capabilities() -> &'static [TtsCapabilites] {
        &[
            TtsCapabilites::VoiceChoice,
            TtsCapabilites::SpeechSpeedChoice,
            TtsCapabilites::RequiresAuth,
        ]
    }

    fn default() -> Self {
        Self {
            api_key: None,
            voice: None,
            speed: None,
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
            speed: self.speed.unwrap_or(1.0),
        }
    }

    fn set_speed(self, speed: super::SpeechSpeed) -> Self {
        Self {
            speed: Some(match speed {
                super::SpeechSpeed::VeryVerySlow => 0.25,
                super::SpeechSpeed::VerySlow => 0.50,
                super::SpeechSpeed::Slow => 0.75,
                super::SpeechSpeed::Normal => 1.0,
                super::SpeechSpeed::Quick => 1.25,
                super::SpeechSpeed::VeryQuick => 1.5,
                super::SpeechSpeed::VeryVeryQuick => 2.0,
            }),
            ..self
        }
    }
}

#[cfg(test)]
mod test {
    use crate::text_to_speach::provider::SpeechSpeed;

    use super::*;

    #[tokio::test]
    async fn openai_it_test() {
        let client = OpenAiTtsClientBuilder::default()
            .authorize()
            .with_voice("alloy".to_owned())
            .set_speed(SpeechSpeed::Normal)
            .build();

        client
            .speak_to_file("sono un bot, ciao".to_owned(), "test.it.mp3".to_owned())
            .await;
    }
}
