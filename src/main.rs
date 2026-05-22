#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::fs;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Local;
use futures::stream::StreamExt;
use futures::Stream;
use iced::{clipboard, keyboard, time, Element, Size, Subscription, Task, Theme};
use iced_widget::markdown;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::completion::GenerationResponse;
use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use rustrict::Censor;
use serde_json;
use webbrowser;

use image;

mod gui;
mod app;

use crate::app::{
    AppState, Channels, Correspondence, CurrentChat, DebugMessage, History, HostLocation, Log,
    Prompt, Response, SystemPrompt, UserInformation,
};

/// Tick points:
/// Each tick occurs every TICK_MS; these constants decide what happens on each tick.
const VERSION_TICK: i32 = 2;
const MAX_TICK: i32 = 50;
const BOT_LIST_TICK: i32 = 3;
const TICK_MS: u64 = 200;

const APP_VERSION: &str = "0.4.0";

#[derive(PartialEq, Clone, Copy)]
pub enum GUIState {
    InfoPopup,
    Main,
    Settings,
    AdvancedSettings,
}

#[derive(Debug, Clone)]
enum Message {
    ChangeBatchTokens(i32),
    AsyncResult(()),
    ListPrompt,
    ToggleThinking,
    ToggleSettings,
    SystemPromptChange(String),
    Prompt(String),
    UpdatePrompt(String),
    None,
    KeyPressed(keyboard::Key),
    KeyReleased(keyboard::Key),
    Tick,
    CopyPressed(String),
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

    system_prompt: SystemPrompt,
    app_state: AppState,
    channels: Channels,
    user_information: UserInformation,
    response: Response,
    prompt: Prompt,
    batch_tokens: i32,
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

impl Program {
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

        let mut new_markdown_cache: Vec<Vec<markdown::Item>> =
            Vec::with_capacity(messages.len());
        let mut new_model_name_cache: Vec<Option<String>> =
            Vec::with_capacity(messages.len());

