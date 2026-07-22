#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::Local;
use iced::{Element, Size, Subscription, Task, Theme, clipboard, keyboard, time};
use iced_widget::markdown;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::GenerationResponse;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::models::ModelOptions;
use rustrict::Censor;
use serde_json;
use webbrowser;

use image;

mod app;
mod gui;

use crate::app::{
    AppState, Channels, ChatImage, Correspondence, CurrentChat, DebugMessage, History,
    HostLocation, Log, Prompt, Response, SavedChat, SystemPrompt, ThinkingLevel, UserInformation,
};

/// Tick points:
/// Each tick occurs every TICK_MS; these constants decide what happens on each tick.
const VERSION_TICK: i32 = 2;
const MAX_TICK: i32 = 50;
const BOT_LIST_TICK: i32 = 3;
const TICK_MS: u64 = 200;

const APP_VERSION: &str = "0.4.1";

#[derive(PartialEq, Clone, Copy)]
pub enum GUIState {
    InfoPopup,
    Main,
    Settings,
    AdvancedSettings,
    Images,
}

#[derive(Debug, Clone)]
enum Message {
    ChangeBatchTokens(i32),
    ToggleFastStreaming,
    ToggleChatMenu,
    NewChat,
    OpenChat(String),
    ToggleChatPin(String),
    DeleteChat(String),
    ToggleTemporaryChat,
    ChooseChatFolder,
    ChatFolderSelected(Option<PathBuf>),
    AsyncResult(()),
    ListPrompt,
    ThinkingLevelChange(ThinkingLevel),
    ToggleImages,
    PickImage,
    DropImage(PathBuf),
    PasteImage,
    ImageLoaded(Result<ChatImage, String>),
    RemoveImage,
    GenerateImage,
    ImageGenerated(Result<String, String>),
    CopyImage(String),
    ModelCapabilityKnown(String, Option<bool>),
    ToggleSettings,
    SystemPromptChange(String),
    Prompt(String),
    StopResponse,
    UpdatePrompt(String),
    None,
    KeyPressed(keyboard::Key, keyboard::Modifiers),
    KeyReleased(keyboard::Key),
    Tick,
    CopyPressed(String),
    ToggleThinking(usize),
    UpdateTextSize(f32),
    InstallationPrompt,
    ModelChange(String),
    InstallModel(String),
    UpdateInstall(String),
    UpdateTemperature(f32),
    ToggleInfoPopup,
    ToggleChatHistory,
    WipeChatHistory,
    ToggleAdvancedSettings,
    ChangeIp(String),
    ChangePort(String),
}

struct Program {
    is_processing: bool,
    response_cancel: Option<Arc<AtomicBool>>,
    current_tick: i32,
    installing_model: String,

    debug_message: DebugMessage,
    debug_message_set_at: Option<Instant>,

    /// Parsed markdown cache for finished chat messages.
    /// This is needed because markdown::view borrows parsed markdown items.
    chat_markdown_cache: Vec<Vec<markdown::Item>>,

    /// One model label per chat message.
    /// User messages use None. Bot messages store the model that generated them.
    chat_model_name_cache: Vec<Option<String>>,

    /// Model currently generating a response. This prevents finished messages from
    /// being relabelled if the user changes the dropdown later.
    active_response_model_name: Option<String>,

    /// Used for brief copy feedback animations/buttons.
    last_copied_text: Option<String>,
    last_copied_at: Option<Instant>,

    pending_image: Option<ChatImage>,
    generated_images: Vec<String>,
    is_generating_image: bool,

    /// Message indexes whose reasoning disclosure is open. Reasoning is hidden by default.
    expanded_thinking: HashSet<usize>,

    system_prompt: SystemPrompt,
    app_state: AppState,
    channels: Channels,
    user_information: UserInformation,
    response: Response,
    prompt: Prompt,
    batch_tokens: i32,
    fast_streaming: bool,
    chat_menu_open: bool,
    temporary_chat: bool,
    current_chat_id: String,
    saved_chats: Vec<SavedChat>,
    chat_storage_dir: PathBuf,
}

fn default_chat_storage_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(base).join("Ollama GUI").join("chats");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Library/Application Support/Ollama GUI/chats");
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(base) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(base).join("ollama-gui").join("chats");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local/share/ollama-gui/chats");
        }
    }
    PathBuf::from("output/chats")
}

fn chat_location_settings_path() -> PathBuf {
    default_chat_storage_dir()
        .parent()
        .unwrap_or_else(|| Path::new("output"))
        .join("chat-location.json")
}

/// Separates model reasoning from the user-facing answer. Unclosed tags are
/// treated as in-progress reasoning, which keeps streamed chain-of-thought out
/// of the transcript until the closing tag arrives.
fn split_thinking_text(input: &str) -> (String, String) {
    let mut thinking = String::new();
    let mut visible = String::new();
    let mut rest = input;

    while let Some(open) = rest.find("<think>") {
        visible.push_str(&rest[..open]);
        let after_open = &rest[open + "<think>".len()..];
        if let Some(close) = after_open.find("</think>") {
            thinking.push_str(&after_open[..close]);
            rest = &after_open[close + "</think>".len()..];
        } else {
            thinking.push_str(after_open);
            rest = "";
            break;
        }
    }
    visible.push_str(rest);

    (
        thinking.trim().to_string(),
        visible.trim_start().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::split_thinking_text;

    #[test]
    fn separates_thinking_from_answer() {
        assert_eq!(
            split_thinking_text("<think>work it out</think>The answer."),
            ("work it out".into(), "The answer.".into())
        );
    }

    #[test]
    fn combines_streamed_thinking_blocks() {
        assert_eq!(
            split_thinking_text("<think>first </think><think>second</think>Done"),
            ("first second".into(), "Done".into())
        );
    }

    #[test]
    fn hides_unclosed_streaming_thinking() {
        assert_eq!(
            split_thinking_text("Intro<think>still reasoning"),
            ("still reasoning".into(), "Intro".into())
        );
    }
}

fn convert_port_to_u16(port: String) -> u16 {
    match port.parse::<u16>() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Invalid port number: {}", port);
            11434
        }
    }
}

fn load_chat_image(path: &Path) -> Result<ChatImage, String> {
    let bytes = fs::read(path).map_err(|error| format!("Could not read image: {error}"))?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Err("Images must be smaller than 20 MB.".to_string());
    }
    image::load_from_memory(&bytes).map_err(|error| format!("Unsupported image: {error}"))?;
    let mime_type = match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/png",
    };
    let preview_handle = iced::widget::image::Handle::from_bytes(bytes.clone());
    Ok(ChatImage {
        name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("image")
            .to_string(),
        mime_type: mime_type.to_string(),
        bytes,
        preview_handle,
    })
}

