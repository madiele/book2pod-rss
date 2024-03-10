mod google;
mod openai;

#[trait_variant::make(HttpService: Send)]
pub trait TtsClient {
    async fn speak_to_file(self, text: String, path: String);
}

pub trait TtsClientBuilder {
    fn capabilities() -> &'static [TtsCapabilites];
    fn default() -> impl TtsClientBuilder;
    fn authorize(self) -> impl TtsClientBuilder;
    fn with_voice(self, voice: String) -> impl TtsClientBuilder;
    fn for_language(self, language: &locale_codes::language::LanguageInfo)
        -> impl TtsClientBuilder;
    fn build(self) -> impl TtsClient;
}

pub enum TtsCapabilites {
    LanguageChoice,
    VoiceChoice,
    RequiresAuth,
}