        for (index, message) in messages.iter().enumerate() {
            match message {
                Correspondence::User(_) => {
                    new_markdown_cache.push(Vec::new());
                    new_model_name_cache.push(None);
                }

                Correspondence::Bot(text) => {
                    if let Some(cached) = old_markdown_cache.get(index) {
                        new_markdown_cache.push(cached.clone());
                    } else {
                        new_markdown_cache.push(markdown::parse(text).collect());
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
            Channels::send_request_to_channel(
                Arc::clone(&self.channels.debounce_channel),
                false,
            );
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

        let (markdown_sender, markdown_receiver) = crossbeam_channel::unbounded();
        self.channels.markdown_channel_reciever = markdown_receiver;

        let (tx, rx) = std::sync::mpsc::channel::<GenerationResponse>();
        let channels: Channels = self.channels.clone();
        let batch_tokens = self.batch_tokens.clone();
        let response_string = Arc::clone(&self.response.response_as_string);

        std::thread::spawn(move || {
            fn render(
                buffer: &String,
                markdown_sender: crossbeam_channel::Sender<Vec<markdown::Item>>,
                channels: Channels,
            ) {
                let md = markdown::parse(&buffer.clone()).collect();

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

                if !(total_tokens >= batch_tokens || last_render_time.elapsed().as_secs() >= 5) {
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
            Channels::send_request_to_channel(
                Arc::clone(&self.channels.debounce_channel),
                false,
            );
            return Task::none();
        }

        let logging = self.app_state.logging.clone();
        let filtering = self.app_state.filtering.clone();
        let user_info = self.user_information.clone();
        let channels = self.channels.clone();

        user_info
            .chat_history
            .lock()
            .unwrap()
            .push_message(Correspondence::User(prompt.clone()));

        self.refresh_chat_markdown_cache();

        Task::perform(
            async move {
                println!("Received prompt: {}", prompt.clone());
                user_info.chat_history.lock().unwrap().bot_responding = true;

                let system_prompt: String = system_prompt.unwrap();
                let ip = user_info.ip_address.clone();
                let ollama = Ollama::new(format!("http://{}", ip.ip), convert_port_to_u16(ip.port));

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

                let mut response: Pin<
                    Box<
                        dyn Stream<
                                Item = Result<
                                    Vec<GenerationResponse>,
                                    ollama_rs::error::OllamaError,
                                >,
                            > + Send
                            + 'static,
                    >,
                > = match ollama.generate_stream(request.think(user_info.think)).await {
                    Ok(stream) => stream,
                    Err(e) => {
                        eprintln!("Error generating response: {}", e);
                        Channels::send_request_to_channel(
                            Arc::clone(&channels.debug_channel),
                            DebugMessage {
                                message:
                                    "Error getting ollama response (have you enabled thinking on a bot which does not allow this feature?)"
                                        .to_string(),
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

                while let Some(data) = response.next().await {
                    match data {
                        Ok(responses) => {
                            for token in responses {
                                print!("{}", token.response);

                                let filtered_token: GenerationResponse = if filtering {
                                    GenerationResponse {
                                        response: Censor::from_str(token.response.as_str()).censor(),
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
                        }
                        Err(e) => {
                            eprintln!("Error in stream: {}", e);
                            Channels::send_request_to_channel(
                                Arc::clone(&channels.debug_channel),
                                DebugMessage {
                                    message: "Error while streaming Ollama response".to_string(),
                                    is_error: true,
                                },
                            );
                            break;
                        }
                    }
                }

                if logging {
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

                if user_info.current_chat_history_enabled {
                    user_info
                        .chat_history
                        .lock()
                        .unwrap()
                        .generate_and_push(prompt.clone(), final_response.join(""));
                }

                user_info
                    .chat_history
                    .lock()
                    .unwrap()
                    .push_message(Correspondence::Bot(final_response.join("")));

                user_info.chat_history.lock().unwrap().bot_responding = false;

                Channels::send_request_to_channel(
                    Arc::clone(&channels.debounce_channel),
                    false,
                );
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

            Message::ChangeBatchTokens(new_batch_tokens) => {
                self.batch_tokens = new_batch_tokens;
                Task::none()
            }

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
                    let ollama = Ollama::new(format!("http://{}", ip.ip), convert_port_to_u16(ip.port));
                    let bots_list = Arc::clone(&self.app_state.bots_list);
                    let channels = self.channels.clone();

                    return Task::perform(
                        async move {
                            match ollama.list_local_models().await {
                                Ok(bots) => {
                                    bots.iter().for_each(|bot| {
                                        if !(bots_list.lock().unwrap().contains(&bot.name.to_string())) {
                                            println!("Found bot: {}", bot.name);
                                            bots_list.lock().unwrap().push(bot.name.to_string());
                                        }
                                    });
                                }
                                Err(e) => {
                                    Channels::send_request_to_channel(
                                        Arc::clone(&channels.debug_channel),
                                        DebugMessage {
                                            message: "Error occurred while listing bots".to_string(),
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
                self.response.parsed_markdown = vec![];

                if let Ok(mut response_text) = self.response.response_as_string.lock() {
                    *response_text = String::new();
                }

                self.set_debug_message(DebugMessage {
                    message: "Chat history wiped.".to_string(),
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

            Message::ToggleThinking => {
                self.user_information.think = !self.user_information.think;
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
                let ollama = Ollama::new(format!("http://{}", ip.ip), convert_port_to_u16(ip.port));
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
                                println!("Failed to install model {}: {:?}", model_install, outcome);
                                Channels::send_request_to_channel(
                                    Arc::clone(&channels.debug_channel),
                                    DebugMessage {
                                        message: format!("Failed to install model {}", model_install),
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
                self.user_information.model = Some(model);
                Task::none()
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

            Message::KeyPressed(_key) => Task::none(),

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
                iced::event::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    Some(Message::KeyPressed(key))
                }
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
                    json_error.push_str("| Failed to read: ./config/defaultprompts.json (bad formatting)");
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

        let settings_hmap: HashMap<String, bool> = match serde_json::from_str(&settings) {
            Ok(sp) => sp,
            Err(_e) => {
                println!("An error occurred reading settings (bad format)");
                json_error.push_str("| Failed to read: ./config/settings.json (bad formatting. reset to default)");
                HashMap::from([
                    ("filtering".to_string(), false),
                    ("logging".to_string(), false),
                    ("dark_mode".to_string(), false),
                    ("info_popup".to_string(), false),
                ])
            }
        };

        let filtering = *settings_hmap.get("filtering").unwrap_or(&true);
        let logging = *settings_hmap.get("logging").unwrap_or(&false);
        let info_popup = *settings_hmap.get("info_popup").unwrap_or(&false);
        let dark_mode = *settings_hmap.get("dark_mode").unwrap_or(&false);

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
            is_processing: false,
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
                think: false,
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
    unsafe {
        #[cfg(target_os = "windows")]
        std::env::set_var("WGPU_BACKEND", "gl");
    }

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

    let dark_mode = *settings_hmap.get("dark_mode").unwrap_or(&false);

    let mode: Theme = if dark_mode {
        Theme::Dark
    } else {
        Theme::Light
    };

    iced::application(|| Program::boot(), Program::update, Program::view)
        .subscription(Program::subscription)
        .theme(mode)
        .window_size(Size::new(700.0, 785.0))
        .window(window_settings)
        .run()
}