fn paste_chat_image() -> Result<ChatImage, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("Could not open clipboard: {error}"))?;
    let image_data = clipboard
        .get_image()
        .map_err(|_| "The clipboard does not contain an image.".to_string())?;
    let rgba = image::RgbaImage::from_raw(
        image_data.width as u32,
        image_data.height as u32,
        image_data.bytes.into_owned(),
    )
    .ok_or_else(|| "Clipboard image data was invalid.".to_string())?;
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|error| format!("Could not prepare clipboard image: {error}"))?;
    let preview_handle = iced::widget::image::Handle::from_bytes(bytes.clone());
    Ok(ChatImage {
        name: "Pasted image.png".to_string(),
        mime_type: "image/png".to_string(),
        bytes,
        preview_handle,
    })
}

fn copy_image_file(path: &str) -> Result<(), String> {
    let decoded = image::open(path)
        .map_err(|error| format!("Could not open image: {error}"))?
        .into_rgba8();
    let (width, height) = decoded.dimensions();
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("Could not open clipboard: {error}"))?;
    clipboard
        .set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: std::borrow::Cow::Owned(decoded.into_raw()),
        })
        .map_err(|error| format!("Could not copy image: {error}"))
}

impl Program {
    fn new_chat_id() -> String {
        format!(
            "chat-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    }

    fn clear_open_chat(&mut self) {
        self.user_information.chat_history = Arc::new(Mutex::new(CurrentChat {
            chats: vec![],
            messages: vec![],
            bot_responding: false,
        }));
        self.chat_markdown_cache.clear();
        self.chat_model_name_cache.clear();
        self.active_response_model_name = None;
        self.last_copied_text = None;
        self.last_copied_at = None;
        self.response.parsed_markdown.clear();
        self.expanded_thinking.clear();
        if let Ok(mut response_text) = self.response.response_as_string.lock() {
            response_text.clear();
        }
    }

    fn save_open_chat(&mut self) {
        if self.temporary_chat {
            return;
        }
        let chat = self.user_information.chat_history.lock().unwrap().clone();
        if chat.messages.is_empty() {
            return;
        }
        let title = chat
            .messages
            .iter()
            .find_map(|message| match message {
                Correspondence::User { text, .. } => Some(text.trim().chars().take(42).collect()),
                _ => None,
            })
            .filter(|title: &String| !title.is_empty())
            .unwrap_or_else(|| "New chat".into());
        let mut saved = SavedChat::from_current(self.current_chat_id.clone(), title, &chat);
        if let Some(existing) = self.saved_chats.iter_mut().find(|item| item.id == saved.id) {
            saved.pinned = existing.pinned;
            *existing = saved;
        } else {
            // New chats appear after the pinned section. Updating or opening an
            // existing chat deliberately leaves it at its current position.
            let insert_at = self
                .saved_chats
                .iter()
                .position(|chat| !chat.pinned)
                .unwrap_or(self.saved_chats.len());
            self.saved_chats.insert(insert_at, saved);
        }
        if let Err(error) = fs::create_dir_all(&self.chat_storage_dir).and_then(|_| {
            fs::write(
                self.chat_storage_dir.join("chats.json"),
                serde_json::to_string_pretty(&self.saved_chats).unwrap_or_else(|_| "[]".into()),
            )
        }) {
            self.set_debug_message(DebugMessage {
                message: format!("Could not save chats: {error}"),
                is_error: true,
            });
        }
    }

    fn persist_saved_chats(&mut self) {
        if let Err(error) = fs::create_dir_all(&self.chat_storage_dir).and_then(|_| {
            fs::write(
                self.chat_storage_dir.join("chats.json"),
                serde_json::to_string_pretty(&self.saved_chats).unwrap_or_else(|_| "[]".into()),
            )
        }) {
            self.set_debug_message(DebugMessage {
                message: format!("Could not update saved chats: {error}"),
                is_error: true,
            });
        }
    }

    fn persist_boolean_setting(&mut self, key: &str, value: bool) {
        let mut settings: serde_json::Map<String, serde_json::Value> =
            fs::read_to_string("./config/settings.json")
                .ok()
                .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
                .and_then(|value| value.as_object().cloned())
                .unwrap_or_default();
        settings.insert(key.to_string(), serde_json::Value::Bool(value));
        if let Err(error) = fs::write(
            "./config/settings.json",
            serde_json::to_string_pretty(&settings).unwrap_or_else(|_| "{}".into()),
        ) {
            self.set_debug_message(DebugMessage {
                message: format!("Could not save setting: {error}"),
                is_error: true,
            });
        }
    }

    fn persist_chat_storage_dir(&mut self) -> Result<(), String> {
        let settings_path = chat_location_settings_path();
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let value = serde_json::json!({
            "chat_storage_dir": self.chat_storage_dir.to_string_lossy()
        });
        fs::write(
            settings_path,
            serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }

    fn set_debug_message(&mut self, debug_message: DebugMessage) {
        let has_message = !debug_message.message.trim().is_empty();

        self.debug_message = debug_message;
        self.debug_message_set_at = if has_message {
            Some(Instant::now())
        } else {
            None
        };
    }

    fn clear_debug_message_if_old(&mut self) {
        if let Some(set_at) = self.debug_message_set_at {
            if set_at.elapsed() >= Duration::from_secs(15) {
                self.debug_message.message.clear();
                self.debug_message.is_error = false;
                self.debug_message_set_at = None;
            }
        }
    }

    fn refresh_chat_markdown_cache(&mut self) {
        let messages = {
            let chat_history = self.user_information.chat_history.lock().unwrap();
            chat_history.messages.clone()
        };

        let old_markdown_cache = self.chat_markdown_cache.clone();
        let old_model_name_cache = self.chat_model_name_cache.clone();

        let mut new_markdown_cache: Vec<Vec<markdown::Item>> = Vec::with_capacity(messages.len());
        let mut new_model_name_cache: Vec<Option<String>> = Vec::with_capacity(messages.len());

        for (index, message) in messages.iter().enumerate() {
            match message {
                Correspondence::User { .. } => {
                    new_markdown_cache.push(Vec::new());
                    new_model_name_cache.push(None);
                }

                Correspondence::Bot(text) => {
                    let (_, visible_text) = split_thinking_text(text);
                    if let Some(cached) = old_markdown_cache.get(index) {
                        new_markdown_cache.push(cached.clone());
                    } else {
                        new_markdown_cache.push(markdown::parse(&visible_text).collect());
                    }

                    let model_name = old_model_name_cache
                        .get(index)
                        .cloned()
                        .flatten()
                        .or_else(|| self.active_response_model_name.clone())
                        .or_else(|| Some("Unknown model".to_string()));

                    new_model_name_cache.push(model_name);
                }
            }
        }

        self.chat_markdown_cache = new_markdown_cache;
        self.chat_model_name_cache = new_model_name_cache;
    }

    fn clear_copy_feedback_if_old(&mut self) {
        if let Some(copied_at) = self.last_copied_at {
            if copied_at.elapsed() >= Duration::from_millis(1400) {
                self.last_copied_text = None;
                self.last_copied_at = None;
            }
        }
    }

    fn prompt(&mut self, prompt: String) -> Task<Message> {
        if self.user_information.model == None {
            Channels::send_request_to_channel(Arc::clone(&self.channels.debounce_channel), false);
            Channels::send_request_to_channel(
                Arc::clone(&self.channels.debug_channel),
                DebugMessage {
                    message: "Model selected is invalid, have you selected a model?".to_string(),
                    is_error: true,
                },
            );
            println!("Model is None");
            return Task::none();
        }

        self.active_response_model_name = self.user_information.model.clone();
        self.prompt.prompt_time_sent = Instant::now();

        let cancel = Arc::new(AtomicBool::new(false));
        self.response_cancel = Some(Arc::clone(&cancel));

        let (markdown_sender, markdown_receiver) = crossbeam_channel::unbounded();
        self.channels.markdown_channel_reciever = markdown_receiver;

        let (tx, rx) = std::sync::mpsc::channel::<GenerationResponse>();
        let channels: Channels = self.channels.clone();
        let batch_tokens = self.batch_tokens.clone();
        let fast_streaming = self.fast_streaming;
        let response_string = Arc::clone(&self.response.response_as_string);

        std::thread::spawn(move || {
            fn render(
                buffer: &String,
                markdown_sender: crossbeam_channel::Sender<Vec<markdown::Item>>,
                channels: Channels,
            ) {
                let (_, visible) = split_thinking_text(buffer);
                let md = markdown::parse(&visible).collect();

                match markdown_sender.send(md) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to send markdown response: {}", e);
                        Channels::send_request_to_channel(
                            Arc::clone(&channels.debug_channel),
                            DebugMessage {
                                message:
                                    "Failed to create markdown response [markdown_sender.send failed]"
                                        .to_string(),
                                is_error: true,
                            },
                        );
                    }
                };
            }

            let mut buffer = String::new();
            let mut last_render_time = Instant::now();
            let mut total_tokens = 0;

            for token in rx {
                buffer.push_str(&token.response);

                if let Ok(mut current_response) = response_string.lock() {
                    *current_response = buffer.clone();
                }

                total_tokens += 1;

                if !(fast_streaming
                    || total_tokens >= batch_tokens
                    || last_render_time.elapsed().as_millis() >= 250)
                {
                    continue;
                }

                total_tokens = 0;
                last_render_time = Instant::now();

                render(&buffer, markdown_sender.clone(), channels.clone());
            }

            if !buffer.is_empty() {
                if let Ok(mut current_response) = response_string.lock() {
                    *current_response = buffer.clone();
                }

                render(&buffer, markdown_sender.clone(), channels.clone());
            }
        });

        let system_prompt: Option<String> = SystemPrompt::get_current(&self);

        if system_prompt.is_none() {
            Channels::send_request_to_channel(
                Arc::clone(&self.channels.debug_channel),
                DebugMessage {
                    message: "Could not get system prompt, is it selected?".to_string(),
                    is_error: true,
                },
            );
            Channels::send_request_to_channel(Arc::clone(&self.channels.debounce_channel), false);
            return Task::none();
        }

        // Clone the attachment into the request/chat first. The composer owns its
        // copy until the submission has been accepted, avoiding a transient blank
        // preview while the async request is being prepared.
        let attached_image = self.pending_image.clone();
        let logging = self.app_state.logging.clone();
        let filtering = self.app_state.filtering.clone();
        let user_info = self.user_information.clone();
        let channels = self.channels.clone();

        user_info
            .chat_history
            .lock()
            .unwrap()
            .push_message(Correspondence::User {
                text: prompt.clone(),
                image: attached_image.clone(),
            });

        self.refresh_chat_markdown_cache();
        self.pending_image = None;

        Task::perform(
            async move {
                println!("Received prompt: {}", prompt.clone());
                user_info.chat_history.lock().unwrap().bot_responding = true;

                let system_prompt: String = system_prompt.unwrap();
                let ip = user_info.ip_address.clone();
                let to_send_prompt: String = if user_info.current_chat_history_enabled {
                    format!(
                        "The following is a conversation between an AI language model and a User. You are the AI language model:
                    {}
                    [END CONVERSATION CONTEXT]
                    Now, the user is sending another message: {}
                    Respond:
                    ",
                        user_info.chat_history.lock().unwrap().unravel(),
                        prompt.clone()
                    )
                } else {
                    prompt.clone()
                };

                let request: GenerationRequest<'_> =
                    GenerationRequest::new(user_info.model.clone().unwrap(), to_send_prompt)
                        .options(ModelOptions::default().temperature(user_info.temperature / 10.0))
                        .system(system_prompt.clone());

                println!("System prompt: {}", system_prompt.clone());

                let mut request_body = match serde_json::to_value(request) {
                    Ok(body) => body,
                    Err(e) => {
                        eprintln!("Error serializing request: {}", e);
                        Channels::send_request_to_channel(
                            Arc::clone(&channels.debug_channel),
                            DebugMessage {
                                message: "Could not prepare the Ollama request".to_string(),
                                is_error: true,
                            },
                        );
                        Channels::send_request_to_channel(
                            Arc::clone(&channels.debounce_channel),
                            false,
                        );
                        user_info.chat_history.lock().unwrap().bot_responding = false;
                        return;
                    }
                };

                request_body["stream"] = serde_json::Value::Bool(true);
                request_body["think"] = user_info.thinking_level.api_value();
                if let Some(image) = attached_image {
                    request_body["images"] = serde_json::json!([BASE64.encode(image.bytes)]);
                }

                let url = format!("http://{}:{}/api/generate", ip.ip, ip.port);
                let mut response = match reqwest::Client::new()
                    .post(url)
                    .json(&request_body)
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => response,
                    Ok(response) => {
                        let status = response.status();
                        let detail = response.text().await.unwrap_or_default();
                        Channels::send_request_to_channel(
                            Arc::clone(&channels.debug_channel),
                            DebugMessage {
                                message: format!(
                                    "Ollama rejected the request ({status}): {detail}"
                                ),
                                is_error: true,
                            },
                        );
                        Channels::send_request_to_channel(
                            Arc::clone(&channels.debounce_channel),
                            false,
                        );
                        user_info.chat_history.lock().unwrap().bot_responding = false;
                        return;
                    }
                    Err(error) => {
                        Channels::send_request_to_channel(
                            Arc::clone(&channels.debug_channel),
                            DebugMessage {
                                message: format!("Could not reach Ollama: {error}"),
                                is_error: true,
                            },
                        );
                        Channels::send_request_to_channel(
                            Arc::clone(&channels.debounce_channel),
                            false,
                        );
                        user_info.chat_history.lock().unwrap().bot_responding = false;
                        return;
                    }
                };

                let mut final_response: Vec<String> = vec![];
                let mut stream_buffer = String::new();

                while !cancel.load(Ordering::Relaxed) {
                    let Ok(Some(chunk)) = response.chunk().await else {
                        break;
                    };
                    stream_buffer.push_str(&String::from_utf8_lossy(&chunk));
                    while let Some(newline) = stream_buffer.find('\n') {
                        let line = stream_buffer[..newline].trim().to_string();
                        stream_buffer.drain(..=newline);
                        if line.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<GenerationResponse>(&line) {
                            Ok(mut token) => {
                                print!("{}", token.response);

                                // Ollama may return reasoning in its dedicated `thinking`
                                // field, while some models emit literal <think> tags.
                                // Normalize both forms so the renderer can disclose them alike.
                                if let Some(thinking) = token.thinking.take() {
                                    if !thinking.is_empty() {
                                        token.response =
                                            format!("<think>{thinking}</think>{}", token.response);
                                    }
                                }

                                let filtered_token: GenerationResponse = if filtering {
                                    GenerationResponse {
                                        response: Censor::from_str(token.response.as_str())
                                            .censor(),
                                        ..token
                                    }
                                } else {
                                    token
                                };

                                final_response.push(filtered_token.clone().response);

                                if tx.send(filtered_token).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error decoding Ollama response: {}", e);
                                Channels::send_request_to_channel(
                                    Arc::clone(&channels.debug_channel),
                                    DebugMessage {
                                        message: "Ollama returned an invalid streaming response"
                                            .to_string(),
                                        is_error: true,
                                    },
                                );
                            }
                        }
                    }
                }

                let was_cancelled = cancel.load(Ordering::Relaxed);

                if logging && !was_cancelled {
                    Channels::send_request_to_channel(
                        Arc::clone(&channels.logging_channel),
                        Log::create_with_current_time(
                            filtering,
                            user_info.model,
                            final_response.clone(),
                            Some(system_prompt),
                            prompt.clone(),
                        ),
                    );
                }

                if user_info.current_chat_history_enabled && !was_cancelled {
                    let complete = final_response.join("");
                    let (_, visible_response) = split_thinking_text(&complete);
                    user_info
                        .chat_history
                        .lock()
                        .unwrap()
                        .generate_and_push(prompt.clone(), visible_response);
                }

                let partial_response = final_response.join("");
                if !partial_response.is_empty() {
                    user_info
                        .chat_history
                        .lock()
                        .unwrap()
                        .push_message(Correspondence::Bot(partial_response));
                }

                user_info.chat_history.lock().unwrap().bot_responding = false;

                Channels::send_request_to_channel(Arc::clone(&channels.debounce_channel), false);
            },
            |result| Message::AsyncResult(result),
        )
    }

