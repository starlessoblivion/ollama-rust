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
pub async fn get_hostname() -> Result<String, ServerFnError> {
    // Try to get hostname from system
    if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
        let hostname = hostname.trim().to_string();
        if !hostname.is_empty() {
            return Ok(hostname);
        }
    }

    // Fallback: try HOSTNAME env var
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        if !hostname.is_empty() {
            return Ok(hostname);
        }
    }

    // Fallback: try running hostname command
    if let Ok(output) = std::process::Command::new("hostname").output() {
        if output.status.success() {
            let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !hostname.is_empty() {
                return Ok(hostname);
            }
        }
    }

    Ok("ollama".to_string())
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
    use std::process::Command;

    // Check current status
    let current = get_ollama_status().await?;

    if current.running {
        // Stop Ollama - try pkill first, then killall
        let _ = Command::new("pkill")
            .args(["-f", "ollama serve"])
            .output();

        // Give it a moment to stop
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    } else {
        // Start Ollama serve in background
        let _ = Command::new("ollama")
            .arg("serve")
            .spawn();

        // Give it a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    // Return new status
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
    let (ollama_running, set_ollama_running) = signal(false);
    let (toggle_pending, set_toggle_pending) = signal(false);

    // Resources
    let status_resource = Resource::new(|| (), |_| get_ollama_status());
    let hostname_resource = Resource::new(|| (), |_| get_hostname());

    // Toggle action
    let toggle_action = Action::new(move |_: &()| async move {
        toggle_ollama_service().await
    });

    // Update running state when status loads
    Effect::new(move |_| {
        if let Some(Ok(status)) = status_resource.get() {
            set_ollama_running.set(status.running);
        }
    });

    // Update running state when toggle completes
    Effect::new(move |_| {
        if let Some(Ok(status)) = toggle_action.value().get() {
            set_ollama_running.set(status.running);
            set_toggle_pending.set(false);
            // Refetch models after toggle
            status_resource.refetch();
        }
    });

    // Auto-select first model when status loads
    Effect::new(move |_| {
        if let Some(Ok(status)) = status_resource.get() {
            if selected_model.get().is_none() && !status.models.is_empty() {
                set_selected_model.set(Some(status.models[0].clone()));
            }
        }
    });

    // Send message handler
    let do_send = move || {
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

            // Use fetch with SSE
            wasm_bindgen_futures::spawn_local(async move {
                let window = web_sys::window().unwrap();

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
                    if let Some(body) = resp.body() {
                        let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();

                        let mut full_text = String::new();

                        loop {
                            let read_promise = reader.read();
                            let result = wasm_bindgen_futures::JsFuture::from(read_promise).await;
                            if let Ok(chunk) = result {
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
                }
                set_is_streaming.set(false);
            });
        }
    };

    // Close all menus
    let close_menus = move || {
        set_menu_open.set(false);
        set_models_panel_open.set(false);
    };

    // Toggle menu
    let toggle_menu = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        if menu_open.get() {
            close_menus();
        } else {
            set_menu_open.set(true);
        }
    };

    // Select model
    let select_model = move |model: String| {
        set_selected_model.set(Some(model));
        close_menus();
    };

    // Handle runner item interaction (hover/click)
    let open_models_panel = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        set_models_panel_open.set(true);
    };

    view! {
        <Stylesheet id="leptos" href="/pkg/ollama-rust.css"/>
        <Title text="Ollama Rust"/>

        // Backdrop to close menus when clicking outside
        <div class="menu-backdrop"
             class:hidden=move || !menu_open.get()
             on:click=move |_| close_menus()
             on:touchend=move |_| close_menus()>
        </div>

        <div class="chat-container">
            // Header
            <div class="chat-header">
                <div class="header-left">
                    <div class="model-dropdown">
                        <button id="model-button" type="button" on:click=toggle_menu>
                            {move || {
                                if let Some(model) = selected_model.get() {
                                    // Truncate long model names
                                    let display = if model.len() > 15 {
                                        format!("{}...", &model[..12])
                                    } else {
                                        model
                                    };
                                    format!("🧠 {}", display)
                                } else {
                                    "🧠 Model".to_string()
                                }
                            }}
                        </button>

                        <div id="model-menu"
                             class="model-menu"
                             class:hidden=move || !menu_open.get()
                             on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()>
                            <div class="runner-list">
                                <div class="runner-item"
                                     on:mouseenter=open_models_panel
                                     on:click=open_models_panel
                                     on:touchstart=move |ev: web_sys::TouchEvent| {
                                         ev.stop_propagation();
                                         set_models_panel_open.set(true);
                                     }>
                                    <div class="runner-name">"ollama"</div>

                                    <div id="models-panel"
                                         class="models-panel"
                                         class:hidden=move || !models_panel_open.get()
                                         on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()>
                                        <Suspense fallback=move || view! { <div>"Loading..."</div> }>
                                            {move || {
                                                status_resource.get().map(|result| {
                                                    match result {
                                                        Ok(status) => {
                                                            view! {
                                                                <div id="ollama-models" class="model-submenu">
                                                                    {status.models.into_iter().map(|model| {
                                                                        let m_click = model.clone();
                                                                        let m_touch = model.clone();
                                                                        let m_display = model.clone();
                                                                        view! {
                                                                            <div class="model-option"
                                                                                 on:click=move |ev: web_sys::MouseEvent| {
                                                                                     ev.stop_propagation();
                                                                                     select_model(m_click.clone());
                                                                                 }
                                                                                 on:touchend=move |ev: web_sys::TouchEvent| {
                                                                                     ev.stop_propagation();
                                                                                     select_model(m_touch.clone());
                                                                                 }>
                                                                                {m_display}
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

                <div class="chat-title">
                    <Suspense fallback=move || view! { "..." }>
                        {move || {
                            hostname_resource.get().map(|result| {
                                result.unwrap_or_else(|_| "ollama".to_string())
                            })
                        }}
                    </Suspense>
                </div>

                <div class="header-right">
                    <label class="toggle-switch" title="Toggle Ollama serve">
                        <input type="checkbox"
                               id="ollama-toggle"
                               prop:checked=move || ollama_running.get()
                               prop:disabled=move || toggle_pending.get()
                               on:change=move |_| {
                                   set_toggle_pending.set(true);
                                   toggle_action.dispatch(());
                               } />
                        <span class="slider"></span>
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
                        let msg_text = msg.text.clone();

                        view! {
                            <div class="chat-bubble"
                                 class:user-bubble=is_user
                                 class:ai-bubble=!is_user>
                                {if is_empty_ai {
                                    // Thinking animation
                                    view! {
                                        <span class="thinking">
                                            <span class="msg-prefix">
                                                <Suspense fallback=move || view! { "[...]" }>
                                                    {move || hostname_resource.get().map(|h| {
                                                        format!("[{}]", h.unwrap_or_else(|_| "ollama".to_string()))
                                                    })}
                                                </Suspense>
                                            </span>
                                            <span class="thinking-dots">
                                                <span class="thinking-dot"></span>
                                                <span class="thinking-dot"></span>
                                                <span class="thinking-dot"></span>
                                            </span>
                                        </span>
                                    }.into_any()
                                } else if is_user {
                                    // User message - just show text
                                    view! { <span>{msg_text}</span> }.into_any()
                                } else {
                                    // AI message with hostname prefix
                                    view! {
                                        <span>
                                            <span class="msg-prefix">
                                                <Suspense fallback=move || view! { "[...]:" }>
                                                    {move || hostname_resource.get().map(|h| {
                                                        format!("[{}]: ", h.unwrap_or_else(|_| "ollama".to_string()))
                                                    })}
                                                </Suspense>
                                            </span>
                                            {msg_text.clone()}
                                        </span>
                                    }.into_any()
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
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" && !ev.shift_key() && !ev.alt_key() {
                            ev.prevent_default();
                            do_send();
                        }
                    }
                    disabled=move || is_streaming.get()
                ></textarea>
                <button id="send-button"
                        type="button"
                        on:click=move |_: web_sys::MouseEvent| do_send()
                        disabled=move || is_streaming.get()>
                    "➤"
                </button>
            </div>
        </div>
    }
}
