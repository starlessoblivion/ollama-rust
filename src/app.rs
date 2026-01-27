use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StatusResponse {
    pub running: bool,
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub role: String, // "user" or "ai"
    pub text: String,
}

// --- SERVER FUNCTIONS ---

#[server]
pub async fn get_ollama_status() -> Result<StatusResponse, ServerFnError> {
    let running = Command::new("pgrep").arg("-x").arg("ollama").output()
    .map(|o| o.status.success()).unwrap_or(false);

    let mut models = Vec::new();
    if let Ok(out) = Command::new("ollama").arg("list").output() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        models = stdout.lines().skip(1)
        .filter_map(|line| line.split_whitespace().next())
        .map(|s| s.to_string()).collect();
    }
    Ok(StatusResponse { running, models })
}

#[server]
pub async fn toggle_ollama_service() -> Result<StatusResponse, ServerFnError> {
    let is_running = Command::new("pgrep").arg("-x").arg("ollama").output()
    .map(|o| o.status.success()).unwrap_or(false);

    if is_running {
        let _ = Command::new("pkill").arg("-x").arg("ollama").output();
    } else {
        let _ = Command::new("ollama").arg("serve").spawn();
    }
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    get_ollama_status().await
}

// --- UI COMPONENTS ---

#[component]
pub fn App() -> impl IntoView {
    // 1. STATE: Core logic signals
    let (selected_model, set_selected_model) = signal(None::<String>);
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input, set_input) = signal(String::new());

    // 2. DATA RESOURCES
    let status_resource = Resource::new(|| (), |_| get_ollama_status());
    let toggle_action = Action::new(|_: &()| toggle_ollama_service());

    // 3. HANDLERS
    let send_message = move |_| {
        let text = input.get();
        if !text.is_empty() {
            set_messages.update(|msgs| msgs.push(ChatMessage {
                role: "user".to_string(),
                                                 text: text.clone(),
            }));
            set_input.set("".to_string());

            // Note: Streaming logic for AI response would go here
            // using the /stream-run route defined in main.rs
        }
    };

    view! {
        <div class="chat-container">
        <header class="chat-header">
        <div class="header-left">
        <button class="model-button">
        "🧠 " {move || selected_model.get().unwrap_or_else(|| "Select Model".to_string())}
        </button>
        </div>
        <div class="chat-title">"Ollama Rust"</div>
        <div class="header-right">
        <label class="toggle-switch">
        <input type="checkbox"
        checked=move || status_resource.get().map(|s| s.map(|r| r.running).unwrap_or(false)).unwrap_or(false)
        on:change=move |_| toggle_action.dispatch(())
        />
        <span class="slider"></span>
        </label>
        </div>
        </header>

        <div class="chat-window">
        // Dynamically render messages from the signal
        <For
        each=move || messages.get()
        key=|msg| msg.text.clone() // Simple key for demo
        children=move |msg| {
            let class = if msg.role == "user" { "chat-bubble user-bubble" } else { "chat-bubble ai-bubble" };
            view! { <div class=class>{msg.text}</div> }
        }
        />
        </div>

        <div class="chat-input-area">
        <textarea
        placeholder="Type your message..."
        prop:value=input
        on:input=move |ev| set_input.set(event_target_value(&ev))
        />
        <button class="send-button" on:click=send_message>"➤"</button>
        </div>
        </div>
    }
}
