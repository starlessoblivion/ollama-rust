use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use futures::StreamExt;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub text: String,
}

#[server]
pub async fn get_ollama_status() -> Result<StatusResponse, ServerFnError> {
    // Basic check for Ollama local API
    let client = reqwest::Client::new();
    let res = client.get("http://localhost:11434/api/tags").send().await;

    match res {
        Ok(_) => Ok(StatusResponse { status: "Online".to_string() }),
        Err(_) => Ok(StatusResponse { status: "Offline".to_string() }),
    }
}

#[server]
pub async fn toggle_ollama_service() -> Result<StatusResponse, ServerFnError> {
    // Logic to start/stop service would go here
    Ok(StatusResponse { status: "Toggled".to_string() })
}

#[component]
pub fn App() -> impl IntoView {
    let (input, set_input) = signal(String::new());
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (selected_model, _set_selected_model) = signal(Some("llama3".to_string()));

    let status_resource = Resource::new(|| (), |_| get_ollama_status());
    let toggle_action = Action::new(|_| toggle_ollama_service());

    let send_message = move |_| {
        let text = input.get();
        let model = selected_model.get().unwrap_or_else(|| "llama3".to_string());

        if text.is_empty() { return; }

        set_messages.update(|msgs| msgs.push(ChatMessage { role: "user".into(), text: text.clone() }));
        set_messages.update(|msgs| msgs.push(ChatMessage { role: "ai".into(), text: "".into() }));
        set_input.set("".into());

        spawn_local(async move {
            let client = reqwest::Client::new();
            let res = client.post("/api/stream")
            .json(&serde_json::json!({ "model": model, "prompt": text }))
            .send()
            .await;

            if let Ok(response) = res {
                let mut stream = response.bytes_stream();
                while let Some(Ok(chunk)) = stream.next().await {
                    let raw_chunk = String::from_utf8_lossy(&chunk);
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

    view! {
        <div class="chat-container">
        <header class="chat-header">
        <span>"Ollama Status: " {move || status_resource.get().map(|r| r.map(|s| s.status).unwrap_or_default())}</span>
        <button on:click=move |_| { toggle_action.dispatch(()); }>
        "Toggle Service"
        </button>
        </header>
        <textarea
        placeholder="Type your message..."
        prop:value=move || input.get()
        on:input=move |ev| set_input.set(event_target_value(&ev))
        ></textarea>
        <button on:click=send_message>"Send"</button>
        </div>
    }
}