    fn boot() -> (Program, Task<Message>) {
        (Program::default(), Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AsyncResult(_result) => Task::none(),

            Message::None => Task::none(),

            Message::ToggleImages => {
                self.app_state.gui_state = if self.app_state.gui_state == GUIState::Images {
                    GUIState::Main
                } else {
                    GUIState::Images
                };
                Task::none()
            }

            Message::PickImage => Task::perform(
                async {
                    let path = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
                        .pick_file()
                        .ok_or_else(|| "No image selected.".to_string())?;
                    load_chat_image(&path)
                },
                Message::ImageLoaded,
            ),

            Message::DropImage(path) => {
                Task::perform(async move { load_chat_image(&path) }, Message::ImageLoaded)
            }

            Message::PasteImage => {
                Task::perform(async { paste_chat_image() }, Message::ImageLoaded)
            }

            Message::ImageLoaded(result) => {
                match result {
                    Ok(image) => {
                        let name = image.name.clone();
                        self.pending_image = Some(image);
                        self.set_debug_message(DebugMessage {
                            message: format!("Attached {name}."),
                            is_error: false,
                        });
                    }
                    Err(error) if error != "No image selected." => {
                        self.set_debug_message(DebugMessage {
                            message: error,
                            is_error: true,
                        })
                    }
                    Err(_) => {}
                }
                Task::none()
            }

            Message::RemoveImage => {
                self.pending_image = None;
                Task::none()
            }

            Message::CopyImage(path) => {
                Task::perform(async move { copy_image_file(&path) }, |result| {
                    Message::ImageGenerated(result.map(|_| "__copied__".to_string()))
                })
            }

            Message::GenerateImage => {
                if self.is_generating_image {
                    return Task::none();
                }
                let Some(model) = self.user_information.model.clone() else {
                    self.set_debug_message(DebugMessage {
                        message: "Select an image-generation model first.".to_string(),
                        is_error: true,
                    });
                    return Task::none();
                };
                let prompt = self.prompt.prompt.trim().to_string();
                if prompt.is_empty() {
                    self.set_debug_message(DebugMessage {
                        message: "Describe the image you want to generate.".to_string(),
                        is_error: true,
                    });
                    return Task::none();
                }
                self.is_generating_image = true;
                let host = format!(
                    "http://{}:{}",
                    self.user_information.ip_address.ip, self.user_information.ip_address.port
                );
                Task::perform(
                    async move {
                        let output_dir = PathBuf::from("output/generated");
                        fs::create_dir_all(&output_dir).map_err(|error| {
                            format!("Could not create image output folder: {error}")
                        })?;
                        let before = fs::read_dir(&output_dir)
                            .ok()
                            .into_iter()
                            .flatten()
                            .filter_map(Result::ok)
                            .map(|entry| entry.path())
                            .collect::<Vec<_>>();
                        let output = std::process::Command::new("ollama")
                            .arg("run")
                            .arg(&model)
                            .arg(&prompt)
                            .env("OLLAMA_HOST", host)
                            .current_dir(&output_dir)
                            .output()
                            .map_err(|error| {
                                format!("Could not start Ollama image generation: {error}")
                            })?;
                        if !output.status.success() {
                            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
                        }
                        let mut candidates = fs::read_dir(&output_dir)
                            .map_err(|error| error.to_string())?
                            .filter_map(Result::ok)
                            .map(|entry| entry.path())
                            .filter(|path| !before.contains(path))
                            .filter(|path| {
                                matches!(
                                    path.extension()
                                        .and_then(|ext| ext.to_str())
                                        .map(str::to_ascii_lowercase)
                                        .as_deref(),
                                    Some("png" | "jpg" | "jpeg" | "webp")
                                )
                            })
                            .collect::<Vec<_>>();
                        candidates.sort_by_key(|path| {
                            fs::metadata(path).and_then(|meta| meta.modified()).ok()
                        });
                        candidates.pop().map(|path| path.to_string_lossy().to_string())
                        .ok_or_else(|| "Ollama finished but did not produce an image. This model or platform may not support experimental image generation.".to_string())
                    },
                    Message::ImageGenerated,
                )
            }

            Message::ImageGenerated(result) => {
                self.is_generating_image = false;
                match result {
                    Ok(marker) if marker == "__copied__" => self.set_debug_message(DebugMessage {
                        message: "Image copied to clipboard.".to_string(),
                        is_error: false,
                    }),
                    Ok(path) => {
                        self.generated_images.push(path);
                        self.set_debug_message(DebugMessage {
                            message: "Image generated locally.".to_string(),
                            is_error: false,
                        });
                    }
                    Err(error) => self.set_debug_message(DebugMessage {
                        message: error,
                        is_error: true,
                    }),
                }
                Task::none()
            }

            Message::ChangeBatchTokens(new_batch_tokens) => {
                self.batch_tokens = new_batch_tokens;
                Task::none()
            }

            Message::ToggleFastStreaming => {
                self.fast_streaming = !self.fast_streaming;
                self.persist_boolean_setting("fast_streaming", self.fast_streaming);
                Task::none()
            }

            Message::ToggleChatMenu => {
                self.chat_menu_open = !self.chat_menu_open;
                Task::none()
            }

            Message::NewChat => {
                if self.is_processing {
                    return Task::none();
                }
                self.save_open_chat();
                self.current_chat_id = Self::new_chat_id();
                self.temporary_chat = false;
                self.clear_open_chat();
                Task::none()
            }

            Message::OpenChat(id) => {
                if self.is_processing {
                    return Task::none();
                }
                self.save_open_chat();
                if let Some(saved) = self.saved_chats.iter().find(|chat| chat.id == id).cloned() {
                    self.current_chat_id = saved.id.clone();
                    self.temporary_chat = false;
                    self.user_information.chat_history = Arc::new(Mutex::new(saved.to_current()));
                    self.response.parsed_markdown.clear();
                    if let Ok(mut text) = self.response.response_as_string.lock() {
                        text.clear();
                    }
                    self.refresh_chat_markdown_cache();
                }
                Task::none()
            }

            Message::DeleteChat(id) => {
                if self.is_processing {
                    return Task::none();
                }
                self.saved_chats.retain(|chat| chat.id != id);
                if self.current_chat_id == id {
                    self.current_chat_id = Self::new_chat_id();
                    self.clear_open_chat();
                }
                self.persist_saved_chats();
                Task::none()
            }

            Message::ToggleChatPin(id) => {
                if self.is_processing {
                    return Task::none();
                }
                if let Some(chat) = self.saved_chats.iter_mut().find(|chat| chat.id == id) {
                    chat.pinned = !chat.pinned;
                    // Stable sorting changes only the toggled chat's section and
                    // preserves the relative order of every other chat.
                    self.saved_chats.sort_by_key(|chat| !chat.pinned);
                    self.persist_saved_chats();
                }
                Task::none()
            }

            Message::ToggleTemporaryChat => {
                if self.is_processing {
                    return Task::none();
                }
                if self.temporary_chat {
                    self.temporary_chat = false;
                    self.current_chat_id = Self::new_chat_id();
                    self.clear_open_chat();
                } else {
                    self.save_open_chat();
                    self.temporary_chat = true;
                    self.current_chat_id = Self::new_chat_id();
                    self.clear_open_chat();
                }
                Task::none()
            }

            Message::ChooseChatFolder => Task::perform(
                async { rfd::FileDialog::new().pick_folder() },
                Message::ChatFolderSelected,
            ),

            Message::ChatFolderSelected(Some(folder)) => {
                self.save_open_chat();
                self.chat_storage_dir = folder;
                self.saved_chats = fs::read_to_string(self.chat_storage_dir.join("chats.json"))
                    .ok()
                    .and_then(|data| serde_json::from_str(&data).ok())
                    .unwrap_or_default();
                let result = fs::create_dir_all(&self.chat_storage_dir)
                    .map_err(|error| error.to_string())
                    .and_then(|_| self.persist_chat_storage_dir());
                self.set_debug_message(match result {
                    Ok(()) => DebugMessage {
                        message: format!(
                            "Chats will be saved to {}",
                            self.chat_storage_dir.display()
                        ),
                        is_error: false,
                    },
                    Err(error) => DebugMessage {
                        message: format!("Could not use that chat folder: {error}"),
                        is_error: true,
                    },
                });
                Task::none()
            }

            Message::ChatFolderSelected(None) => Task::none(),

            Message::Tick => {
                self.clear_debug_message_if_old();
                self.clear_copy_feedback_if_old();

                if self.current_tick > MAX_TICK {
                    println!("Resetting current tick");
                    self.current_tick = 0;
                }

                self.current_tick += 1;

                if self.current_tick == VERSION_TICK {
                    let ollama_state = Arc::clone(&self.app_state.ollama_state);
                    let user_info = self.user_information.clone();

                    return Task::perform(
                        async move {
                            println!("Checking Ollama version...");
                            let ip = user_info.ip_address;
                            let url = format!("http://{}:{}/api/version", ip.ip, ip.port);

                            match reqwest::get(url).await {
                                Ok(response) => {
                                    println!("API responded with status: {}", response.status());

                                    if response.status().is_success() {
                                        match response.json::<serde_json::Value>().await {
                                            Ok(json) => {
                                                if let Some(version) =
                                                    json.get("version").and_then(|v| v.as_str())
                                                {
                                                    *ollama_state.lock().unwrap() =
                                                        format!("Online (v{})", version);
                                                } else {
                                                    *ollama_state.lock().unwrap() =
                                                        "Online (unknown version)".to_string();
                                                }
                                            }
                                            Err(_) => {
                                                *ollama_state.lock().unwrap() =
                                                    "Online (version parse error)".to_string();
                                            }
                                        }
                                    } else {
                                        *ollama_state.lock().unwrap() = "Offline".to_string();
                                    }
                                }
                                Err(err) => {
                                    println!("Failed to reach API: {}", err);
                                    *ollama_state.lock().unwrap() = "Offline".to_string();
                                }
                            }
                        },
                        |result| Message::AsyncResult(result),
                    );
                } else if self.current_tick == BOT_LIST_TICK {
                    let ip = self.user_information.ip_address.clone();
                    let ollama = Ollama::builder()
                        .host(format!("http://{}", ip.ip))
                        .port(convert_port_to_u16(ip.port))
                        .build();
                    let bots_list = Arc::clone(&self.app_state.bots_list);
                    let channels = self.channels.clone();

                    return Task::perform(
                        async move {
                            match ollama.list_local_models().await {
                                Ok(bots) => {
                                    bots.iter().for_each(|bot| {
                                        if !(bots_list
                                            .lock()
                                            .unwrap()
                                            .contains(&bot.name.to_string()))
                                        {
                                            println!("Found bot: {}", bot.name);
                                            bots_list.lock().unwrap().push(bot.name.to_string());
                                        }
                                    });
                                }
                                Err(e) => {
                                    Channels::send_request_to_channel(
                                        Arc::clone(&channels.debug_channel),
                                        DebugMessage {
                                            message: "Error occurred while listing bots"
                                                .to_string(),
                                            is_error: true,
                                        },
                                    );
                                    bots_list.lock().unwrap().clear();
                                    println!("Error: {:?}", e);
                                }
                            }
                        },
                        |result| Message::AsyncResult(result),
                    );
                }

                if let Ok(md) = self.channels.markdown_channel_reciever.try_recv() {
                    self.response.parsed_markdown = md;
                }

                let debounce_result = {
                    let guard = self.channels.debounce_channel.lock().unwrap();
                    guard.1.try_recv()
                };

                if let Ok(is_processing) = debounce_result {
                    self.is_processing = is_processing;

                    if !is_processing {
                        self.refresh_chat_markdown_cache();
                        self.active_response_model_name = None;
                        self.save_open_chat();
                    }
                }

                let debug_result = {
                    let guard = self.channels.debug_channel.lock().unwrap();
                    guard.1.try_recv()
                };

                if let Ok(debug_msg) = debug_result {
                    self.set_debug_message(debug_msg);
                }

                let log_result = {
                    let guard = self.channels.logging_channel.lock().unwrap();
                    guard.1.try_recv()
                };

                if let Ok(log) = log_result {
                    self.app_state.logs.push_log(log);

                    match fs::write(
                        "./output/history.json",
                        serde_json::to_string_pretty(&self.app_state.logs).unwrap(),
                    ) {
                        Ok(_) => {}
                        Err(_) => {
                            eprintln!("An error writing to history.json");
                            self.set_debug_message(DebugMessage {
                                message: "Failed to write to history.json".to_string(),
                                is_error: true,
                            });
                        }
                    };
                }

                self.refresh_chat_markdown_cache();

                Task::none()
            }

            Message::ChangeIp(ip) => {
                self.user_information.ip_address.ip = ip;
                Task::none()
            }

            Message::ChangePort(port) => {
                self.user_information.ip_address.port = port;
                Task::none()
            }

            Message::ToggleChatHistory => {
                self.user_information.current_chat_history_enabled =
                    !self.user_information.current_chat_history_enabled;
                Task::none()
            }

            Message::WipeChatHistory => {
                // The saved conversation keeps its id and remains on disk. Further
                // messages start a fresh chat, so clearing context cannot overwrite it.
                self.current_chat_id = Self::new_chat_id();
                self.clear_open_chat();

                self.set_debug_message(DebugMessage {
                    message: "Current model context cleared. Saved chats were not deleted."
                        .to_string(),
                    is_error: false,
                });

                Task::none()
            }

            Message::UpdateTextSize(n) => {
                self.user_information.text_size = n;
                Task::none()
            }

            Message::ToggleInfoPopup => {
                if self.app_state.gui_state == GUIState::InfoPopup {
                    self.app_state.gui_state = GUIState::Main;
                } else {
                    self.app_state.gui_state = GUIState::InfoPopup;
                }

                Task::none()
            }

            Message::ToggleSettings => {
                if self.app_state.gui_state == GUIState::Settings {
                    self.app_state.gui_state = GUIState::Main;
                } else {
                    self.app_state.gui_state = GUIState::Settings;
                }

                Task::none()
            }

            Message::ToggleAdvancedSettings => {
                if self.app_state.gui_state == GUIState::AdvancedSettings {
                    self.app_state.gui_state = GUIState::Settings;
                } else {
                    self.app_state.gui_state = GUIState::AdvancedSettings;
                }

                Task::none()
            }

            Message::UpdateTemperature(n) => {
                self.user_information.temperature = n;
                Task::none()
            }

            Message::ThinkingLevelChange(level) => {
                self.user_information.thinking_level = level;
                Task::none()
            }

            Message::ToggleThinking(index) => {
                if !self.expanded_thinking.insert(index) {
                    self.expanded_thinking.remove(&index);
                }
                Task::none()
            }

            Message::ModelCapabilityKnown(model, supported) => {
                if self.user_information.model.as_ref() == Some(&model) {
                    self.user_information.thinking_supported = supported;
                    if supported == Some(false) {
                        self.user_information.thinking_level = ThinkingLevel::Off;
                    }
                }
                Task::none()
            }

            Message::SystemPromptChange(system_prompt) => {
                self.system_prompt.system_prompt = Some(system_prompt);
                Task::none()
            }

            Message::InstallModel(model_install) => {
                Channels::send_request_to_channel(
                    Arc::clone(&self.channels.debug_channel),
                    DebugMessage {
                        message: format!("Installing model... {}", model_install),
                        is_error: false,
                    },
                );

                let ip = self.user_information.ip_address.clone();
                let ollama = Ollama::builder()
                    .host(format!("http://{}", ip.ip))
                    .port(convert_port_to_u16(ip.port))
                    .build();
                let channels = self.channels.clone();

                Task::perform(
                    async move {
                        match ollama.pull_model(model_install.clone(), false).await {
                            Ok(outcome) => {
                                println!(
                                    "Model {} installed successfully: {}",
                                    model_install, outcome.message
                                );
                                Channels::send_request_to_channel(
                                    Arc::clone(&channels.debug_channel),
                                    DebugMessage {
                                        message: format!(
                                            "Installed model {}: {}",
                                            model_install, outcome.message
                                        ),
                                        is_error: false,
                                    },
                                );
                            }
                            Err(outcome) => {
                                println!(
                                    "Failed to install model {}: {:?}",
                                    model_install, outcome
                                );
                                Channels::send_request_to_channel(
                                    Arc::clone(&channels.debug_channel),
                                    DebugMessage {
                                        message: format!(
                                            "Failed to install model {}",
                                            model_install
                                        ),
                                        is_error: true,
                                    },
                                );
                            }
                        };
                    },
                    |result| Message::AsyncResult(result),
                )
            }

            Message::ModelChange(model) => {
                self.user_information.model = Some(model.clone());
                self.user_information.thinking_supported = None;
                // Reasoning support and accepted effort values vary by model. Do not carry an
                // effort setting across models while capability detection is still in flight.
                self.user_information.thinking_level = ThinkingLevel::Off;
                let ip = self.user_information.ip_address.clone();
                Task::perform(
                    async move {
                        let url = format!("http://{}:{}/api/show", ip.ip, ip.port);
                        let result = reqwest::Client::new()
                            .post(url)
                            .json(&serde_json::json!({ "model": model }))
                            .send()
                            .await
                            .ok();
                        let supported = match result {
                            Some(response) if response.status().is_success() => response
                                .json::<serde_json::Value>()
                                .await
                                .ok()
                                .and_then(|json| {
                                    json.get("capabilities")
                                        .and_then(|value| value.as_array())
                                        .map(|items| {
                                            items
                                                .iter()
                                                .any(|item| item.as_str() == Some("thinking"))
                                        })
                                }),
                            _ => None,
                        };
                        (model, supported)
                    },
                    |(model, supported)| Message::ModelCapabilityKnown(model, supported),
                )
            }

            Message::InstallationPrompt => {
                if webbrowser::open("https://ollama.com/download").is_ok() {
                    println!("Opened URL in default browser");
                } else {
                    eprintln!("Failed to open URL");
                }

                Task::none()
            }

            Message::ListPrompt => {
                if webbrowser::open("https://ollama.com/search").is_ok() {
                    println!("Opened URL in default browser");
                } else {
                    eprintln!("Failed to open URL");
                }

                Task::none()
            }

            Message::CopyPressed(input) => {
                if input.trim().is_empty() {
                    self.set_debug_message(DebugMessage {
                        message: "Nothing to copy yet.".to_string(),
                        is_error: true,
                    });

                    Task::none()
                } else {
                    self.last_copied_text = Some(input.clone());
                    self.last_copied_at = Some(Instant::now());

                    self.set_debug_message(DebugMessage {
                        message: "Copied to clipboard.".to_string(),
                        is_error: false,
                    });

                    clipboard::write::<Message>(input)
                }
            }

            Message::KeyPressed(keyboard::Key::Character(key), modifiers)
                if modifiers.control() && key.eq_ignore_ascii_case("v") =>
            {
                Task::perform(async { paste_chat_image() }, Message::ImageLoaded)
            }

            Message::KeyPressed(_, _) => Task::none(),

            Message::KeyReleased(_key) => Task::none(),

            Message::Prompt(prompt) => {
                if !self.is_processing {
                    self.is_processing = true;
                    self.prompt.prompt = String::new();

                    self.response.parsed_markdown = vec![];

                    if let Ok(mut response_text) = self.response.response_as_string.lock() {
                        *response_text = String::new();
                    }

                    self.response.parsed_markdown = markdown::parse("Waiting for bot...").collect();

                    return Self::prompt(self, prompt.clone());
                }

                Task::none()
            }

            Message::StopResponse => {
                if let Some(cancel) = &self.response_cancel {
                    cancel.store(true, Ordering::Relaxed);
                }
                self.is_processing = false;
                self.user_information
                    .chat_history
                    .lock()
                    .unwrap()
                    .bot_responding = false;
                self.set_debug_message(DebugMessage {
                    message: "Response stopped.".to_string(),
                    is_error: false,
                });
                Task::none()
            }

            Message::UpdatePrompt(prompt) => {
                self.prompt.prompt = prompt;
                Task::none()
            }

            Message::UpdateInstall(model) => {
                self.installing_model = model;
                Task::none()
            }
        }
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        Self::get_ui_information(self, &self.app_state.gui_state).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            iced::event::listen().filter_map(|event| match event {
                iced::event::Event::Window(iced::window::Event::FileDropped(path)) => {
                    Some(Message::DropImage(path))
                }
                iced::event::Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    physical_key,
                    modifiers,
                    ..
                }) if modifiers.command()
                    && (key == keyboard::Key::Character("v".into())
                        || physical_key == keyboard::key::Code::KeyV) =>
                {
                    Some(Message::PasteImage)
                }
                iced::event::Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) => Some(Message::KeyPressed(key, modifiers)),
                iced::event::Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) => {
                    Some(Message::KeyReleased(key))
                }
                _ => None,
            }),
            time::every(Duration::from_millis(TICK_MS)).map(|_| Message::Tick),
        ])
    }
}

