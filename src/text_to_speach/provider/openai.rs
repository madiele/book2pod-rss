use reqwest::header;
use tokio;

async fn run() -> Result<(), anyhow::Error> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("Please set the OPENAI_API_KEY environment variable");
    let url = "https://api.openai.com/v1/audio/speech";

    let client = reqwest::Client::new();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", api_key))?,
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
            "input": "Today is a wonderful day to build something people love!",
            "voice": "alloy"
        }))
        .send()
        .await?;

    if response.status().is_success() {
        let bytes = response.bytes().await?;
        tokio::fs::write("speech.mp3", bytes).await?;
        println!("Speech saved to speech.mp3");
    } else {
        let error_msg = response.text().await?;
        println!("Failed to retrieve speech: {}", error_msg);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn name() {
        run().await;
    }
}
