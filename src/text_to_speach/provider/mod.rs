use std::{error::Error, fmt::Display};

mod google;
mod openai;

#[trait_variant::make(HttpService: Send)]
pub trait TtsClient {
    async fn speak_to_file(self, text: String, path: String) -> Result<(), TtsError>;
}

pub trait TtsClientBuilder {
    fn capabilities() -> &'static [TtsCapabilites];
    fn default() -> Self;
    fn authorize(self) -> Self;
    fn with_voice(self, voice: String) -> Self;
    fn set_speed(self, speed: SpeechSpeed) -> Self;
    fn for_language(self, language: &locale_codes::language::LanguageInfo) -> Self;
    fn build(self) -> impl TtsClient;
}

pub enum TtsCapabilites {
    LanguageChoice,
    VoiceChoice,
    RequiresAuth,
    SpeechSpeedChoice,
}

pub enum SpeechSpeed {
    VeryVerySlow,
    VerySlow,
    Slow,
    Normal,
    Quick,
    VeryQuick,
    VeryVeryQuick,
}

#[derive(Debug)]
pub enum TtsError {
    Unauthorized(String),
    Unknown(String),
    NoContent(String),
    ConnectionFailure(String),
    WriteToFileFailure(String),
}

impl Error for TtsError {}
impl Display for TtsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtsError::Unauthorized(error_str) => write!(f, "Unauthorized: {}", error_str),
            TtsError::Unknown(error_str) => write!(f, "Unknown: {}", error_str),
            TtsError::NoContent(error_str) => write!(f, "NoContent: {}", error_str),
            TtsError::ConnectionFailure(error_str) => write!(f, "ConnectionFailure: {}", error_str),
            TtsError::WriteToFileFailure(error_str) => {
                write!(f, "WriteToFileFailure: {}", error_str)
            }
        }
    }
}
