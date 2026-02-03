use leptos::prelude::*;
use leptos::task::spawn_local;
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PullProgress {
    pub model: String,
    pub status: String,
    pub percent: f32,
    pub done: bool,
    pub error: Option<String>,
    pub bytes_downloaded: u64,
    pub speed: String,
    pub last_update: i64, // timestamp for speed calculation
}

// Global state for tracking pull progress (simple approach using lazy_static would be better but this works)
use std::sync::OnceLock;
use std::collections::HashMap;
use std::sync::Mutex;

static PULL_PROGRESS: OnceLock<Mutex<HashMap<String, PullProgress>>> = OnceLock::new();

fn get_progress_store() -> &'static Mutex<HashMap<String, PullProgress>> {
    PULL_PROGRESS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[server]
pub async fn start_model_pull(model_name: String) -> Result<PullProgress, ServerFnError> {
    use std::process::Command;

    if model_name.trim().is_empty() {
        return Ok(PullProgress {
            model: model_name,
            status: "Error".to_string(),
            percent: 0.0,
            done: true,
            error: Some("Model name cannot be empty".to_string()),
            bytes_downloaded: 0,
            speed: "".to_string(),
            last_update: 0,
        });
    }

    // First ensure Ollama is running
    let status = get_ollama_status().await?;
    if !status.running {
        let _ = Command::new("ollama").arg("serve").spawn();
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    let model = model_name.trim().to_string();
    let model_clone = model.clone();

    // Initialize progress
    {
        let store = get_progress_store();
        let mut map = store.lock().unwrap();
        map.insert(model.clone(), PullProgress {
            model: model.clone(),
            status: "Starting...".to_string(),
            percent: 0.0,
            done: false,
            error: None,
            bytes_downloaded: 0,
            speed: "".to_string(),
            last_update: 0,
        });
    }

    // Start the pull using Ollama API (streams JSON progress)
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let res = client.post("http://localhost:11434/api/pull")
            .json(&serde_json::json!({ "name": model_clone }))
            .send()
            .await;

        match res {
            Ok(response) => {
                use futures::StreamExt;
                let mut stream = response.bytes_stream();

                while let Some(chunk) = stream.next().await {
                    if let Ok(bytes) = chunk {
                        let text = String::from_utf8_lossy(&bytes);
                        // Parse each line as JSON
                        for line in text.lines() {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                let store = get_progress_store();
                                let mut map = store.lock().unwrap();

                                let status_text = json["status"].as_str().unwrap_or("").to_string();
                                let total = json["total"].as_u64().unwrap_or(0);
                                let completed = json["completed"].as_u64().unwrap_or(0);

                                let percent = if total > 0 {
                                    (completed as f32 / total as f32) * 100.0
                                } else {
                                    0.0
                                };

                                // Calculate speed from completed bytes
                                let speed = if total > 0 && completed > 0 && completed < total {
                                    format_bytes(completed) + " / " + &format_bytes(total)
                                } else {
                                    "".to_string()
                                };

                                let is_done = status_text == "success" || json.get("error").is_some();
                                let error = json["error"].as_str().map(|s| s.to_string());

                                map.insert(model_clone.clone(), PullProgress {
                                    model: model_clone.clone(),
                                    status: if is_done && error.is_none() { "Complete".to_string() } else { status_text },
                                    percent: if is_done && error.is_none() { 100.0 } else { percent },
                                    done: is_done,
                                    error,
                                    bytes_downloaded: completed,
                                    speed,
                                    last_update: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64,
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let store = get_progress_store();
                let mut map = store.lock().unwrap();
                map.insert(model_clone.clone(), PullProgress {
                    model: model_clone,
                    status: "Error".to_string(),
                    percent: 0.0,
                    done: true,
                    error: Some(e.to_string()),
                    bytes_downloaded: 0,
                    speed: "".to_string(),
                    last_update: 0,
                });
            }
        }
    });

    Ok(PullProgress {
        model: model_name.trim().to_string(),
        status: "Starting...".to_string(),
        percent: 0.0,
        done: false,
        error: None,
        bytes_downloaded: 0,
        speed: "".to_string(),
        last_update: 0,
    })
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[server]
pub async fn cancel_model_pull(model_name: String) -> Result<bool, ServerFnError> {
    use std::process::Command;

    let model = model_name.trim().to_string();

    // Mark as cancelled in progress store
    {
        let store = get_progress_store();
        let mut map = store.lock().unwrap();
        if let Some(progress) = map.get_mut(&model) {
            progress.done = true;
            progress.status = "Cancelled".to_string();
            progress.error = Some("Download cancelled by user".to_string());
        }
    }

    // Kill any running ollama pull process for this model
    let _ = Command::new("pkill")
        .args(["-f", &format!("ollama pull {}", model)])
        .output();

    Ok(true)
}

#[server]
pub async fn check_pull_progress(model_name: String) -> Result<PullProgress, ServerFnError> {
    let model = model_name.trim().to_string();

    // Check progress store first
    {
        let store = get_progress_store();
        let map = store.lock().unwrap();
        if let Some(progress) = map.get(&model) {
            return Ok(progress.clone());
        }
    }

    // Fallback: check if model exists (might have been pulled before tracking)
    let status = get_ollama_status().await?;
    let model_exists = status.models.iter().any(|m| {
        m.starts_with(&model) || m.contains(&model)
    });

    if model_exists {
        Ok(PullProgress {
            model,
            status: "Complete".to_string(),
            percent: 100.0,
            done: true,
            error: None,
            bytes_downloaded: 0,
            speed: "".to_string(),
            last_update: 0,
        })
    } else {
        Ok(PullProgress {
            model,
            status: "Waiting...".to_string(),
            percent: 0.0,
            done: false,
            error: None,
            bytes_downloaded: 0,
            speed: "".to_string(),
            last_update: 0,
        })
    }
}

#[server]
pub async fn delete_model(model_name: String) -> Result<bool, ServerFnError> {
    use std::process::Command;

    if model_name.trim().is_empty() {
        return Ok(false);
    }

    let output = Command::new("ollama")
        .args(["rm", model_name.trim()])
        .output();

    match output {
        Ok(out) => Ok(out.status.success()),
        Err(_) => Ok(false),
    }
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
    let (show_add_model, set_show_add_model) = signal(false);
    let (new_model_name, set_new_model_name) = signal(String::new());
    let (active_downloads, set_active_downloads) = signal::<Vec<PullProgress>>(vec![]);
    let (deleting_model, set_deleting_model) = signal::<Option<String>>(None);
    let (status_dropdown_open, set_status_dropdown_open) = signal(false);

    // Resources
    let status_resource = Resource::new(|| (), |_| get_ollama_status());
    let hostname_resource = Resource::new(|| (), |_| get_hostname());

    // Toggle action
    let toggle_action = Action::new(move |_: &()| async move {
        toggle_ollama_service().await
    });

    // Delete model action
    let do_delete_model = move |model_name: String| {
        if model_name.trim().is_empty() {
            return;
        }

        set_deleting_model.set(Some(model_name.clone()));

        let model = model_name.clone();
        spawn_local(async move {
            if let Ok(success) = delete_model(model.clone()).await {
                if success {
                    // Clear selected model if it was deleted
                    if selected_model.get().as_ref() == Some(&model) {
                        set_selected_model.set(None);
                    }
                    // Refresh models list
                    status_resource.refetch();
                }
            }
            set_deleting_model.set(None);
        });
    };

    // Start download action
    let start_download = move |model_name: String| {
        if model_name.trim().is_empty() {
            return;
        }

        // Check if already downloading
        let downloads = active_downloads.get();
        if downloads.iter().any(|d| d.model == model_name.trim() && !d.done) {
            return;
        }

        // Add to active downloads
        set_active_downloads.update(|downloads| {
            downloads.push(PullProgress {
                model: model_name.trim().to_string(),
                status: "Starting...".to_string(),
                percent: 0.0,
                done: false,
                error: None,
                bytes_downloaded: 0,
                speed: "".to_string(),
                last_update: 0,
            });
        });

        // Start the pull
        let model = model_name.trim().to_string();
        spawn_local(async move {
            let _ = start_model_pull(model).await;
        });

        // Clear input
        set_new_model_name.set(String::new());
        set_show_add_model.set(false);
    };

    // Poll for download progress
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;

        let check_progress = move || {
            let downloads = active_downloads.get();
            let pending: Vec<_> = downloads.iter()
                .filter(|d| !d.done)
                .map(|d| d.model.clone())
                .collect();

            for model in pending {
                let model_clone = model.clone();
                spawn_local(async move {
                    if let Ok(progress) = check_pull_progress(model_clone.clone()).await {
                        let is_complete = progress.done && progress.error.is_none();

                        set_active_downloads.update(|downloads| {
                            if let Some(d) = downloads.iter_mut().find(|d| d.model == model_clone) {
                                // Calculate download speed
                                let now = js_sys::Date::now() as i64;
                                let time_diff = if d.last_update > 0 { (now - d.last_update) / 1000 } else { 0 };
                                let percent_diff = progress.percent - d.percent;
                                
                                // Estimate speed based on percent change (rough estimate)
                                let speed_str = if time_diff > 0 && percent_diff > 0.0 {
                                    // Assume models are roughly 4GB for estimation
                                    let estimated_bytes = (percent_diff / 100.0) * 4_000_000_000.0;
                                    let bytes_per_sec = estimated_bytes / (time_diff as f32);
                                    if bytes_per_sec > 1_000_000_000.0 {
                                        format!("{:.1} GB/s", bytes_per_sec / 1_000_000_000.0)
                                    } else if bytes_per_sec > 1_000_000.0 {
                                        format!("{:.1} MB/s", bytes_per_sec / 1_000_000.0)
                                    } else if bytes_per_sec > 1_000.0 {
                                        format!("{:.1} KB/s", bytes_per_sec / 1_000.0)
                                    } else {
                                        format!("{:.0} B/s", bytes_per_sec)
                                    }
                                } else {
                                    "".to_string()
                                };

                                d.status = progress.status;
                                d.percent = progress.percent;
                                d.done = progress.done;
                                d.error = progress.error;
                                d.speed = speed_str;
                                d.last_update = now;
                            }
                        });

                        // Refresh models list when complete
                        if is_complete {
                            status_resource.refetch();
                        }
                    }
                });
            }
        };

        // Set up interval to check progress
        Effect::new(move |_| {
            let downloads = active_downloads.get();
            if downloads.iter().any(|d| !d.done) {
                let cb = Closure::wrap(Box::new(move || {
                    check_progress();
                }) as Box<dyn Fn()>);

                if let Some(window) = web_sys::window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(),
                        2000, // Check every 2 seconds
                    );
                }
                cb.forget();
            }
        });
    }

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
                                        let data = line.trim_start_matches("data:").trim();
                                        if data == "__END__" || data.is_empty() {
                                            if data == "__END__" {
                                                set_is_streaming.set(false);
                                            }
                                            break;
                                        }
                                        full_text.push_str(data);
                                        full_text.push(' '); // Add space between chunks

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
                                        // Add Model section
                                        <div class="add-model-section">
                                            // Library link
                                            <a href="https://ollama.com/library"
                                               target="_blank"
                                               rel="noopener noreferrer"
                                               class="model-option library-link"
                                               on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()>
                                                "📚 Browse Models"
                                            </a>

                                            {move || if show_add_model.get() {
                                                view! {
                                                    <div class="add-model-input-row">
                                                        <input
                                                            type="text"
                                                            class="add-model-input"
                                                            placeholder="model name (e.g. llama3)"
                                                            prop:value=move || new_model_name.get()
                                                            on:input=move |ev| set_new_model_name.set(event_target_value(&ev))
                                                            on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
                                                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                                ev.stop_propagation();
                                                                if ev.key() == "Enter" {
                                                                    let name = new_model_name.get();
                                                                    start_download(name);
                                                                }
                                                            }
                                                        />
                                                        <button
                                                            class="add-model-btn pull-btn"
                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                ev.stop_propagation();
                                                                let name = new_model_name.get();
                                                                start_download(name);
                                                            }
                                                        >
                                                            "Pull"
                                                        </button>
                                                        <button
                                                            class="add-model-btn cancel-btn"
                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                ev.stop_propagation();
                                                                set_show_add_model.set(false);
                                                                set_new_model_name.set(String::new());
                                                            }
                                                        >
                                                            "✕"
                                                        </button>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="model-option add-model-option"
                                                         on:click=move |ev: web_sys::MouseEvent| {
                                                             ev.stop_propagation();
                                                             set_show_add_model.set(true);
                                                         }>
                                                        "+ Add Model"
                                                    </div>
                                                }.into_any()
                                            }}
                                        </div>

                                        // Divider
                                        <div class="model-divider"></div>

                                        // Models list
                                        <Suspense fallback=move || view! { <div class="loading-models">"Loading..."</div> }>
                                            {move || {
                                                status_resource.get().map(|result| {
                                                    match result {
                                                        Ok(status) => {
                                                            if status.models.is_empty() {
                                                                view! {
                                                                    <div class="no-models">"Turn on Ollama to view installed models"</div>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <div id="ollama-models" class="model-submenu">
                                                                        {status.models.into_iter().map(|model| {
                                                                            let m_click = model.clone();
                                                                            let m_touch = model.clone();
                                                                            let m_display = model.clone();
                                                                            let m_delete = model.clone();
                                                                            let m_delete_for_closure = m_delete.clone();
                                                                            let is_deleting = move || {
                                                                                deleting_model.get().as_ref() == Some(&m_delete_for_closure)
                                                                            };
                                                                            view! {
                                                                                <div class="model-option-row">
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
                                                                                    <button
                                                                                        class="model-delete-btn"
                                                                                        title="Delete model"
                                                                                        disabled=is_deleting()
                                                                                        on:click=move |ev: web_sys::MouseEvent| {
                                                                                            ev.stop_propagation();
                                                                                            do_delete_model(m_delete.clone());
                                                                                        }>
                                                                                        {if is_deleting() { "..." } else { "❌" }}
                                                                                    </button>
                                                                                </div>
                                                                            }
                                                                        }).collect_view()}
                                                                    </div>
                                                                }.into_any()
                                                            }
                                                        }
                                                        Err(_) => view! { <div class="error-models">"Error loading models"</div> }.into_any()
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
                    <div class="status-dropdown">
                        <button class="status-button"
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    set_status_dropdown_open.update(|v| *v = !*v);
                                }>
                            <span class="status-dot"
                                  class:status-green=move || ollama_running.get()
                                  class:status-red=move || !ollama_running.get()
                                  class:status-yellow=move || toggle_pending.get()>
                            </span>
                            "Status"
                        </button>
                        <div class="status-menu"
                             class:hidden=move || !status_dropdown_open.get()
                             on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()>
                            <div class="status-menu-item">
                                <span class="status-label">"Ollama Serve"</span>
                                <label class="toggle-switch">
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
                    </div>
                </div>
            </div>

            // Backdrop for status dropdown
            <div class="menu-backdrop"
                 class:hidden=move || !status_dropdown_open.get()
                 on:click=move |_| set_status_dropdown_open.set(false)
                 on:touchend=move |_| set_status_dropdown_open.set(false)>
            </div>

            // Download progress bars
            <div class="download-progress-container">
                {move || {
                    let downloads: Vec<_> = active_downloads.get()
                        .into_iter()
                        .filter(|d| !d.done || d.error.is_some())
                        .collect();

                    downloads.into_iter().map(|dl| {
                        let model_name = dl.model.clone();
                        let model_for_hide = dl.model.clone();
                        let model_for_cancel = dl.model.clone();
                        let model_for_cancel_update = dl.model.clone();
                        let status = dl.status.clone();
                        let status_for_check = status.clone();
                        let percent = dl.percent;
                        let speed = dl.speed.clone();
                        let is_done = dl.done;

                        let is_complete = status_for_check == "Complete";
                        let is_cancelled = status_for_check == "Cancelled";
                        let can_cancel = !is_done && !is_complete && !is_cancelled;
                        let percent_display = format!("{:.0}%", percent);

                        view! {
                            <div class="download-progress-bar">
                                <div class="download-info">
                                    <span class="download-model">{model_name}</span>
                                    <span class="download-status"
                                          class:download-complete=is_complete>
                                        {status}
                                    </span>
                                    {if !speed.is_empty() {
                                        view! { <span class="download-speed">{speed}</span> }.into_any()
                                    } else if !is_complete && percent > 0.0 {
                                        view! { <span class="download-speed">{percent_display}</span> }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }}
                                    // Cancel button - stops the download
                                    {if can_cancel {
                                        view! {
                                            <button class="download-cancel"
                                                    title="Cancel download"
                                                    on:click=move |_| {
                                                        let model = model_for_cancel.clone();
                                                        let model_update = model_for_cancel_update.clone();
                                                        spawn_local(async move {
                                                            let _ = cancel_model_pull(model).await;
                                                        });
                                                        set_active_downloads.update(|downloads| {
                                                            if let Some(d) = downloads.iter_mut().find(|d| d.model == model_update) {
                                                                d.done = true;
                                                                d.status = "Cancelled".to_string();
                                                            }
                                                        });
                                                    }>
                                                "✕"
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }}
                                    // Hide button - just removes from UI
                                    <button class="download-hide"
                                            title="Hide"
                                            on:click=move |_| {
                                                set_active_downloads.update(|downloads| {
                                                    downloads.retain(|d| d.model != model_for_hide);
                                                });
                                            }>
                                        "−"
                                    </button>
                                </div>
                                <div class="progress-track">
                                    <div class="progress-fill"
                                         style:width=format!("{}%", percent)>
                                    </div>
                                </div>
                            </div>
                        }
                    }).collect_view()
                }}
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
