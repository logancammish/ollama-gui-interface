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
use rustrict::{Censor, Type};
mod app;
mod gui;
mod web_search;

use crate::app::{
    AppState, Channels, ChatImage, Correspondence, CurrentChat, DebugMessage, History,
    HostLocation, Language, Log, Prompt, SavedChat, SystemPrompt, ThinkingLevel, UserInformation,
};
use crate::web_search::{
    BraveSearchProvider, ToolLoopRequest, WebSearchProviderKind, WebSearchSettings, WebSearchState,
    run_tool_loop,
};

/// Tick points:
/// Each tick occurs every TICK_MS; these constants decide what happens on each tick.
const VERSION_TICK: i32 = 2;
const MAX_TICK: i32 = 59;
const BOT_LIST_TICK: i32 = 3;
const TICK_MS: u64 = 200;
const DEFAULT_MAX_RESPONSE_TOKENS: u32 = 10_240;
const DEFAULT_CONTEXT_TOKENS: u32 = 20_480;
const MIN_RESPONSE_TOKENS: u32 = 512;
const MAX_RESPONSE_TOKENS: u32 = 65_536;
const MIN_CONTEXT_TOKENS: u32 = 4_096;
const MAX_CONTEXT_TOKENS: u32 = 262_144;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    ToggleWebSearch,
    ToggleMultipleWebSearches,
    ToggleChatWebSearch,
    WebSearchProviderChange(WebSearchProviderKind),
    WebSearchApiKeyChange(String),
    WebSearchResultLimitChange(f32),
    OpenSource(String),
    UrlOpened(Result<(), String>),
    NewChat,
    OpenChat(String),
    ToggleChatPin(String),
    DeleteChat(String),
    DeleteTemporaryChat(String),
    ToggleTemporaryChat,
    ChooseChatFolder,
    ChatFolderSelected(Option<PathBuf>),
    AsyncResult(()),
    PromptFinished(String),
    ListPrompt,
    ThinkingLevelChange(ThinkingLevel),
    ToggleImages,
    PickImage,
    DropImage(PathBuf),
    PasteImage,
    ImageLoaded(Result<ChatImage, String>),
    ImagesLoaded(Result<Vec<ChatImage>, String>),
    RemoveImage(usize),
    GenerateImage,
    ImageGenerated(Result<String, String>),
    CopyImage(String),
    ModelCapabilitiesKnown(String, Option<(bool, bool, bool)>),
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
    UpdateMaxResponseTokens(f32),
    UpdateContextTokens(f32),
    LanguageChange(Language),
    ToggleInfoPopup,
    ToggleChatHistory,
    ToggleFiltering,
    ToggleDarkMode,
    WipeChatHistory,
    ToggleAdvancedSettings,
    ChangeIp(String),
    ChangePort(String),
}

struct ActivePrompt {
    chat_history: Arc<Mutex<CurrentChat>>,
    response_as_string: Arc<Mutex<String>>,
    parsed_markdown: Vec<markdown::Item>,
    markdown_receiver: crossbeam_channel::Receiver<Vec<markdown::Item>>,
    web_search_state: WebSearchState,
    web_search_state_receiver: crossbeam_channel::Receiver<WebSearchState>,
    cancel: Arc<AtomicBool>,
    model_name: String,
    started_at: Instant,
    response_start_index: usize,
    had_image: bool,
    web_search_enabled: bool,
    temporary: bool,
}

struct TemporaryChatSession {
    chat_history: Arc<Mutex<CurrentChat>>,
    web_search_enabled: bool,
}

struct VisionResponse {
    markdown: Vec<markdown::Item>,
}

struct Program {
    active_prompts: HashMap<String, ActivePrompt>,
    temporary_chats: HashMap<String, TemporaryChatSession>,
    chat_notices: HashMap<String, (DebugMessage, Instant)>,
    chat_notice_sender: crossbeam_channel::Sender<(String, DebugMessage)>,
    chat_notice_receiver: crossbeam_channel::Receiver<(String, DebugMessage)>,
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

    /// Used for brief copy feedback animations/buttons.
    last_copied_text: Option<String>,
    last_copied_at: Option<Instant>,

    pending_images: Vec<ChatImage>,
    generated_images: Vec<String>,
    is_generating_image: bool,
    vision_responses: HashMap<String, VisionResponse>,

    /// Message indexes whose reasoning disclosure is open. Reasoning is hidden by default.
    expanded_thinking: HashSet<usize>,

    system_prompt: SystemPrompt,
    app_state: AppState,
    channels: Channels,
    user_information: UserInformation,
    prompt: Prompt,
    batch_tokens: i32,
    fast_streaming: bool,
    chat_menu_open: bool,
    temporary_chat: bool,
    web_search_settings: WebSearchSettings,
    web_search_for_chat: bool,
    current_chat_id: String,
    saved_chats: Vec<SavedChat>,
    chat_storage_dir: PathBuf,
}

fn default_chat_storage_dir() -> PathBuf {
    app_data_dir().join("chats")
}

#[cfg(test)]
fn app_data_dir() -> PathBuf {
    std::env::temp_dir().join(format!("ollama-gui-test-data-{}", std::process::id()))
}

#[cfg(not(test))]
fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(base).join("Ollama GUI");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Library/Application Support/Ollama GUI");
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(base) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(base).join("ollama-gui");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local/share/ollama-gui");
        }
    }
    PathBuf::from("output")
}

fn chat_location_settings_path() -> PathBuf {
    app_data_dir().join("chat-location.json")
}

fn user_settings_path() -> PathBuf {
    app_data_dir().join("settings.json")
}

fn history_path() -> PathBuf {
    app_data_dir().join("history.json")
}

fn generated_images_dir() -> PathBuf {
    app_data_dir().join("generated")
}

fn load_generated_images() -> Vec<String> {
    let mut images = fs::read_dir(generated_images_dir())
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("png" | "jpg" | "jpeg" | "webp")
            )
        })
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    images.sort();
    images
}

fn model_capabilities(json: &serde_json::Value) -> Option<(bool, bool, bool)> {
    let capabilities = json.get("capabilities")?.as_array()?;
    let has = |name| {
        capabilities
            .iter()
            .any(|capability| capability.as_str() == Some(name))
    };
    Some((has("thinking"), has("vision"), has("image")))
}

async fn generate_image_via_ollama(
    host: String,
    model: String,
    prompt: String,
) -> Result<String, String> {
    let response = reqwest::Client::new()
        .post(format!("{host}/api/generate"))
        .json(&serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "width": 1024,
            "height": 1024
        }))
        .send()
        .await
        .map_err(|error| format!("Could not reach Ollama: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        return Err(format!(
            "Ollama image generation failed ({status}): {}",
            detail.trim()
        ));
    }

    let response_text = response
        .text()
        .await
        .map_err(|error| format!("Could not read Ollama's image response: {error}"))?;
    let body = serde_json::from_str::<serde_json::Value>(&response_text)
        .ok()
        .or_else(|| {
            response_text
                .lines()
                .rev()
                .filter(|line| !line.trim().is_empty())
                .find_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        })
        .ok_or_else(|| "Ollama returned an invalid image response.".to_string())?;
    let encoded = body
        .get("image")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            "The selected model did not return an image. Choose a model with the `image` capability."
                .to_string()
        })?;
    let encoded = encoded.rsplit_once(',').map_or(encoded, |(_, data)| data);
    let bytes = BASE64
        .decode(encoded)
        .map_err(|error| format!("Could not decode Ollama's generated image: {error}"))?;
    let decoded = image::load_from_memory(&bytes)
        .map_err(|error| format!("Ollama returned unsupported image data: {error}"))?;

    let output_directory = generated_images_dir();
    fs::create_dir_all(&output_directory)
        .map_err(|error| format!("Could not create image output folder: {error}"))?;
    let output_path = output_directory.join(format!(
        "ollama-image-{}.png",
        chrono::Utc::now().timestamp_millis()
    ));
    decoded
        .save(&output_path)
        .map_err(|error| format!("Could not save generated image: {error}"))?;
    Ok(output_path.to_string_lossy().to_string())
}

/// Installed assets are read-only. Resolve them beside the executable so
/// shortcuts and command-line launches work regardless of their working folder.
fn resource_path(relative: &str) -> PathBuf {
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        let installed = directory.join(relative);
        if installed.exists() {
            return installed;
        }
    }
    PathBuf::from(relative)
}

