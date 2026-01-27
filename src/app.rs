use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use futures::StreamExt;

// ... (Keep your existing StatusResponse and ChatMessage structs) ...

#[component]
pub fn App() -> impl IntoView {
    let (selected_model, _) = signal(Some("llama3".to_string())); // Default for testing
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input, set_input) = signal(String::new());

    // Action to handle the streaming request
    let send_message = move |_| {
        let text = input.get();
        let model = selected_model.get().unwrap_or_default();

        if text.is_empty() || model.is_empty() { return; }

        // 1. Add User Message
        set_messages.update(|msgs| msgs.push(ChatMessage {
            role: "user".to_string(),
                                             text: text.clone(),
        }));

        // 2. Clear input
        set_input.set("".to_string());

        // 3. Prepare AI placeholder message
        set_messages.update(|msgs| msgs.push(ChatMessage {
            role: "ai".to_string(),
                                             text: "".to_string(),
        }));

        // 4. Start Streaming (Client-side WASM)
        spawn_local(async move {
            let client = reqwest::Client::new();
            let res = client.post("/api/stream")
            .json(&serde_json::json!({ "model": model, "prompt": text }))
            .send()
            .await;

            if let Ok(response) = res {
                let mut stream = response.bytes_stream();
                while let Some(Ok(chunk)) = stream.next().await {
                    let chunk_text = String::from_utf8_lossy(&chunk).to_string();

                    // Update the last message (the AI's) with new chunks
                    set_messages.update(|msgs| {
                        if let Some(last) = msgs.last_mut() {
                            last.text.push_str(&chunk_text);
                        }
                    });
                }
            }
        });
    };

    // ... (rest of your view remains mostly the same) ...
}
