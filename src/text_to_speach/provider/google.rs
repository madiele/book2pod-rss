use tokio::{self, task};
use tts_rust::{languages::Languages, tts::GTTSClient};

async fn run(text: String, filename: String) {
    let _ = task::spawn_blocking(move || {
        let narrator: GTTSClient = GTTSClient {
            volume: 1.0,
            language: Languages::English, // use the Languages enum
            tld: "com",
        };
        let _ = narrator.save_to_file(text.as_str(), filename.as_str());
    })
    .await;
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn google_test() {
        run("ciao ciao".to_owned(), "test_gtts.mp3".to_owned()).await
    }
}