fn load_settings_text() -> Option<String> {
    fs::read_to_string(user_settings_path())
        .ok()
        .or_else(|| fs::read_to_string(resource_path("config/settings.json")).ok())
}

fn censor_text(input: &str) -> String {
    Censor::from_str(input)
        .with_censor_replacement('#')
        .with_censor_first_character_threshold(Type::INAPPROPRIATE)
        .censor()
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

fn code_fence_language_alias(language: &str) -> Option<&'static str> {
    match language.to_ascii_lowercase().as_str() {
        "csharp" | "c-sharp" => Some("cs"),
        "cplusplus" | "cxx" => Some("cpp"),
        "golang" => Some("go"),
        "javascriptreact" => Some("jsx"),
        "typescriptreact" => Some("tsx"),
        "objectivec" | "objective-c" => Some("m"),
        "visualbasic" | "vbnet" | "vb.net" => Some("vb"),
        _ => None,
    }
}

/// Syntect accepts a language name or file extension, while models often emit
/// popular aliases such as `csharp`. Normalize only opening fence info strings;
/// code inside a fenced block is left byte-for-byte unchanged.
fn normalize_code_fence_languages(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut open_fence: Option<(u8, usize)> = None;

    for segment in input.split_inclusive('\n') {
        let body_length = segment.trim_end_matches(['\r', '\n']).len();
        let body = &segment[..body_length];
        let line_ending = &segment[body_length..];
        let trimmed = body.trim_start();
        let indentation_length = body.len() - trimmed.len();
        let marker = trimmed.as_bytes().first().copied();
        let marker_length = marker.map_or(0, |marker| {
            trimmed
                .as_bytes()
                .iter()
                .take_while(|character| **character == marker)
                .count()
        });

        if let Some((active_marker, active_length)) = open_fence {
            let is_closing = marker == Some(active_marker)
                && marker_length >= active_length
                && trimmed[marker_length..].trim().is_empty();
            if is_closing {
                open_fence = None;
            }
            output.push_str(segment);
            continue;
        }

        if !matches!(marker, Some(b'`' | b'~')) || marker_length < 3 {
            output.push_str(segment);
            continue;
        }

        open_fence = Some((marker.unwrap(), marker_length));
        let info = &trimmed[marker_length..];
        let leading_space = info.len() - info.trim_start().len();
        let token_start = indentation_length + marker_length + leading_space;
        let token_length = info[leading_space..]
            .find(char::is_whitespace)
            .unwrap_or(info.len() - leading_space);
        let token_end = token_start + token_length;
        let token = &body[token_start..token_end];

        if let Some(alias) = code_fence_language_alias(token) {
            output.push_str(&body[..token_start]);
            output.push_str(alias);
            output.push_str(&body[token_end..]);
            output.push_str(line_ending);
        } else {
            output.push_str(segment);
        }
    }

    output
}

fn parse_markdown_items(input: &str) -> Vec<markdown::Item> {
    let normalized = normalize_code_fence_languages(input);
    markdown::parse(&normalized).collect()
}

fn decode_generation_line(
    input: &str,
) -> Result<(GenerationResponse, Option<String>), serde_json::Error> {
    let value = serde_json::from_str::<serde_json::Value>(input)?;
    let done_reason = value
        .get("done_reason")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let response = serde_json::from_value(value)?;
    Ok((response, done_reason))
}

fn disabled_web_tool_message(input: &str) -> Option<&'static str> {
    let trimmed = input.trim();
    let looks_like_tool_call = (trimmed.starts_with('{') || trimmed.starts_with("```json"))
        && (trimmed.contains("\"web_search\"") || trimmed.contains("\"fetch_webpage\""));
    looks_like_tool_call.then_some(
        "Web search is disabled. Enable it in Settings or with the Web toggle for this chat.",
    )
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

fn open_url(url: String) -> Task<Message> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                webbrowser::open(&url).map_err(|error| format!("Could not open link: {error}"))
            })
            .await
            .map_err(|error| format!("Could not open link: {error}"))?
        },
        Message::UrlOpened,
    )
}

fn send_chat_notice(
    sender: &crossbeam_channel::Sender<(String, DebugMessage)>,
    chat_id: &str,
    message: DebugMessage,
) {
    let _ = sender.send((chat_id.to_string(), message));
}

fn append_failed_response(
    chat_history: &Arc<Mutex<CurrentChat>>,
    model: Option<String>,
    message: String,
) {
    chat_history
        .lock()
        .unwrap()
        .push_message(Correspondence::Bot {
            text: message,
            model,
            thinking_seconds: None,
            sources: Vec::new(),
            web_search_used: false,
        });
}

