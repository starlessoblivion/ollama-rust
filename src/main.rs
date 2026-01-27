#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use ax_ollama::app::*; // Replace ax_ollama with your actual crate name if different
    use ax_ollama::fileserv::file_and_error_handler;
    use axum::routing::post;
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);

    // Build our application with routes
    let app = Router::new()
    // REGISTER THE STREAMING ROUTE HERE
    .route("/api/stream", post(stream_handler))
    .leptos_routes(&leptos_options, routes, {
        let leptos_options = leptos_options.clone();
        move || shell(leptos_options.clone())
    })
    .fallback(file_and_error_handler)
    .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app).await.unwrap();
}

// Ensure this struct matches what your frontend sends
#[cfg(feature = "ssr")]
#[derive(serde::Deserialize)]
pub struct PromptRequest {
    pub model: String,
    pub prompt: String,
}

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
                        // Check if Ollama is finished
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

// Fallback for non-SSR compilation
#[cfg(not(feature = "ssr"))]
pub fn main() {
    // No-op for client-side
}