impl Default for Program {
    fn default() -> Self {
        let mut json_error: String = String::new();

        let data_prompts: String = match fs::read_to_string("./config/defaultprompts.json") {
            Ok(dp) => dp,
            Err(_e) => {
                println!("An error occurred reading default prompts");
                json_error.push_str("| Failed to read: ./config/defaultprompts.json");
                "[]".to_string()
            }
        };

        let system_prompts_as_prompt: HashMap<String, String> =
            match serde_json::from_str(&data_prompts) {
                Ok(sp) => sp,
                Err(_e) => {
                    println!("An error occurred reading default prompts (bad format)");
                    json_error.push_str(
                        "| Failed to read: ./config/defaultprompts.json (bad formatting)",
                    );
                    HashMap::from([(String::new(), String::new())])
                }
            };

        let mut system_prompts: Vec<String> = Vec::new();
        system_prompts_as_prompt.iter().for_each(|prompt| {
            system_prompts.push(prompt.0.clone());
        });

        println!("Loaded system prompts:\n{:?} ", system_prompts);

        let settings = match fs::read_to_string("./config/settings.json") {
            Ok(dp) => dp,
            Err(_e) => {
                println!("An error occurred reading settings");
                json_error.push_str("| Failed to read: ./config/settings.json");
                "[]".to_string()
            }
        };

        println!("Loaded settings:\n{:?} ", settings);

        let settings_hmap: serde_json::Map<String, serde_json::Value> =
            match serde_json::from_str(&settings) {
                Ok(sp) => sp,
                Err(_e) => {
                    println!("An error occurred reading settings (bad format)");
                    json_error.push_str(
                    "| Failed to read: ./config/settings.json (bad formatting. reset to default)",
                );
                    serde_json::Map::new()
                }
            };

        let setting_bool = |key, default| {
            settings_hmap
                .get(key)
                .and_then(|v| v.as_bool())
                .unwrap_or(default)
        };
        let filtering = setting_bool("filtering", true);
        let logging = setting_bool("logging", false);
        let info_popup = setting_bool("info_popup", false);
        let dark_mode = setting_bool("dark_mode", false);
        let fast_streaming = setting_bool("fast_streaming", true);
        let legacy_configured_dir = settings_hmap
            .get("chat_storage_dir")
            .and_then(|value| value.as_str())
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from);
        let chat_storage_dir = fs::read_to_string(chat_location_settings_path())
            .ok()
            .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
            .and_then(|value| {
                value
                    .get("chat_storage_dir")
                    .and_then(|path| path.as_str())
                    .map(PathBuf::from)
            })
            .or(legacy_configured_dir)
            .unwrap_or_else(default_chat_storage_dir);

