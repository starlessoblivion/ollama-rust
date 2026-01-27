#[cfg(feature = "ssr")]
async fn stream_handler(
    axum::extract::State(_state): axum::extract::State<leptos::prelude::LeptosOptions>,
                        axum::Json(payload): axum::Json<PromptRequest>,
) -> axum::response::sse::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    use futures::StreamExt;
    use tokio_util::codec::{FramedRead, LinesCodec};
    use tokio_util::io::StreamReader;

    let client = reqwest::Client::new();
    let res = client
    .post("http://localhost:11434/api/generate")
    .json(&serde_json::json!({
        "model": payload.model,
        "prompt": payload.prompt,
        "stream": true
    }))
    .send()
    .await;

    match res {
        Ok(response) => {
            let body_with_io_error = response.bytes_stream().map(|res| {
                res.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            });
            let reader = StreamReader::new(body_with_io_error);
            let mut lines = FramedRead::new(reader, LinesCodec::new());

            let stream = async_stream::stream! {
                while let Some(Ok(line)) = lines.next().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        if let Some(text) = json["response"].as_str() {
                            yield Ok(axum::response::sse::Event::default().data(text));
                        }
                        if json["done"].as_bool().unwrap_or(false) {
                            yield Ok(axum::response::sse::Event::default().data("__END__"));
                        }
                    }
                }
            };
            axum::response::sse::Sse::new(stream)
        }
        Err(_) => {
            let error_stream = futures::stream::once(async {
                Ok(axum::response::sse::Event::default().data("[Error: Ollama not reachable]"))
            });
            axum::response::sse::Sse::new(error_stream)
        }
    }
}
