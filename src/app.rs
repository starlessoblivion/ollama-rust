use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatusResponse {
    pub running: bool,
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub text: String,
}

#[server]
pub async fn get_ollama_status() -> Result<StatusResponse, ServerFnError> {
    let client = reqwest::Client::new();

    // Check if Ollama is running by hitting the tags endpoint
    let res = client.get("http://localhost:11434/api/tags").send().await;

    match res {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                let models: Vec<String> = json["models"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(StatusResponse { running: true, models })
            } else {
                Ok(StatusResponse { running: true, models: vec![] })
            }
        }
        Err(_) => Ok(StatusResponse { running: false, models: vec![] }),
    }
}

#[server]
pub async fn toggle_ollama_service() -> Result<StatusResponse, ServerFnError> {
    // This is a placeholder - actual toggle would require system commands
    // For now, just return current status
    get_ollama_status().await
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"/>
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

    // State
    let (input, set_input) = signal(String::new());
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (selected_model, set_selected_model) = signal::<Option<String>>(None);
    let (is_streaming, set_is_streaming) = signal(false);
    let (menu_open, set_menu_open) = signal(false);
    let (models_panel_open, set_models_panel_open) = signal(false);

    // Resources
    let status_resource = Resource::new(|| (), |_| get_ollama_status());

    // Auto-select first model when status loads
    Effect::new(move |_| {
        if let Some(Ok(status)) = status_resource.get() {
            if selected_model.get().is_none() && !status.models.is_empty() {
                set_selected_model.set(Some(status.models[0].clone()));
            }
        }
    });

    // Send message handler
    let send_message = move |_| {
        let text = input.get();
        if text.trim().is_empty() || selected_model.get().is_none() || is_streaming.get() {
            return;
        }

        // Add user message
        set_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: "user".to_string(),
                text: text.clone(),
            });
        });

        // Add placeholder AI message
        set_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: "ai".to_string(),
                text: "".to_string(),
            });
        });

        set_input.set(String::new());
        set_is_streaming.set(true);

        // Start streaming
        let model = selected_model.get().unwrap();
        let prompt = text.clone();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;
            use wasm_bindgen::JsCast;
            use web_sys::{EventSource, MessageEvent};

            // Use fetch with SSE
            wasm_bindgen_futures::spawn_local(async move {
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();

                let opts = web_sys::RequestInit::new();
                opts.set_method("POST");
                opts.set_body(&JsValue::from_str(&serde_json::json!({
                    "model": model,
                    "prompt": prompt
                }).to_string()));

                let headers = web_sys::Headers::new().unwrap();
                headers.set("Content-Type", "application/json").unwrap();
                opts.set_headers(&headers);

                let request = web_sys::Request::new_with_str_and_init("/api/stream", &opts).unwrap();

                let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await;

                if let Ok(resp) = resp_value {
                    let resp: web_sys::Response = resp.dyn_into().unwrap();
                    let body = resp.body().unwrap();
                    let reader = body.get_reader();

                    let mut full_text = String::new();

                    loop {
                        let result = wasm_bindgen_futures::JsFuture::from(reader.read()).await;
                        if let Ok(chunk) = result {
                            let chunk: js_sys::Object = chunk.dyn_into().unwrap();
                            let done = js_sys::Reflect::get(&chunk, &JsValue::from_str("done")).unwrap();

                            if done.as_bool().unwrap_or(true) {
                                break;
                            }

                            let value = js_sys::Reflect::get(&chunk, &JsValue::from_str("value")).unwrap();
                            let array: js_sys::Uint8Array = value.dyn_into().unwrap();
                            let bytes = array.to_vec();
                            let text = String::from_utf8_lossy(&bytes);

                            // Parse SSE format
                            for line in text.lines() {
                                if line.starts_with("data:") {
                                    let data = line.trim_start_matches("data:");
                                    if data == "__END__" {
                                        set_is_streaming.set(false);
                                        break;
                                    }
                                    full_text.push_str(data);

                                    let current_text = full_text.clone();
                                    set_messages.update(|msgs| {
                                        if let Some(last) = msgs.last_mut() {
                                            if last.role == "ai" {
                                                last.text = current_text;
                                            }
                                        }
                                    });
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
                set_is_streaming.set(false);
            });
        }
    };

    // Toggle menu
    let toggle_menu = move |_| {
        set_menu_open.update(|v| *v = !*v);
        if !menu_open.get() {
            set_models_panel_open.set(false);
        }
    };

    // Select model
    let select_model = move |model: String| {
        set_selected_model.set(Some(model));
        set_menu_open.set(false);
        set_models_panel_open.set(false);
    };

    view! {
        <Stylesheet id="leptos" href="/pkg/ollama-rust.css"/>
        <Title text="Ollama Rust"/>

        <div class="chat-container">
            // Header
            <div class="chat-header">
                <div class="header-left">
                    <div class="model-dropdown">
                        <button id="model-button" type="button" on:click=toggle_menu>
                            {move || {
                                if let Some(model) = selected_model.get() {
                                    format!("🧠 ollama: {}", model)
                                } else {
                                    "🧠 Model".to_string()
                                }
                            }}
                        </button>

                        <div id="model-menu"
                             class="model-menu"
                             class:hidden=move || !menu_open.get()>
                            <div class="runner-list">
                                <div class="runner-item"
                                     on:mouseenter=move |_| set_models_panel_open.set(true)
                                     on:click=move |_| set_models_panel_open.update(|v| *v = !*v)>
                                    <div class="runner-name">"ollama"</div>

                                    <div id="models-panel"
                                         class="models-panel"
                                         class:hidden=move || !models_panel_open.get()>
                                        <Suspense fallback=move || view! { <div>"Loading..."</div> }>
                                            {move || {
                                                status_resource.get().map(|result| {
                                                    match result {
                                                        Ok(status) => {
                                                            view! {
                                                                <div id="ollama-models" class="model-submenu">
                                                                    {status.models.into_iter().map(|model| {
                                                                        let m = model.clone();
                                                                        let m2 = model.clone();
                                                                        view! {
                                                                            <div class="model-option"
                                                                                 on:click=move |_| select_model(m.clone())>
                                                                                {m2}
                                                                            </div>
                                                                        }
                                                                    }).collect_view()}
                                                                </div>
                                                            }.into_any()
                                                        }
                                                        Err(_) => view! { <div>"Error loading models"</div> }.into_any()
                                                    }
                                                })
                                            }}
                                        </Suspense>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="chat-title">"🧠"</div>

                <div class="header-right">
                    <label class="toggle-switch" title="Ollama Status">
                        <Suspense fallback=move || view! { <input type="checkbox" disabled=true /> }>
                            {move || {
                                status_resource.get().map(|result| {
                                    let running = result.map(|s| s.running).unwrap_or(false);
                                    view! {
                                        <input type="checkbox"
                                               id="ollama-toggle"
                                               prop:checked=running
                                               disabled=true />
                                        <span class="slider"></span>
                                    }
                                })
                            }}
                        </Suspense>
                    </label>
                </div>
            </div>

            // Chat window
            <div id="chat-window" class="chat-window">
                <For
                    each=move || messages.get()
                    key=|msg| format!("{}-{}", msg.role, msg.text.len())
                    children=move |msg| {
                        let is_user = msg.role == "user";
                        let is_empty_ai = msg.role == "ai" && msg.text.is_empty();

                        view! {
                            <div class="chat-bubble"
                                 class:user-bubble=is_user
                                 class:ai-bubble=!is_user>
                                {if is_empty_ai {
                                    view! {
                                        <span class="thinking">
                                            <span class="brain">"🧠"</span>
                                            <span class="thinking-dots">
                                                <span class="thinking-dot"></span>
                                                <span class="thinking-dot"></span>
                                                <span class="thinking-dot"></span>
                                            </span>
                                        </span>
                                    }.into_any()
                                } else {
                                    view! { <span>{msg.text.clone()}</span> }.into_any()
                                }}
                            </div>
                        }
                    }
                />
            </div>

            // Input area
            <div class="chat-input-area">
                <textarea
                    id="prompt-input"
                    placeholder="Type your message..."
                    rows="1"
                    prop:value=move || input.get()
                    on:input=move |ev| set_input.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" && !ev.shift_key() && !ev.alt_key() {
                            ev.prevent_default();
                            send_message(());
                        }
                    }
                    disabled=move || is_streaming.get()
                ></textarea>
                <button id="send-button"
                        type="button"
                        on:click=send_message
                        disabled=move || is_streaming.get()>
                    "➤"
                </button>
            </div>
        </div>
    }
}