        let saved_chats: Vec<SavedChat> = fs::read_to_string(chat_storage_dir.join("chats.json"))
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .or_else(|| {
                fs::read_to_string("./output/chats.json")
                    .ok()
                    .and_then(|data| serde_json::from_str(&data).ok())
            })
            .unwrap_or_default();

        let history: History = History {
            began_logging: Local::now().to_rfc3339(),
            version: APP_VERSION.to_string(),
            filtering: filtering.clone(),
            logs: vec![],
        };

        match fs::write(
            "./output/history.json",
            serde_json::to_string_pretty(&history).unwrap(),
        ) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("An error writing to history.json");
                json_error.push_str("Unable to write to history.json");
            }
        };

        Self {
            batch_tokens: 3,
            fast_streaming,
            chat_menu_open: true,
            temporary_chat: false,
            current_chat_id: Self::new_chat_id(),
            saved_chats,
            chat_storage_dir,
            is_processing: false,
            response_cancel: None,
            current_tick: 0,
            installing_model: String::new(),

            debug_message: DebugMessage {
                message: json_error.clone(),
                is_error: json_error != String::new(),
            },
            debug_message_set_at: if json_error.is_empty() {
                None
            } else {
                Some(Instant::now())
            },
            chat_markdown_cache: Vec::new(),
            chat_model_name_cache: Vec::new(),
            active_response_model_name: None,
            last_copied_text: None,
            last_copied_at: None,
            pending_image: None,
            generated_images: Vec::new(),
            is_generating_image: false,
            expanded_thinking: HashSet::new(),

            system_prompt: SystemPrompt {
                system_prompts_as_hashmap: system_prompts_as_prompt,
                system_prompts_as_vec: Arc::new(Mutex::new(system_prompts)),
                system_prompt: Some(String::new()),
            },
            channels: Channels {
                markdown_channel_reciever: crossbeam_channel::unbounded().1,
                debug_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<DebugMessage>())),
                debounce_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<bool>())),
                logging_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<Log>())),
            },
            user_information: UserInformation {
                chat_history: Arc::new(Mutex::new(CurrentChat {
                    chats: vec![],
                    messages: vec![],
                    bot_responding: false,
                })),
                current_chat_history_enabled: true,
                model: None,
                thinking_level: ThinkingLevel::Off,
                thinking_supported: None,
                temperature: 7.0,
                text_size: 24.0,
                ip_address: HostLocation {
                    ip: "127.0.0.1".to_string(),
                    port: "11434".to_string(),
                },
            },
            response: Response {
                response_as_string: Arc::new(Mutex::new(String::new())),
                parsed_markdown: vec![],
            },
            prompt: Prompt {
                prompt_time_sent: Instant::now(),
                prompt: String::new(),
            },
            app_state: AppState {
                filtering,
                gui_state: if info_popup {
                    GUIState::InfoPopup
                } else {
                    GUIState::Main
                },
                dark_mode,
                logs: history,
                logging,
                ollama_state: Arc::new(Mutex::new("Offline".to_string())),
                bots_list: Arc::new(Mutex::new(vec![])),
            },
        }
    }
}

