use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use serde::{Deserialize, Serialize};

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
    let client = reqwest::Client::new();
    let res = client.get("http://localhost:11434/api/tags").send().await;

    match res {
        Ok(_) => Ok(StatusResponse { status: "Online".to_string() }),
        Err(_) => Ok(StatusResponse { status: "Offline".to_string() }),
    }
}

#[server]
pub async fn toggle_ollama_service() -> Result<StatusResponse, ServerFnError> {
    Ok(StatusResponse { status: "Toggled".to_string() })
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let (input, set_input) = signal(String::new());
    let (messages, _set_messages) = signal(Vec::<ChatMessage>::new());
    let (selected_model, _set_selected_model) = signal(Some("llama3".to_string()));

    let status_resource = Resource::new(|| (), |_| get_ollama_status());
    let toggle_action = Action::new(|_| toggle_ollama_service());

    let send_message = move |_| {
        let text = input.get();
        if text.is_empty() { return; }
        set_input.set("".into());
    };

    view! {
        <Stylesheet id="leptos" href="/pkg/ollama-rust.css"/>
        <Title text="Ollama Rust"/>
        <div class="chat-container">
        <header class="chat-header">
        <span>
        "Ollama Status: "
        {move || status_resource.get().map(|r| r.map(|s| s.status).unwrap_or_else(|_| "Error".into()))}
        </span>
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
