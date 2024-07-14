use std::{error::Error, fmt::Display};

mod google;
mod openai;

enum TtsClientDispatcher {
    OpenAi(openai::OpenAiTtsClient),
    Google(tts_rust::tts::GTTSClient),
}

enum TtsProvider {
    OpenAi,
    Google,
}

impl TtsProvider {
    fn default(self) -> TtsClientDispatcher {
        match self {
            TtsProvider::OpenAi => TtsClientDispatcher::OpenAi(create_tts_client(
                openai::OpenAiTtsClientBuilder::default(),
            )),
            TtsProvider::Google => {
                TtsClientDispatcher::Google(create_tts_client(google::GTTSClientBuilder::default()))
            } // Default to Google if environment variable is not set
        }
    }
}

impl TtsClient for TtsClientDispatcher {
    fn speak_to_file(self, text: String, path: String) -> Result<(), TtsError> {
        match self {
            TtsClientDispatcher::OpenAi(c) => c.speak_to_file(text, path),
            TtsClientDispatcher::Google(c) => c.speak_to_file(text, path),
        }
    }
}

fn create_tts_client<Builder, Client>(builder: Builder) -> Client
where
    Builder: TtsClientBuilder<Client>,
    Client: TtsClient,
{
    builder
        .authorize()
        .with_voice("alloy".to_owned())
        .set_speed(SpeechSpeed::Normal)
        .build()
}

pub trait TtsClient {
    fn speak_to_file(self, text: String, path: String) -> Result<(), TtsError>;
}

pub trait TtsClientBuilder<Client>
where
    Client: TtsClient,
{
    fn capabilities() -> &'static [TtsCapabilites];
    fn default() -> Self;
    fn authorize(self) -> Self;
    fn with_voice(self, voice: String) -> Self;
    fn set_speed(self, speed: SpeechSpeed) -> Self;
    fn for_language(self, language: &locale_codes::language::LanguageInfo) -> Self;
    fn build(self) -> Client;
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

#[cfg(test)]
mod test {
    use super::{TtsClient, TtsProvider};

    #[test]
    fn google() {
        let test = TtsProvider::Google.default();
        _ = test.speak_to_file("".to_owned(), "".to_owned());
    }

    #[test]
    fn openai() {
        let test = TtsProvider::OpenAi.default();
        _ = test.speak_to_file("".to_owned(), "".to_owned());
    }
}