pub fn main() -> iced::Result {
    let icon = match image::ImageReader::open("./assets/icon.ico") {
        Ok(image_reader) => match image_reader.decode() {
            Ok(img) => {
                let rgba_image = img.into_rgba8();
                let (width, height) = rgba_image.dimensions();

                match iced::window::icon::from_rgba(rgba_image.into_raw(), width, height) {
                    Ok(icon) => Some(icon),
                    Err(e) => {
                        eprintln!("Failed to create icon: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to decode the image: {}", e);
                None
            }
        },
        Err(e) => {
            eprintln!("Failed to open the icon file: {}", e);
            None
        }
    };

    let window_settings = iced::window::Settings {
        icon,
        ..iced::window::Settings::default()
    };

    let settings = match fs::read_to_string("./config/settings.json") {
        Ok(dp) => dp,
        Err(_e) => {
            println!("An error occurred reading settings");
            "[]".to_string()
        }
    };

    let settings_hmap: HashMap<String, bool> = match serde_json::from_str(&settings) {
        Ok(sp) => sp,
        Err(_e) => {
            println!("An error occurred reading settings (bad format)");
            HashMap::from([("dark_mode".to_string(), false)])
        }
    };

    // The widget palette is intentionally dark; matching Iced's built-in theme
    // keeps markdown, menus, and overlays consistent with the custom surfaces.
    let _dark_mode = *settings_hmap.get("dark_mode").unwrap_or(&true);
    let mode = Theme::Dark;

    iced::application(|| Program::boot(), Program::update, Program::view)
        .subscription(Program::subscription)
        .theme(mode)
        .window_size(Size::new(700.0, 785.0))
        .window(window_settings)
        .run()
}