async fn wait_until_cancelled(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

impl Program {
    fn new_chat_id() -> String {
        format!(
            "chat-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    }

    fn current_active_prompt(&self) -> Option<&ActivePrompt> {
        self.active_prompts.get(&self.current_chat_id)
    }

    fn current_chat_is_processing(&self) -> bool {
        self.current_active_prompt().is_some()
    }

    fn prompt_progress(&self) -> f32 {
        let phase = (self.current_tick.rem_euclid(20) as f32) / 10.0;
        let wave = if phase <= 1.0 { phase } else { 2.0 - phase };
        0.12 + wave * 0.76
    }

    fn clear_open_chat(&mut self) {
        self.user_information.chat_history = Arc::new(Mutex::new(CurrentChat {
            chats: vec![],
            messages: vec![],
            bot_responding: false,
        }));
        self.chat_markdown_cache.clear();
        self.chat_model_name_cache.clear();
        self.last_copied_text = None;
        self.last_copied_at = None;
        self.expanded_thinking.clear();
    }

    fn save_open_chat(&mut self) {
        if self.temporary_chat {
            return;
        }
        let chat = self.user_information.chat_history.lock().unwrap().clone();
        self.save_chat_snapshot(
            self.current_chat_id.clone(),
            &chat,
            self.web_search_for_chat,
        );
    }

    fn save_chat_snapshot(&mut self, id: String, chat: &CurrentChat, web_search_enabled: bool) {
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
        let mut saved = SavedChat::from_current(id, title, chat, web_search_enabled);
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
        self.persist_saved_chats();
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

    fn persist_current_chat_web_search_setting(&mut self) {
        if self.temporary_chat {
            if let Some(session) = self.temporary_chats.get_mut(&self.current_chat_id) {
                session.web_search_enabled = self.web_search_for_chat;
            }
            return;
        }

        if let Some(chat) = self
            .saved_chats
            .iter_mut()
            .find(|chat| chat.id == self.current_chat_id)
        {
            chat.web_search_enabled = Some(self.web_search_for_chat);
            self.persist_saved_chats();
        }
    }

    fn persist_boolean_setting(&mut self, key: &str, value: bool) {
        self.persist_setting_value(key, serde_json::Value::Bool(value));
    }

    fn persist_web_search_settings(&mut self) {
        match serde_json::to_value(&self.web_search_settings) {
            Ok(value) => self.persist_setting_value("web_search", value),
            Err(error) => self.set_debug_message(DebugMessage {
                message: format!("Could not save web-search settings: {error}"),
                is_error: true,
            }),
        }
    }

    fn persist_setting_value(&mut self, key: &str, value: serde_json::Value) {
        let mut settings: serde_json::Map<String, serde_json::Value> = load_settings_text()
            .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        settings.insert(key.to_string(), value);
        let settings_path = user_settings_path();
        let result = settings_path
            .parent()
            .map(fs::create_dir_all)
            .transpose()
            .and_then(|_| {
                fs::write(
                    settings_path,
                    serde_json::to_string_pretty(&settings).unwrap_or_else(|_| "{}".into()),
                )
            });
        if let Err(error) = result {
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
        if let Some(set_at) = self.debug_message_set_at
            && set_at.elapsed() >= Duration::from_secs(15)
        {
            self.debug_message.message.clear();
            self.debug_message.is_error = false;
            self.debug_message_set_at = None;
        }
        let current_chat_id = &self.current_chat_id;
        self.chat_notices.retain(|chat_id, (_, set_at)| {
            chat_id != current_chat_id || set_at.elapsed() < Duration::from_secs(15)
        });
    }

    fn current_debug_message(&self) -> &DebugMessage {
        self.chat_notices
            .get(&self.current_chat_id)
            .map(|(message, _)| message)
            .unwrap_or(&self.debug_message)
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

                Correspondence::Bot { text, model, .. } => {
                    let (_, visible_text) = split_thinking_text(text);
                    if let Some(cached) = old_markdown_cache.get(index) {
                        new_markdown_cache.push(cached.clone());
                    } else {
                        new_markdown_cache.push(parse_markdown_items(&visible_text));
                    }

                    let model_name = old_model_name_cache
                        .get(index)
                        .cloned()
                        .flatten()
                        .or_else(|| model.clone())
                        .or_else(|| Some("Unknown model".to_string()));

                    new_model_name_cache.push(model_name);
                }
            }
        }

        self.chat_markdown_cache = new_markdown_cache;
        self.chat_model_name_cache = new_model_name_cache;
    }

    fn clear_copy_feedback_if_old(&mut self) {
        if let Some(copied_at) = self.last_copied_at
            && copied_at.elapsed() >= Duration::from_millis(1400)
        {
            self.last_copied_text = None;
            self.last_copied_at = None;
        }
    }

    fn apply_response_metadata(
        chat: &mut CurrentChat,
        response_start_index: usize,
        model_name: &str,
        elapsed_seconds: u64,
    ) {
        if let Some(Correspondence::Bot {
            model: stored_model,
            thinking_seconds,
            ..
        }) = chat
            .messages
            .iter_mut()
            .skip(response_start_index)
            .rev()
            .find(|message| matches!(message, Correspondence::Bot { .. }))
        {
            if stored_model.is_none() {
                *stored_model = Some(model_name.to_string());
            }
            if thinking_seconds.is_none() {
                *thinking_seconds = Some(elapsed_seconds);
            }
        }
    }

    fn finalize_response_metadata(job: &ActivePrompt) {
        let elapsed_seconds = job.started_at.elapsed().as_secs().max(1);
        if let Ok(mut chat) = job.chat_history.lock() {
            Self::apply_response_metadata(
                &mut chat,
                job.response_start_index,
                &job.model_name,
                elapsed_seconds,
            );
        }
    }

    fn finish_prompt(&mut self, chat_id: &str) {
        let Some(job) = self.active_prompts.remove(chat_id) else {
            return;
        };
        Self::finalize_response_metadata(&job);

        let completed_chat = job.chat_history.lock().unwrap().clone();
        if job.temporary {
            self.temporary_chats.insert(
                chat_id.to_string(),
                TemporaryChatSession {
                    chat_history: Arc::clone(&job.chat_history),
                    web_search_enabled: job.web_search_enabled,
                },
            );
        } else {
            self.save_chat_snapshot(chat_id.to_string(), &completed_chat, job.web_search_enabled);
        }

        if job.had_image
            && let Some(response) = completed_chat
                .messages
                .iter()
                .skip(job.response_start_index)
                .rev()
                .find_map(|message| match message {
                    Correspondence::Bot { text, .. } => Some(text.as_str()),
                    Correspondence::User { .. } => None,
                })
        {
            let (_, visible) = split_thinking_text(response);
            self.vision_responses.insert(
                chat_id.to_string(),
                VisionResponse {
                    markdown: parse_markdown_items(&visible),
                },
            );
        }

        if self.current_chat_id == chat_id {
            self.user_information.chat_history = Arc::clone(&job.chat_history);
            self.expanded_thinking.remove(&usize::MAX);
            self.refresh_chat_markdown_cache();
        }
    }

    fn prompt(&mut self, mut prompt: String) -> Task<Message> {
        if self.user_information.model.is_none() {
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

        if self.app_state.filtering {
            prompt = censor_text(&prompt);
        }

        let model_name = self.user_information.model.clone().unwrap();
        let started_at = Instant::now();

        let cancel = Arc::new(AtomicBool::new(false));

        // Keep only the newest full Markdown snapshot. A long response can
        // otherwise queue many increasingly large copies while its chat is in
        // the background.
        let (markdown_sender, markdown_receiver) = crossbeam_channel::bounded(1);
        let markdown_receiver_for_renderer = markdown_receiver.clone();

        // Backpressure bounds token memory if highlighting a large response is
        // temporarily slower than Ollama's stream.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<GenerationResponse>(64);
        let batch_tokens = self.batch_tokens;
        let fast_streaming = self.fast_streaming;
        let response_string = Arc::new(Mutex::new(String::new()));
        let response_string_for_renderer = Arc::clone(&response_string);

        std::thread::spawn(move || {
            fn render(
                buffer: &str,
                markdown_sender: &crossbeam_channel::Sender<Vec<markdown::Item>>,
                markdown_receiver: &crossbeam_channel::Receiver<Vec<markdown::Item>>,
            ) {
                let (_, visible) = split_thinking_text(buffer);
                let md = parse_markdown_items(&visible);
                while markdown_receiver.try_recv().is_ok() {}
                // A disconnected receiver means the completed job has already
                // been reconciled into its chat history.
                let _ = markdown_sender.try_send(md);
            }

            let mut buffer = String::new();
            let mut last_render_time = Instant::now();
            let mut total_tokens = 0;

            while let Some(token) = rx.blocking_recv() {
                buffer.push_str(&token.response);

                if let Ok(mut current_response) = response_string_for_renderer.lock() {
                    *current_response = buffer.clone();
                }

                total_tokens += 1;

                if fast_streaming {
                    // "Fast" remains visually immediate without reparsing the
                    // entire growing answer more than roughly once per frame.
                    if last_render_time.elapsed().as_millis() < 33 {
                        continue;
                    }
                } else if total_tokens < batch_tokens
                    && last_render_time.elapsed().as_millis() < 250
                {
                    continue;
                }

                total_tokens = 0;
                last_render_time = Instant::now();

                render(&buffer, &markdown_sender, &markdown_receiver_for_renderer);
            }

            if !buffer.is_empty() {
                if let Ok(mut current_response) = response_string_for_renderer.lock() {
                    *current_response = buffer.clone();
                }

                render(&buffer, &markdown_sender, &markdown_receiver_for_renderer);
            }
        });

        let system_prompt: Option<String> = SystemPrompt::get_current(self);

        if system_prompt.is_none() {
            Channels::send_request_to_channel(
                Arc::clone(&self.channels.debug_channel),
                DebugMessage {
                    message: "Could not get system prompt, is it selected?".to_string(),
                    is_error: true,
                },
            );
            return Task::none();
        }

        // Clone the attachment into the request/chat first. The composer owns its
        // copy until the submission has been accepted, avoiding a transient blank
        // preview while the async request is being prepared.
        let attached_images = self.pending_images.clone();
        let had_image = !attached_images.is_empty();
        let logging = self.app_state.logging;
        let filtering = self.app_state.filtering;
        let user_info = self.user_information.clone();
        let channels = self.channels.clone();
        let web_search_enabled = self.web_search_for_chat;
        let mut web_search_settings = self.web_search_settings.clone();
        web_search_settings.enabled = web_search_enabled;
        let (web_search_state_sender, web_search_state_receiver) = crossbeam_channel::unbounded();
        let chat_id = self.current_chat_id.clone();
        let completion_chat_id = chat_id.clone();
        let notice_chat_id = chat_id.clone();
        let chat_notice_sender = self.chat_notice_sender.clone();
        self.chat_notices.remove(&chat_id);

        let response_start_index = {
            let mut chat = user_info.chat_history.lock().unwrap();
            chat.push_message(Correspondence::User {
                text: prompt.clone(),
                images: attached_images.clone(),
            });
            chat.messages.len()
        };

        self.refresh_chat_markdown_cache();
        self.pending_images.clear();
        user_info.chat_history.lock().unwrap().bot_responding = true;
        if self.temporary_chat {
            self.temporary_chats.remove(&chat_id);
        }
        self.active_prompts.insert(
            chat_id,
            ActivePrompt {
                chat_history: Arc::clone(&user_info.chat_history),
                response_as_string: response_string,
                parsed_markdown: parse_markdown_items("Waiting for bot..."),
                markdown_receiver,
                web_search_state: WebSearchState::Idle,
                web_search_state_receiver,
                cancel: Arc::clone(&cancel),
                model_name,
                started_at,
                response_start_index,
                had_image,
                web_search_enabled,
                temporary: self.temporary_chat,
            },
        );

        Task::perform(
            async move {
                println!("Received prompt: {}", prompt.clone());

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

                if web_search_enabled {
                    let provider = match BraveSearchProvider::new(&web_search_settings) {
                        Ok(provider) => Arc::new(provider),
                        Err(error) => {
                            let _ = web_search_state_sender.send(WebSearchState::Failed {
                                message: error.user_message().to_string(),
                            });
                            send_chat_notice(
                                &chat_notice_sender,
                                &notice_chat_id,
                                DebugMessage {
                                    message: error.user_message().to_string(),
                                    is_error: true,
                                },
                            );
                            user_info.chat_history.lock().unwrap().push_message(
                                Correspondence::Bot {
                                    text: format!("Web search could not start: {error}"),
                                    model: user_info.model.clone(),
                                    thinking_seconds: None,
                                    sources: Vec::new(),
                                    web_search_used: true,
                                },
                            );
                            user_info.chat_history.lock().unwrap().bot_responding = false;
                            return;
                        }
                    };
                    let result = run_tool_loop(ToolLoopRequest {
                        ollama_url: format!("http://{}:{}/api/chat", ip.ip, ip.port),
                        model: user_info.model.clone().unwrap(),
                        prompt: to_send_prompt,
                        system_prompt: system_prompt.clone(),
                        temperature: user_info.temperature / 10.0,
                        context_tokens: user_info.context_tokens,
                        max_response_tokens: user_info.max_response_tokens,
                        images: attached_images
                            .iter()
                            .map(|image| BASE64.encode(&image.bytes))
                            .collect(),
                        thinking: user_info.thinking_level.api_value(),
                        settings: web_search_settings.clone(),
                        provider,
                        state_sender: web_search_state_sender.clone(),
                        cancel: Arc::clone(&cancel),
                    })
                    .await;

                    match result {
                        Ok(result) => {
                            let answer = if filtering {
                                censor_text(&result.answer)
                            } else {
                                result.answer
                            };
                            let _ = tx
                                .send(GenerationResponse {
                                    model: user_info.model.clone().unwrap(),
                                    created_at: Local::now().to_rfc3339(),
                                    response: answer.clone(),
                                    done: true,
                                    context: None,
                                    total_duration: None,
                                    load_duration: None,
                                    prompt_eval_count: None,
                                    prompt_eval_duration: None,
                                    eval_count: None,
                                    eval_duration: None,
                                    thinking: None,
                                    logprobs: None,
                                })
                                .await;
                            if logging {
                                Channels::send_request_to_channel(
                                    Arc::clone(&channels.logging_channel),
                                    Log::create_with_current_time(
                                        filtering,
                                        user_info.model.clone(),
                                        vec![answer.clone()],
                                        Some(system_prompt),
                                        prompt.clone(),
                                    ),
                                );
                            }
                            if user_info.current_chat_history_enabled {
                                user_info
                                    .chat_history
                                    .lock()
                                    .unwrap()
                                    .generate_and_push(prompt.clone(), answer.clone());
                            }
                            user_info.chat_history.lock().unwrap().push_message(
                                Correspondence::Bot {
                                    text: answer,
                                    model: user_info.model.clone(),
                                    thinking_seconds: None,
                                    sources: result.sources,
                                    web_search_used: true,
                                },
                            );
                        }
                        Err(crate::web_search::WebSearchError::Cancelled) => {}
                        Err(error) => {
                            let message = error.user_message().to_string();
                            eprintln!(
                                "Web-search failure: {}",
                                error.diagnostic(web_search_settings.api_key.as_deref())
                            );
                            let _ = web_search_state_sender.send(WebSearchState::Failed {
                                message: message.clone(),
                            });
                            send_chat_notice(
                                &chat_notice_sender,
                                &notice_chat_id,
                                DebugMessage {
                                    message: message.clone(),
                                    is_error: true,
                                },
                            );
                            user_info.chat_history.lock().unwrap().push_message(
                                Correspondence::Bot {
                                    text: format!("Web search failed: {message}"),
                                    model: user_info.model.clone(),
                                    thinking_seconds: None,
                                    sources: Vec::new(),
                                    web_search_used: true,
                                },
                            );
                        }
                    }
                    user_info.chat_history.lock().unwrap().bot_responding = false;
                    return;
                }

                let request: GenerationRequest<'_> =
                    GenerationRequest::new(user_info.model.clone().unwrap(), to_send_prompt)
                        .options(
                            ModelOptions::default()
                                .temperature(user_info.temperature / 10.0)
                                .num_predict(user_info.max_response_tokens as i32)
                                .num_ctx(user_info.context_tokens as u64),
                        )
                        .system(system_prompt.clone());

                println!("System prompt: {}", system_prompt.clone());

                let mut request_body = match serde_json::to_value(request) {
                    Ok(body) => body,
                    Err(e) => {
                        eprintln!("Error serializing request: {}", e);
                        let message = "Could not prepare the Ollama request".to_string();
                        send_chat_notice(
                            &chat_notice_sender,
                            &notice_chat_id,
                            DebugMessage {
                                message: message.clone(),
                                is_error: true,
                            },
                        );
                        append_failed_response(
                            &user_info.chat_history,
                            user_info.model.clone(),
                            message,
                        );
                        user_info.chat_history.lock().unwrap().bot_responding = false;
                        return;
                    }
                };

                request_body["stream"] = serde_json::Value::Bool(true);
                request_body["think"] = user_info.thinking_level.api_value();
                if !attached_images.is_empty() {
                    request_body["images"] = serde_json::json!(
                        attached_images
                            .iter()
                            .map(|image| BASE64.encode(&image.bytes))
                            .collect::<Vec<_>>()
                    );
                }

                let url = format!("http://{}:{}/api/generate", ip.ip, ip.port);
                let request = reqwest::Client::new().post(url).json(&request_body).send();
                let response = tokio::select! {
                    response = request => Some(response),
                    () = wait_until_cancelled(&cancel) => None,
                };
                let Some(response) = response else {
                    user_info.chat_history.lock().unwrap().bot_responding = false;
                    return;
                };
                let mut response = match response {
                    Ok(response) if response.status().is_success() => response,
                    Ok(response) => {
                        let status = response.status();
                        let response_text = response.text();
                        let detail = tokio::select! {
                            detail = response_text => detail.unwrap_or_default(),
                            () = wait_until_cancelled(&cancel) => {
                                user_info.chat_history.lock().unwrap().bot_responding = false;
                                return;
                            }
                        };
                        let message = format!("Ollama rejected the request ({status}): {detail}");
                        send_chat_notice(
                            &chat_notice_sender,
                            &notice_chat_id,
                            DebugMessage {
                                message: message.clone(),
                                is_error: true,
                            },
                        );
                        append_failed_response(
                            &user_info.chat_history,
                            user_info.model.clone(),
                            message,
                        );
                        user_info.chat_history.lock().unwrap().bot_responding = false;
                        return;
                    }
                    Err(error) => {
                        let message = format!("Could not reach Ollama: {error}");
                        send_chat_notice(
                            &chat_notice_sender,
                            &notice_chat_id,
                            DebugMessage {
                                message: message.clone(),
                                is_error: true,
                            },
                        );
                        append_failed_response(
                            &user_info.chat_history,
                            user_info.model.clone(),
                            message,
                        );
                        user_info.chat_history.lock().unwrap().bot_responding = false;
                        return;
                    }
                };

                let mut final_response: Vec<String> = vec![];
                let mut stream_buffer = String::new();

                'response_stream: while !cancel.load(Ordering::Relaxed) {
                    let chunk_result = tokio::select! {
                        chunk = response.chunk() => chunk,
                        _ = tokio::time::sleep(Duration::from_millis(50)) => continue,
                    };
                    let Ok(Some(chunk)) = chunk_result else {
                        break;
                    };
                    stream_buffer.push_str(&String::from_utf8_lossy(&chunk));
                    while let Some(newline) = stream_buffer.find('\n') {
                        let line = stream_buffer[..newline].trim().to_string();
                        stream_buffer.drain(..=newline);
                        if line.is_empty() {
                            continue;
                        }
                        match decode_generation_line(&line) {
                            Ok((mut token, done_reason)) => {
                                if token.done
                                    && (done_reason.as_deref() == Some("length")
                                        || token.eval_count.unwrap_or_default()
                                            >= user_info.max_response_tokens as u64)
                                {
                                    send_chat_notice(
                                        &chat_notice_sender,
                                        &notice_chat_id,
                                        DebugMessage {
                                            message: format!(
                                                "The model reached the generation limit ({} tokens). Increase Maximum response or Context window in Settings.",
                                                user_info.max_response_tokens
                                            ),
                                            is_error: true,
                                        },
                                    );
                                }
                                if !filtering {
                                    print!("{}", token.response);
                                }

                                // Ollama may return reasoning in its dedicated `thinking`
                                // field, while some models emit literal <think> tags.
                                // Normalize both forms so the renderer can disclose them alike.
                                if let Some(thinking) = token.thinking.take()
                                    && !thinking.is_empty()
                                {
                                    token.response =
                                        format!("<think>{thinking}</think>{}", token.response);
                                }

                                final_response.push(token.response.clone());

                                // Filtering must see the complete response: Ollama can
                                // split a profane word across arbitrary stream tokens.
                                if !filtering {
                                    let sent = tokio::select! {
                                        result = tx.send(token) => result.is_ok(),
                                        () = wait_until_cancelled(&cancel) => false,
                                    };
                                    if !sent {
                                        break 'response_stream;
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error decoding Ollama response: {}", e);
                                send_chat_notice(
                                    &chat_notice_sender,
                                    &notice_chat_id,
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
                // NDJSON normally ends with a newline, but accepting a final
                // unterminated object avoids dropping the last token from
                // proxies or older Ollama builds.
                let trailing_line = stream_buffer.trim();
                if !was_cancelled && !trailing_line.is_empty() {
                    match decode_generation_line(trailing_line) {
                        Ok((mut token, done_reason)) => {
                            if token.done
                                && (done_reason.as_deref() == Some("length")
                                    || token.eval_count.unwrap_or_default()
                                        >= user_info.max_response_tokens as u64)
                            {
                                send_chat_notice(
                                    &chat_notice_sender,
                                    &notice_chat_id,
                                    DebugMessage {
                                        message: format!(
                                            "The model reached the generation limit ({} tokens). Increase Maximum response or Context window in Settings.",
                                            user_info.max_response_tokens
                                        ),
                                        is_error: true,
                                    },
                                );
                            }
                            if let Some(thinking) = token.thinking.take()
                                && !thinking.is_empty()
                            {
                                token.response =
                                    format!("<think>{thinking}</think>{}", token.response);
                            }
                            final_response.push(token.response.clone());
                            if !filtering {
                                let _ = tx.send(token).await;
                            }
                        }
                        Err(error) => {
                            eprintln!("Error decoding final Ollama response: {error}");
                            send_chat_notice(
                                &chat_notice_sender,
                                &notice_chat_id,
                                DebugMessage {
                                    message: "Ollama returned an invalid final streaming response"
                                        .to_string(),
                                    is_error: true,
                                },
                            );
                        }
                    }
                }

                if !was_cancelled && final_response.concat().trim().is_empty() {
                    let message =
                        "Ollama ended the response without returning content.".to_string();
                    send_chat_notice(
                        &chat_notice_sender,
                        &notice_chat_id,
                        DebugMessage {
                            message: message.clone(),
                            is_error: true,
                        },
                    );
                    append_failed_response(
                        &user_info.chat_history,
                        user_info.model.clone(),
                        message,
                    );
                    user_info.chat_history.lock().unwrap().bot_responding = false;
                    return;
                }

                if filtering && !final_response.is_empty() {
                    let filtered = censor_text(&final_response.join(""));
                    final_response = vec![filtered.clone()];
                    let _ = tx
                        .send(GenerationResponse {
                            model: user_info.model.clone().unwrap_or_default(),
                            created_at: Local::now().to_rfc3339(),
                            response: filtered,
                            done: true,
                            context: None,
                            total_duration: None,
                            load_duration: None,
                            prompt_eval_count: None,
                            prompt_eval_duration: None,
                            eval_count: None,
                            eval_duration: None,
                            thinking: None,
                            logprobs: None,
                        })
                        .await;
                }

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
                    let partial_response = disabled_web_tool_message(&partial_response)
                        .unwrap_or(&partial_response)
                        .to_string();
                    user_info
                        .chat_history
                        .lock()
                        .unwrap()
                        .push_message(Correspondence::Bot {
                            text: partial_response,
                            model: None,
                            thinking_seconds: None,
                            sources: Vec::new(),
                            web_search_used: false,
                        });
                }

                user_info.chat_history.lock().unwrap().bot_responding = false;
            },
            move |_| Message::PromptFinished(completion_chat_id.clone()),
        )
    }

    fn boot() -> (Program, Task<Message>) {
        (Program::default(), Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AsyncResult(_result) => Task::none(),

            Message::PromptFinished(chat_id) => {
                self.finish_prompt(&chat_id);
                Task::none()
            }

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
                    let paths = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
                        .pick_files()
                        .ok_or_else(|| "No image selected.".to_string())?;
                    paths
                        .iter()
                        .map(|path| load_chat_image(path))
                        .collect::<Result<Vec<_>, _>>()
                },
                Message::ImagesLoaded,
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
                        self.pending_images.push(image);
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

            Message::ImagesLoaded(result) => {
                match result {
                    Ok(images) => {
                        let count = images.len();
                        self.pending_images.extend(images);
                        self.set_debug_message(DebugMessage {
                            message: format!("Attached {count} images."),
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

            Message::RemoveImage(index) => {
                if index < self.pending_images.len() {
                    self.pending_images.remove(index);
                }
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
                match self.user_information.image_generation_supported {
                    Some(true) => {}
                    Some(false) => {
                        self.set_debug_message(DebugMessage {
                            message:
                                "Choose a model with Ollama's experimental `image` capability."
                                    .to_string(),
                            is_error: true,
                        });
                        return Task::none();
                    }
                    None => {
                        self.set_debug_message(DebugMessage {
                            message: "Waiting for Ollama to report this model's capabilities."
                                .to_string(),
                            is_error: true,
                        });
                        return Task::none();
                    }
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
                    generate_image_via_ollama(host, model, prompt),
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

            Message::ToggleWebSearch => {
                self.web_search_settings.enabled = !self.web_search_settings.enabled;
                if !self.current_chat_is_processing() {
                    self.web_search_for_chat = self.web_search_settings.enabled;
                    self.persist_current_chat_web_search_setting();
                }
                self.persist_web_search_settings();
                Task::none()
            }

            Message::ToggleMultipleWebSearches => {
                self.web_search_settings.allow_multiple_searches =
                    !self.web_search_settings.allow_multiple_searches;
                self.persist_web_search_settings();
                Task::none()
            }

            Message::ToggleChatWebSearch => {
                if self.current_chat_is_processing() {
                    self.set_debug_message(DebugMessage {
                        message: "Web search cannot be changed while this chat is working."
                            .to_string(),
                        is_error: false,
                    });
                    return Task::none();
                }
                self.web_search_for_chat = !self.web_search_for_chat;
                self.persist_current_chat_web_search_setting();
                Task::none()
            }

            Message::WebSearchProviderChange(provider) => {
                self.web_search_settings.provider = provider;
                self.persist_web_search_settings();
                Task::none()
            }

            Message::WebSearchApiKeyChange(api_key) => {
                self.web_search_settings.api_key = if api_key.is_empty() {
                    None
                } else {
                    Some(api_key)
                };
                self.persist_web_search_settings();
                Task::none()
            }

            Message::WebSearchResultLimitChange(value) => {
                self.web_search_settings.result_limit =
                    (value.round() as usize).clamp(1, crate::web_search::MAX_RESULT_LIMIT);
                self.persist_web_search_settings();
                Task::none()
            }

            Message::OpenSource(url) => {
                let parsed = url::Url::parse(&url)
                    .ok()
                    .filter(|url| matches!(url.scheme(), "http" | "https"));
                match parsed {
                    Some(url) => open_url(url.to_string()),
                    None => {
                        self.set_debug_message(DebugMessage {
                            message: "Could not open the source link.".to_string(),
                            is_error: true,
                        });
                        Task::none()
                    }
                }
            }

            Message::UrlOpened(result) => {
                if let Err(error) = result {
                    self.set_debug_message(DebugMessage {
                        message: error,
                        is_error: true,
                    });
                }
                Task::none()
            }

            Message::NewChat => {
                self.save_open_chat();
                self.current_chat_id = Self::new_chat_id();
                self.temporary_chat = false;
                self.web_search_for_chat = self.web_search_settings.enabled;
                self.clear_open_chat();
                Task::none()
            }

            Message::OpenChat(id) => {
                self.save_open_chat();
                let running_history = self
                    .active_prompts
                    .get(&id)
                    .map(|job| Arc::clone(&job.chat_history));
                let running_settings = self
                    .active_prompts
                    .get(&id)
                    .map(|job| (job.temporary, job.web_search_enabled));
                let temporary_history = self
                    .temporary_chats
                    .get(&id)
                    .map(|chat| Arc::clone(&chat.chat_history));
                let temporary_settings = self
                    .temporary_chats
                    .get(&id)
                    .map(|chat| (true, chat.web_search_enabled));
                let chat_settings = running_settings.or(temporary_settings);
                let saved = self.saved_chats.iter().find(|chat| chat.id == id).cloned();
                let saved_web_search_enabled =
                    saved.as_ref().and_then(|chat| chat.web_search_enabled);
                if let Some(chat_history) = running_history
                    .or(temporary_history)
                    .or_else(|| saved.map(|chat| Arc::new(Mutex::new(chat.to_current()))))
                {
                    self.current_chat_id = id;
                    if let Some((_, shown_at)) = self.chat_notices.get_mut(&self.current_chat_id) {
                        *shown_at = Instant::now();
                    }
                    self.temporary_chat = chat_settings
                        .map(|(temporary, _)| temporary)
                        .unwrap_or(false);
                    self.web_search_for_chat = chat_settings
                        .map(|(_, web_search_enabled)| web_search_enabled)
                        .or(saved_web_search_enabled)
                        .unwrap_or(self.web_search_settings.enabled);
                    self.user_information.chat_history = chat_history;
                    // Rendering caches are positional and belong only to the
                    // previously open chat.
                    self.chat_markdown_cache.clear();
                    self.chat_model_name_cache.clear();
                    self.expanded_thinking.clear();
                    self.last_copied_text = None;
                    self.last_copied_at = None;
                    self.refresh_chat_markdown_cache();
                }
                Task::none()
            }

            Message::DeleteChat(id) => {
                if self.active_prompts.contains_key(&id) {
                    self.set_debug_message(DebugMessage {
                        message: "Stop that chat's response before deleting it.".to_string(),
                        is_error: true,
                    });
                    return Task::none();
                }
                self.saved_chats.retain(|chat| chat.id != id);
                self.chat_notices.remove(&id);
                self.vision_responses.remove(&id);
                if self.current_chat_id == id {
                    self.current_chat_id = Self::new_chat_id();
                    self.clear_open_chat();
                }
                self.persist_saved_chats();
                Task::none()
            }

            Message::DeleteTemporaryChat(id) => {
                self.temporary_chats.remove(&id);
                self.chat_notices.remove(&id);
                self.vision_responses.remove(&id);
                if self.current_chat_id == id {
                    self.current_chat_id = Self::new_chat_id();
                    self.temporary_chat = false;
                    self.web_search_for_chat = self.web_search_settings.enabled;
                    self.clear_open_chat();
                }
                Task::none()
            }

            Message::ToggleChatPin(id) => {
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
                if self.temporary_chat {
                    if !self.active_prompts.contains_key(&self.current_chat_id) {
                        self.temporary_chats.remove(&self.current_chat_id);
                        self.chat_notices.remove(&self.current_chat_id);
                        self.vision_responses.remove(&self.current_chat_id);
                    }
                    self.temporary_chat = false;
                    self.current_chat_id = Self::new_chat_id();
                    self.web_search_for_chat = self.web_search_settings.enabled;
                    self.clear_open_chat();
                } else {
                    self.save_open_chat();
                    self.temporary_chat = true;
                    self.current_chat_id = Self::new_chat_id();
                    self.web_search_for_chat = self.web_search_settings.enabled;
                    self.clear_open_chat();
                }
                Task::none()
            }

            Message::ChooseChatFolder => {
                if !self.active_prompts.is_empty() {
                    self.set_debug_message(DebugMessage {
                        message: "Wait for running chats before changing the chat folder."
                            .to_string(),
                        is_error: false,
                    });
                    Task::none()
                } else {
                    Task::perform(
                        async { rfd::FileDialog::new().pick_folder() },
                        Message::ChatFolderSelected,
                    )
                }
            }

            Message::ChatFolderSelected(Some(folder)) => {
                // The folder picker is asynchronous, so a prompt may have
                // started after it opened. Keep the current storage location
                // stable until every background chat has finished.
                if !self.active_prompts.is_empty() {
                    self.set_debug_message(DebugMessage {
                        message: "Wait for running chats before changing the chat folder."
                            .to_string(),
                        is_error: false,
                    });
                    return Task::none();
                }
                self.save_open_chat();
                let previous_directory = self.chat_storage_dir.clone();
                let result = fs::create_dir_all(&folder)
                    .map_err(|error| error.to_string())
                    .and_then(|_| {
                        self.chat_storage_dir = folder.clone();
                        self.persist_chat_storage_dir()
                    });
                if result.is_ok() {
                    self.saved_chats = fs::read_to_string(folder.join("chats.json"))
                        .ok()
                        .and_then(|data| serde_json::from_str(&data).ok())
                        .unwrap_or_default();
                } else {
                    self.chat_storage_dir = previous_directory;
                }
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
                for job in self.active_prompts.values_mut() {
                    while let Ok(state) = job.web_search_state_receiver.try_recv() {
                        job.web_search_state = state;
                    }
                    while let Ok(markdown) = job.markdown_receiver.try_recv() {
                        job.parsed_markdown = markdown;
                    }
                }
                while let Ok((chat_id, notice)) = self.chat_notice_receiver.try_recv() {
                    self.chat_notices.insert(chat_id, (notice, Instant::now()));
                }

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
                        Message::AsyncResult,
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
                                    let mut names =
                                        bots.into_iter().map(|bot| bot.name).collect::<Vec<_>>();
                                    names.sort();
                                    names.dedup();
                                    *bots_list.lock().unwrap() = names;
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
                        Message::AsyncResult,
                    );
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

                    let path = history_path();
                    let result = path
                        .parent()
                        .map(fs::create_dir_all)
                        .transpose()
                        .and_then(|_| {
                            fs::write(
                                path,
                                serde_json::to_string_pretty(&self.app_state.logs).unwrap(),
                            )
                        });
                    match result {
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
                self.persist_boolean_setting(
                    "current_chat_history_enabled",
                    self.user_information.current_chat_history_enabled,
                );
                Task::none()
            }

            Message::ToggleFiltering => {
                self.app_state.filtering = !self.app_state.filtering;
                self.app_state.logs.filtering = self.app_state.filtering;
                self.persist_boolean_setting("filtering", self.app_state.filtering);
                Task::none()
            }

            Message::ToggleDarkMode => {
                self.app_state.dark_mode = !self.app_state.dark_mode;
                gui::set_dark_mode(self.app_state.dark_mode);
                self.persist_boolean_setting("dark_mode", self.app_state.dark_mode);
                Task::none()
            }

            Message::WipeChatHistory => {
                // The saved conversation keeps its id and remains on disk. Further
                // messages start a fresh chat, so clearing context cannot overwrite it.
                self.save_open_chat();
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

            Message::UpdateMaxResponseTokens(value) => {
                let tokens = ((value / 512.0).round() as u32 * 512)
                    .clamp(MIN_RESPONSE_TOKENS, MAX_RESPONSE_TOKENS);
                self.user_information.max_response_tokens = tokens;
                self.persist_setting_value("max_response_tokens", serde_json::Value::from(tokens));
                Task::none()
            }

            Message::UpdateContextTokens(value) => {
                let tokens = ((value / 1_024.0).round() as u32 * 1_024)
                    .clamp(MIN_CONTEXT_TOKENS, MAX_CONTEXT_TOKENS);
                self.user_information.context_tokens = tokens;
                self.persist_setting_value("context_tokens", serde_json::Value::from(tokens));
                Task::none()
            }

            Message::LanguageChange(language) => {
                self.user_information.language = language;
                let value = match language {
                    Language::English => "english",
                    Language::Spanish => "spanish",
                };
                self.persist_setting_value("language", serde_json::Value::String(value.into()));
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

            Message::ModelCapabilitiesKnown(model, capabilities) => {
                if self.user_information.model.as_ref() == Some(&model)
                    && let Some((thinking, vision, image_generation)) = capabilities
                {
                    self.user_information.thinking_supported = Some(thinking);
                    self.user_information.vision_supported = Some(vision);
                    self.user_information.image_generation_supported = Some(image_generation);
                    if !thinking {
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
                    Message::AsyncResult,
                )
            }

            Message::ModelChange(model) => {
                self.user_information.model = Some(model.clone());
                self.user_information.thinking_supported = None;
                self.user_information.vision_supported = None;
                self.user_information.image_generation_supported = None;
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
                        let capabilities = match result {
                            Some(response) if response.status().is_success() => response
                                .json::<serde_json::Value>()
                                .await
                                .ok()
                                .and_then(|json| model_capabilities(&json)),
                            _ => None,
                        };
                        (model, capabilities)
                    },
                    |(model, capabilities)| Message::ModelCapabilitiesKnown(model, capabilities),
                )
            }

            Message::InstallationPrompt => open_url("https://ollama.com/download".to_string()),

            Message::ListPrompt => open_url("https://ollama.com/search".to_string()),

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
                if !self.current_chat_is_processing() {
                    let mut prompt = prompt.trim().to_string();
                    if prompt.is_empty() && self.pending_images.is_empty() {
                        self.set_debug_message(DebugMessage {
                            message: "Enter a message or attach an image first.".to_string(),
                            is_error: true,
                        });
                        return Task::none();
                    }
                    if !self.pending_images.is_empty()
                        && self.user_information.vision_supported == Some(false)
                    {
                        self.set_debug_message(DebugMessage {
                            message:
                                "The selected model cannot inspect images. Choose a model with the `vision` capability."
                                    .to_string(),
                            is_error: true,
                        });
                        return Task::none();
                    }
                    if self.user_information.model.is_none() {
                        self.set_debug_message(DebugMessage {
                            message: "Select a model before sending a message.".to_string(),
                            is_error: true,
                        });
                        return Task::none();
                    }
                    if prompt.is_empty() {
                        prompt = "Describe this image in detail.".to_string();
                    }
                    self.prompt.prompt = String::new();
                    return Self::prompt(self, prompt);
                }

                Task::none()
            }

            Message::StopResponse => {
                if let Some(job) = self.active_prompts.get(&self.current_chat_id) {
                    job.cancel.store(true, Ordering::Relaxed);
                }
                self.chat_notices.insert(
                    self.current_chat_id.clone(),
                    (
                        DebugMessage {
                            message: "Stopping response…".to_string(),
                            is_error: false,
                        },
                        Instant::now(),
                    ),
                );
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

        let prompts_path = resource_path("config/defaultprompts.json");
        let data_prompts: String = match fs::read_to_string(&prompts_path) {
            Ok(dp) => dp,
            Err(_e) => {
                println!("An error occurred reading default prompts");
                json_error.push_str("| Failed to read the installed default prompts");
                "{}".to_string()
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
        system_prompts.sort();
        let selected_system_prompt = if system_prompts_as_prompt.contains_key("default") {
            Some("default".to_string())
        } else {
            system_prompts.first().cloned()
        };

        println!("Loaded system prompts:\n{:?} ", system_prompts);

        let settings = match load_settings_text() {
            Some(dp) => dp,
            None => {
                println!("An error occurred reading settings");
                json_error.push_str("| Failed to read settings");
                "{}".to_string()
            }
        };

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
        let setting_u32 = |key, default, minimum, maximum| {
            settings_hmap
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(default)
                .clamp(minimum, maximum)
        };
        let filtering = setting_bool("filtering", true);
        let dark_mode = setting_bool("dark_mode", true);
        let logging = setting_bool("logging", false);
        let info_popup = setting_bool("info_popup", false);
        let fast_streaming = setting_bool("fast_streaming", true);
        let current_chat_history_enabled = setting_bool("current_chat_history_enabled", true);
        let web_search_settings = settings_hmap
            .get("web_search")
            .cloned()
            .and_then(|value| serde_json::from_value::<WebSearchSettings>(value).ok())
            .unwrap_or_default()
            .normalized();
        let max_response_tokens = setting_u32(
            "max_response_tokens",
            DEFAULT_MAX_RESPONSE_TOKENS,
            MIN_RESPONSE_TOKENS,
            MAX_RESPONSE_TOKENS,
        );
        let context_tokens = setting_u32(
            "context_tokens",
            DEFAULT_CONTEXT_TOKENS,
            MIN_CONTEXT_TOKENS,
            MAX_CONTEXT_TOKENS,
        );
        let language = match settings_hmap
            .get("language")
            .and_then(|value| value.as_str())
        {
            Some("spanish") => Language::Spanish,
            _ => Language::English,
        };
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
        let restored_chat = saved_chats
            .iter()
            .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
            .cloned();
        let current_chat_id = restored_chat
            .as_ref()
            .map(|chat| chat.id.clone())
            .unwrap_or_else(Self::new_chat_id);
        let web_search_for_chat = restored_chat
            .as_ref()
            .and_then(|chat| chat.web_search_enabled)
            .unwrap_or(web_search_settings.enabled);
        let current_chat =
            restored_chat
                .as_ref()
                .map(SavedChat::to_current)
                .unwrap_or(CurrentChat {
                    chats: vec![],
                    messages: vec![],
                    bot_responding: false,
                });

        let history: History = History {
            began_logging: Local::now().to_rfc3339(),
            version: APP_VERSION.to_string(),
            filtering,
            logs: vec![],
        };

        let history_file = history_path();
        let history_result = history_file
            .parent()
            .map(fs::create_dir_all)
            .transpose()
            .and_then(|_| {
                fs::write(
                    history_file,
                    serde_json::to_string_pretty(&history).unwrap(),
                )
            });
        match history_result {
            Ok(_) => {}
            Err(_) => {
                eprintln!("An error writing to history.json");
                json_error.push_str("Unable to write to history.json");
            }
        };

        let (chat_notice_sender, chat_notice_receiver) = crossbeam_channel::unbounded();
        gui::set_dark_mode(dark_mode);

        Self {
            batch_tokens: 3,
            fast_streaming,
            chat_menu_open: true,
            temporary_chat: false,
            web_search_for_chat,
            web_search_settings,
            current_chat_id,
            saved_chats,
            chat_storage_dir,
            active_prompts: HashMap::new(),
            temporary_chats: HashMap::new(),
            chat_notices: HashMap::new(),
            chat_notice_sender,
            chat_notice_receiver,
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
            last_copied_text: None,
            last_copied_at: None,
            pending_images: Vec::new(),
            generated_images: load_generated_images(),
            is_generating_image: false,
            vision_responses: HashMap::new(),
            expanded_thinking: HashSet::new(),

            system_prompt: SystemPrompt {
                system_prompts_as_hashmap: system_prompts_as_prompt,
                system_prompts_as_vec: Arc::new(Mutex::new(system_prompts)),
                system_prompt: selected_system_prompt,
            },
            channels: Channels {
                debug_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<DebugMessage>())),
                logging_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<Log>())),
            },
            user_information: UserInformation {
                chat_history: Arc::new(Mutex::new(current_chat)),
                current_chat_history_enabled,
                model: None,
                thinking_level: ThinkingLevel::Off,
                thinking_supported: None,
                vision_supported: None,
                image_generation_supported: None,
                max_response_tokens,
                context_tokens,
                temperature: 7.0,
                text_size: 24.0,
                ip_address: HostLocation {
                    ip: "127.0.0.1".to_string(),
                    port: "11434".to_string(),
                },
                language,
            },
            prompt: Prompt {
                prompt: String::new(),
            },
            app_state: AppState {
                filtering,
                dark_mode,
                gui_state: if info_popup {
                    GUIState::InfoPopup
                } else {
                    GUIState::Main
                },
                logs: history,
                logging,
                ollama_state: Arc::new(Mutex::new("Offline".to_string())),
                bots_list: Arc::new(Mutex::new(vec![])),
            },
        }
    }
}

pub fn main() -> iced::Result {
    let icon = match image::ImageReader::open(resource_path("assets/icon.ico")) {
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
        min_size: Some(Size::new(900.0, 640.0)),
        ..iced::window::Settings::default()
    };

    iced::application(Program::boot, Program::update, Program::view)
        .subscription(Program::subscription)
        .theme(|program: &Program| {
            if program.app_state.dark_mode {
                Theme::Dark
            } else {
                Theme::Light
            }
        })
        .window_size(Size::new(1100.0, 800.0))
        .window(window_settings)
        .run()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use iced_widget::markdown;

    use super::{
        ActivePrompt, Correspondence, CurrentChat, Message, Program, WebSearchState, app_data_dir,
        censor_text, decode_generation_line, disabled_web_tool_message, model_capabilities,
        normalize_code_fence_languages, parse_markdown_items, split_thinking_text,
    };

    fn test_active_prompt(
        chat_history: Arc<Mutex<CurrentChat>>,
        cancel: Arc<AtomicBool>,
    ) -> ActivePrompt {
        let (_markdown_sender, markdown_receiver) = crossbeam_channel::unbounded();
        let (_web_sender, web_search_state_receiver) = crossbeam_channel::unbounded();
        ActivePrompt {
            chat_history,
            response_as_string: Arc::new(Mutex::new("Background answer".to_string())),
            parsed_markdown: Vec::new(),
            markdown_receiver,
            web_search_state: WebSearchState::Idle,
            web_search_state_receiver,
            cancel,
            model_name: "test-model".to_string(),
            started_at: Instant::now(),
            response_start_index: 1,
            had_image: false,
            web_search_enabled: true,
            temporary: false,
        }
    }

    #[test]
    fn content_filter_replaces_entire_inappropriate_words_with_hashes() {
        assert_eq!(censor_text("hello crap"), "hello ####");
    }

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

    #[test]
    fn reads_distinct_ollama_image_capabilities() {
        let details = serde_json::json!({
            "capabilities": ["completion", "thinking", "vision"]
        });
        assert_eq!(model_capabilities(&details), Some((true, true, false)));

        let generator = serde_json::json!({ "capabilities": ["image"] });
        assert_eq!(model_capabilities(&generator), Some((false, false, true)));
    }

    #[test]
    fn retains_ollama_generation_stop_reason() {
        let line = r#"{
            "model":"test",
            "created_at":"2026-01-01T00:00:00Z",
            "response":"",
            "done":true,
            "done_reason":"length",
            "eval_count":10240
        }"#;
        let (response, reason) = decode_generation_line(line).unwrap();
        assert!(response.done);
        assert_eq!(reason.as_deref(), Some("length"));
    }

    #[test]
    fn explains_disabled_model_web_tool_attempts() {
        assert!(
            disabled_web_tool_message(r#"{"tool":"web_search","arguments":{"query":"today"}}"#)
                .is_some()
        );
        assert!(disabled_web_tool_message("A normal answer about web search.").is_none());
    }

    #[test]
    fn failed_prompt_does_not_relabel_an_older_response() {
        let mut chat = CurrentChat {
            chats: Vec::new(),
            messages: vec![Correspondence::Bot {
                text: "Older answer".into(),
                model: None,
                thinking_seconds: None,
                sources: Vec::new(),
                web_search_used: false,
            }],
            bot_responding: false,
        };

        Program::apply_response_metadata(&mut chat, 1, "new-model", 9);

        assert!(matches!(
            &chat.messages[0],
            Correspondence::Bot {
                model: None,
                thinking_seconds: None,
                ..
            }
        ));
    }

    #[test]
    fn response_metadata_only_updates_the_new_prompt_boundary() {
        let mut chat = CurrentChat {
            chats: Vec::new(),
            messages: vec![
                Correspondence::Bot {
                    text: "Older answer".into(),
                    model: None,
                    thinking_seconds: None,
                    sources: Vec::new(),
                    web_search_used: false,
                },
                Correspondence::User {
                    text: "New question".into(),
                    images: Vec::new(),
                },
                Correspondence::Bot {
                    text: "New answer".into(),
                    model: None,
                    thinking_seconds: None,
                    sources: Vec::new(),
                    web_search_used: false,
                },
            ],
            bot_responding: false,
        };

        Program::apply_response_metadata(&mut chat, 2, "new-model", 9);

        assert!(matches!(
            &chat.messages[0],
            Correspondence::Bot {
                model: None,
                thinking_seconds: None,
                ..
            }
        ));
        assert!(matches!(
            &chat.messages[2],
            Correspondence::Bot {
                model: Some(model),
                thinking_seconds: Some(9),
                ..
            } if model == "new-model"
        ));
    }

    #[test]
    fn navigating_away_keeps_prompt_running_and_finishes_its_original_chat() {
        let test_app_data_dir = app_data_dir();
        let _ = std::fs::remove_dir_all(&test_app_data_dir);
        let mut program = Program::default();
        program.active_prompts.clear();
        program.saved_chats.clear();
        program.temporary_chats.clear();
        let test_storage_dir = std::env::temp_dir().join(format!(
            "ollama-gui-background-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        program.chat_storage_dir.clone_from(&test_storage_dir);

        let chat_a_id = "background-chat-a".to_string();
        let chat_a = Arc::new(Mutex::new(CurrentChat {
            chats: Vec::new(),
            messages: vec![Correspondence::User {
                text: "Background question".to_string(),
                images: Vec::new(),
            }],
            bot_responding: true,
        }));
        let cancel = Arc::new(AtomicBool::new(false));

        program.current_chat_id.clone_from(&chat_a_id);
        program.user_information.chat_history = Arc::clone(&chat_a);
        program.active_prompts.insert(
            chat_a_id.clone(),
            test_active_prompt(Arc::clone(&chat_a), Arc::clone(&cancel)),
        );

        let original_storage_dir = program.chat_storage_dir.clone();
        drop(program.update(Message::ChatFolderSelected(Some(
            original_storage_dir.join("should-not-be-used"),
        ))));
        assert_eq!(program.chat_storage_dir, original_storage_dir);

        drop(program.update(Message::NewChat));
        let foreground_chat_id = program.current_chat_id.clone();
        assert_ne!(foreground_chat_id, chat_a_id);
        assert!(!cancel.load(Ordering::Relaxed));
        assert!(program.active_prompts.contains_key(&chat_a_id));

        chat_a.lock().unwrap().push_message(Correspondence::Bot {
            text: "Background answer".to_string(),
            model: None,
            thinking_seconds: None,
            sources: Vec::new(),
            web_search_used: true,
        });
        chat_a.lock().unwrap().bot_responding = false;

        drop(program.update(Message::PromptFinished(chat_a_id.clone())));

        assert!(!program.active_prompts.contains_key(&chat_a_id));
        assert_eq!(program.current_chat_id, foreground_chat_id);
        assert!(
            program
                .user_information
                .chat_history
                .lock()
                .unwrap()
                .messages
                .is_empty()
        );
        let saved_chat = program
            .saved_chats
            .iter()
            .find(|chat| chat.id == chat_a_id)
            .expect("background chat should be updated in saved chats");
        assert_eq!(saved_chat.messages.len(), 2);
        assert_eq!(saved_chat.web_search_enabled, Some(true));
        let reopened_chat = saved_chat.to_current();
        assert!(matches!(
            &reopened_chat.messages[1],
            Correspondence::Bot {
                text,
                model: Some(model),
                ..
            } if text == "Background answer" && model == "test-model"
        ));

        drop(program.update(Message::OpenChat(chat_a_id.clone())));
        assert!(program.web_search_for_chat);
        drop(program.update(Message::ToggleChatWebSearch));
        assert!(!program.web_search_for_chat);
        assert_eq!(
            program
                .saved_chats
                .iter()
                .find(|chat| chat.id == chat_a_id)
                .and_then(|chat| chat.web_search_enabled),
            Some(false)
        );

        std::fs::remove_dir_all(test_storage_dir).unwrap();
        std::fs::remove_dir_all(test_app_data_dir).unwrap();
    }

    #[test]
    fn normalizes_common_code_fence_language_aliases_without_touching_code() {
        let input = "```csharp\nlet marker = \"```csharp\";\n```\n~~~cplusplus\nint main() {}\n~~~";
        let normalized = normalize_code_fence_languages(input);
        assert_eq!(
            normalized,
            "```cs\nlet marker = \"```csharp\";\n```\n~~~cpp\nint main() {}\n~~~"
        );

        let items = parse_markdown_items("```csharp\nConsole.WriteLine(\"hi\");\n```");
        assert!(matches!(
            &items[0],
            markdown::Item::CodeBlock {
                language: Some(language),
                ..
            } if language == "cs"
        ));
    }
}
