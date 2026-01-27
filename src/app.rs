use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use futures::StreamExt;

#[component]
pub fn App() -> impl IntoView {
    // ... existing signals ...

    let send_message = move |_| {
        let text = input.get();
        let model = selected_model.get().unwrap_or_else(|| "llama3".to_string());

        if text.is_empty() { return; }

        // Push User Message
        set_messages.update(|msgs| msgs.push(ChatMessage { role: "user".into(), text: text.clone() }));
        // Push Placeholder AI Message
        set_messages.update(|msgs| msgs.push(ChatMessage { role: "ai".into(), text: "".into() }));
        set_input.set("".into());

        spawn_local(async move {
            let client = reqwest::Client::new();
            let res = client.post("/api/stream") // Ensure this route is registered in main.rs
            .json(&serde_json::json!({ "model": model, "prompt": text }))
            .send()
            .await;

            if let Ok(response) = res {
                let mut stream = response.bytes_stream();
                while let Some(Ok(chunk)) = stream.next().await {
                    let raw_chunk = String::from_utf8_lossy(&chunk);

                    // Parse SSE format: "data: <content>\n\n"
                    for line in raw_chunk.lines() {
                        if let Some(content) = line.strip_prefix("data: ") {
                            if content == "__END__" { break; }

                            set_messages.update(|msgs| {
                                if let Some(last) = msgs.last_mut() {
                                    last.text.push_str(content);
                                }
                            });
                        }
                    }
                }
            }
        });
    };

    // ... view! logic ...
}
