//#![windows_subsystem = "windows"]
//std crate imports
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Context};
use std::fs;
//external crate imports
use chrono::Local;
use futures::Stream;
use iced::{ clipboard, keyboard, Element, Size, Subscription, Task, Theme};
use iced_widget::markdown;
use ollama_rs::generation::completion::GenerationResponse;
use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use tokio::runtime::Runtime; 
use iced_native::subscription::Recipe;
use futures::stream::StreamExt;
use webbrowser;
use serde_json;
use rustrict::Censor;
use image;
//local file imports
mod gui; 
mod app;
use crate::app::{AppState, Channels, Correspondence, CurrentChat, DebugMessage, History, HostLocation, Log, Prompt, Response, SystemPrompt, UserInformation};

/// Tick points:
/// Each tick occurs every 1ms; so these will perform certain actions 
/// at each corresponding tick.
/// These are constants for the purpose of easy modification. 
const VERSION_TICK: i32 = 5000; // The tick in which the version of the program will be checked 
const MAX_TICK: i32 = 20000; // The maximum tick in which the ticks will reset
const BOT_LIST_TICK: i32 = 1000; // The tick in which the Ollama bots list will be checked
const TICK_MS: u64 = 200; // Tick rate
///
const APP_VERSION: &str = "0.3.3"; // The current version of the application

#[derive(PartialEq, Clone, Copy)]
pub enum GUIState {
    InfoPopup, 
    Main,
    Settings,
    AdvancedSettings
}

// message enum defined to send communications to the GUI logic
#[derive(Debug, Clone)]
enum Message {
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
    ChangeIp(String),
    ChangePort(String)
} 


// program struct, stores the current program state
// e.g., the current prompt, debug message, etc.
struct Program { 
    runtime: Runtime,
    is_processing: bool,
    current_tick: i32,
    installing_model: String,
    debug_message: DebugMessage,
    system_prompt: SystemPrompt,
    app_state: AppState, 
    channels: Channels, 
    user_information: UserInformation,
    response: Response,
    prompt: Prompt
}

fn convert_port_to_u16(port: String) -> u16 {
    return match port.parse::<u16>() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Invalid port number: {}", port);
            11434 as u16
        }
    };
} 

// impliment the program function with several functions
// to allow the program to function
// e.g. view() is for gui logic
impl Program { 

    // this function will prompt the Ollama interface and recieve a reaction, 
    // then send this information to the GUI
    fn prompt(&mut self, prompt: String) {

        // invalid case handler
        if self.user_information.model == None {
            Channels::send_request_to_channel(Arc::clone(&self.channels.debounce_channel), false);
            Channels::send_request_to_channel(Arc::clone(&self.channels.debug_channel), 
        DebugMessage {
                    message: "Model selected is invalid, have you selected a model?".to_string(), 
                    is_error: true 
                }
            );
            println!("Model is None");
            return; 
        }

        self.prompt.prompt_time_sent = std::time::Instant::now();

        let (markdown_sender, markdown_receiver) = crossbeam_channel::unbounded();
        self.channels.markdown_channel_reciever = markdown_receiver;
        let response_arc: Arc<Mutex<String>> = Arc::clone(&self.response.response_as_string);
        let (tx, rx) = std::sync::mpsc::channel::<GenerationResponse>();
        let channels: Channels = self.channels.clone();
        // create a new thread to prevent blocking
        std::thread::spawn(move || {
            for token in rx {
                let mut resp: std::sync::MutexGuard<'_, String> = match response_arc.lock() {
                    Ok(resp) => {
                        resp
                    }
                    Err(e) => {
                        eprintln!("Failed to get response: {}", e);
                        Channels::send_request_to_channel(Arc::clone(&channels.debug_channel), 
                            DebugMessage{
                                message: "Failed to get response [responsearc failed]".to_string(),
                                is_error: true
                            }
                        );
                        return 
                    }
                };
                resp.push_str(&token.response);
                let md = markdown::parse(&resp).collect();
                match markdown_sender.send(md) {
                    Ok(_) => {  }
                    Err(e) => {
                        eprintln!("Failed to send markdown response: {}", e);
                        Channels::send_request_to_channel(Arc::clone(&channels.debug_channel), 
                            DebugMessage{
                                message: "Failed to create markdown response [markdown_sender.send failed]".to_string(),
                                is_error: true
                            }
                        );
                    }
                };
            }
        });

        let system_prompt: Option<String> = SystemPrompt::get_current(&self);
        if system_prompt.is_none() {
            Channels::send_request_to_channel(Arc::clone(&self.channels.debug_channel), 
                DebugMessage{
                    message: "Could not get system prompt, is it selected?".to_string(),
                    is_error: true
                }
            );
            Channels::send_request_to_channel(Arc::clone(&self.channels.debounce_channel), false); 
            return;
        }
        
        let logging = self.app_state.logging.clone(); 
        let filtering = self.app_state.filtering.clone(); 
        let user_info = self.user_information.clone();
        let channels = self.channels.clone();

        user_info.chat_history.lock().unwrap().push_message(Correspondence::User(prompt.clone()));

        
        // create a new tokio runtime
        // this is done because the function is not async
        // but async programming must be done for the REST API calls
        self.runtime.spawn(async move {           
            println!("Received prompt: {}", prompt.clone());
            user_info.chat_history.lock().unwrap().bot_responding = true;

            let system_prompt: String = system_prompt.unwrap();
            let ip = user_info.ip_address.clone();
            let ollama = Ollama::new(format!("http://{}", ip.ip), convert_port_to_u16(ip.port));
            let to_send_prompt: String = if user_info.current_chat_history_enabled {
                format!("The following is a conversation between an AI language model and a User. You are the AI language model:
                    {}
                    [END CONVERSATION CONTEXT]
                    Now, the user is sending another message: {}
                    Respond: 
                    ", user_info.chat_history.lock().unwrap().unravel(), prompt.clone())
            } else {
                prompt.clone()
            };

            let request: GenerationRequest<'_> = GenerationRequest::new(user_info.model.clone().unwrap(), to_send_prompt)
                .options(ModelOptions::default().temperature(user_info.temperature / 10.0))
                .system(system_prompt.clone());
            
            println!("System prompt: {}", system_prompt.clone());

            let mut response: Pin<Box<dyn Stream<Item = Result<Vec<GenerationResponse>, ollama_rs::error::OllamaError>> + Send + 'static>> = match ollama.generate_stream(request.think(user_info.think)).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Error generating response: {}", e);            
                    Channels::send_request_to_channel(Arc::clone(&channels.debug_channel), 
                        DebugMessage{
                            message: "Error getting ollama response (have you enabled thinking on a bot which does not allow this feature?)".to_string(),
                            is_error: true
                        }
                    );
                    Channels::send_request_to_channel(Arc::clone(&channels.debounce_channel), false);     
                    return   
                }
            };
            
            let mut final_response: Vec<String> = vec![];

            // iterate through responses and send them to the mpsc channel
            while let Some(data) = response.next().await {
                match data {
                    Ok(responses) => {
                        for token in responses {
                            print!("{}", token.response);
                            let filtered_token: GenerationResponse = if filtering {
                                GenerationResponse{ 
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
                        break;
                    }
                }
            }
            // tells the is_processing channel to set the variable to false
            Channels::send_request_to_channel(Arc::clone(&channels.debounce_channel), false);

            //logs the information 
            if logging { 
                Channels::send_request_to_channel(Arc::clone(&channels.logging_channel), 
                    Log::create_with_current_time(
                        filtering,
                        user_info.model,
                        final_response.clone(), 
                        Some(system_prompt),
                        prompt.clone()
                    )
                );
            } 

            if user_info.current_chat_history_enabled { 
                user_info.chat_history
                    .lock()
                    .unwrap()
                    .generate_and_push(prompt.clone(), final_response.join(""));
            }

            user_info.chat_history.lock().unwrap().push_message(Correspondence::Bot(final_response.join("")));
            
            user_info.chat_history.lock().unwrap().bot_responding = true;
        });
        self.prompt.prompt = String::new();
    }

    // update function which updates occurding to the current subscription,
    // this handles Message requests
    fn update(&mut self, message: Message) -> Task<Message>  {
        match message { 
            Message::None => {
                Task::none()
            }

            // is activated once every millisecond
            // this will:
            // - check whether Ollama is online
            // - check the currently installed bots
            // - handle mpsc channels
            Message::Tick => { 
                
                //println!("{:?}", self.user_information.chat_history);
             //   println!("Tick: {}", self.current_tick);
                if self.current_tick > MAX_TICK {
                    println!("Resetting current tick");
                    self.current_tick = 0;
                }
                self.current_tick += 1; 

                if self.current_tick == VERSION_TICK {
                    let ollama_state = Arc::clone(&self.app_state.ollama_state);
                    let user_info = self.user_information.clone();

                    self.runtime.spawn(async move {
                        let ip = user_info.ip_address;
                        let url = format!("http://{}:{}/api/version", ip.ip, ip.port.to_string());

                        match reqwest::get(url).await {
                            Ok(response) => {
                                if response.status().is_success() {
                                     match response.json::<serde_json::Value>().await {
                                        Ok(json) => {
                                            if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                                                *ollama_state.lock().unwrap() = format!("Online (v{})", version);
                                            } else {
                                                *ollama_state.lock().unwrap() = "Online (unknown version)".to_string();
                                            }
                                        }
                                        Err(_) => {
                                            *ollama_state.lock().unwrap() = "Online (version parse error)".to_string();
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
                    });
                } else if self.current_tick == BOT_LIST_TICK {      
                    let ip = self.user_information.ip_address.clone();
                    let ollama = Ollama::new(format!("http://{}", ip.ip), convert_port_to_u16(ip.port));
                    let bots_list = Arc::clone(&self.app_state.bots_list);
                    let channels = self.channels.clone();

                    self.runtime.spawn(async move {
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
                                Channels::send_request_to_channel(Arc::clone(&channels.debug_channel),
                                    DebugMessage{
                                        message: "Error occured while listing bots".to_string(),
                                        is_error: true
                                    }
                                );
                                bots_list.lock().unwrap().clear();
                                println!("Error: {:?}", e);
                            }
                        }
                    });
                }

                if let Ok(md) = self.channels.markdown_channel_reciever.try_recv() {
                    self.response.parsed_markdown = md;
                }
                if let Ok(is_processing) = self.channels.debounce_channel.lock().unwrap().1.try_recv() {
                    self.is_processing = is_processing;
                }
                if let Ok(debug_msg) = self.channels.debug_channel.lock().unwrap().1.try_recv()  {
                    self.debug_message = debug_msg;
                }
                if let Ok(log) = self.channels.logging_channel.lock().unwrap().1.try_recv() {
                    self.app_state.logs.push_log(log);

                    match fs::write("./output/history.json", serde_json::to_string_pretty(
                        &self.app_state.logs
                    ).unwrap()) {
                        Ok(_) => {},
                        Err(_) => {
                            eprintln!("An error writing to history.json");
                            self.debug_message = DebugMessage {
                                message: "Failed to write to history.json".to_string(), 
                                is_error: true
                            };
                        }
                    };
                }

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
                self.user_information.current_chat_history_enabled = !self.user_information.current_chat_history_enabled;
                Task::none()
            }

            Message::WipeChatHistory => {
                self.user_information.chat_history = Arc::new(Mutex::new(
                    CurrentChat { 
                        chats: vec![],
                        messages: vec![],
                        bot_responding: false
                    }
                ));
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
                Channels::send_request_to_channel(Arc::clone(&self.channels.debug_channel), 
                    DebugMessage{
                        message: format!("Installing model... {}", model_install).to_string(),
                        is_error: false 
                    }
                );


                let ip = self.user_information.ip_address.clone();
                let ollama = Ollama::new(format!("http://{}", ip.ip), convert_port_to_u16(ip.port));                
                let channels = self.channels.clone();

                self.runtime.spawn(async move {
                    match ollama.pull_model(model_install.clone(), false).await {
                        Ok(outcome) => {
                            println!("Model {} installed successfully: {}", model_install, outcome.message);     
                            Channels::send_request_to_channel(Arc::clone(&channels.debug_channel), 
                                    DebugMessage{ 
                                        message: format!("Installed model {}: {}", model_install, outcome.message),
                                        is_error: false
                                    }
                                );
                        }  
                        Err(outcome) => {
                            println!("Failed to install model {}: {:?}", model_install, outcome);
                            Channels::send_request_to_channel(Arc::clone(&channels.debug_channel), 
                                DebugMessage { 
                                    message: format!("Failed to install model {}", model_install),
                                    is_error: true
                                }
                            );
                        }
                    };
                });
                return Task::none();
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
                return clipboard::write::<Message>(input)
            }

            Message::KeyPressed(_key) => {   
                // match key {
                //     keyboard::Key::Named(name) => {
                //         if name == Named::Enter { 
                //             Self::request_response(self, self.prompt.clone());
                //         } 
                //     }
                //     _ => {

                //     } 
                // }
                
                Task::none()
            }


            Message::KeyReleased(_key) => { 
                Task::none()
            }

            Message::Prompt(prompt) => {
                if !self.is_processing {
                    self.is_processing = true;
                    self.response.parsed_markdown = vec![];
                    *self.response.response_as_string.lock().unwrap() = String::new(); 
                    Self::prompt(self, prompt.clone());
                    self.response.parsed_markdown = markdown::parse("Waiting for bot...").collect();

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

    // display the GUI
    fn view(&self) -> Element<Message> {
        Self::get_ui_information(self, self.app_state.gui_state).into()
    }

    // sets up the Tick and keypressed events
    fn subscription(&self) -> Subscription<Message> {
        struct Timer;
        impl<H: std::hash::Hasher, E> Recipe<H, E> for Timer {            
            type Output = Message;
            fn hash(&self, state: &mut H) {
                use std::hash::Hash;
                "timer".hash(state);
            }

            fn stream(self: Box<Self>, _: futures::stream::BoxStream<'static, E>) -> futures::stream::BoxStream<'static, Self::Output> {
                futures::stream::unfold((), |_| async {
                    tokio::time::sleep(std::time::Duration::from_millis(TICK_MS)).await;
                    Some((Message::Tick, ()))
                }).boxed()
            }
        }

        impl Stream for Timer {
            type Item = Message;

            fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                cx.waker().wake_by_ref();
                Poll::Ready(Some(Message::Tick))
            }
        }
        

        Subscription::batch(vec![
            keyboard::on_key_press(|key, _modifiers| Some(Message::KeyPressed(key))),
            keyboard::on_key_release(|key, _modifiers| Some(Message::KeyReleased(key))),
            Subscription::run_with_id("timer", Timer),
        ])
    }
}


impl Default for Program {
    fn default() -> Self {
        let mut json_error: String = String::new();
        // Reading defaultprompts.json 
        // Ensures that the system prompts are loaded 
        // and visible to user on start-up 
        let data_prompts: String = match fs::read_to_string("./config/defaultprompts.json") {
            Ok(dp) => dp, 
            Err(_e) => {
                println!("An error occured reading default prompts"); 
                json_error.push_str("| Failed to read: ./config/defaultprompts.json");
                "[]".to_string()
            }
        };
        let system_prompts_as_prompt: HashMap<String, String> = match serde_json::from_str(&data_prompts) {
            Ok(sp) => sp, 
            Err(_e) => {
                println!("An error occured reading default prompts (bad format)"); 
                json_error.push_str("| Failed to read: ./config/defaultprompts.json (bad formatting)");
                HashMap::from([(String::new(), String::new())])
            }
        };
        let mut system_prompts: Vec<String> = Vec::new();
        system_prompts_as_prompt.iter().for_each(|prompt| {
            system_prompts.push(prompt.0.clone());
        });
        println!("Loaded system prompts:\n{:?} ", system_prompts);


        // Reading settings.json
        // Ensures that users settings are loaded 
        let settings = match fs::read_to_string("./config/settings.json") {
            Ok(dp) => dp, 
            Err(_e) => {
                println!("An error occured reading settings"); 
                json_error.push_str("| Failed to read: ./config/settings.json");
                "[]".to_string()
            }
        };
        let settings_hmap: HashMap<String, bool> = match serde_json::from_str(&settings) {
            Ok(sp) => sp, 
            Err(_e) => {
                println!("An error occured reading settings (bad format)"); 
                json_error.push_str("| Failed to read: ./config/settings.json (bad formatting. reset to default)");
                HashMap::from([
                    ("filtering".to_string(), false),
                    ("logging".to_string(), false)
                ])
            }
        };
        let filtering = *settings_hmap.get("filtering")
            .unwrap_or(&true);
        let logging = *settings_hmap.get("logging")
            .unwrap_or(&false);
        let info_popup = *settings_hmap.get("info_popup")
            .unwrap_or(&false);
        let dark_mode = *settings_hmap.get("dark_mode")
            .unwrap_or(&false);

        // Writing to history.json for the first time
        let history: History = History { 
            began_logging: Local::now().to_rfc3339(),
            version: APP_VERSION.to_string(),
            filtering: filtering.clone(),
            logs: vec![]
        };

        match fs::write("./output/history.json", serde_json::to_string_pretty(
            &history
        ).unwrap()) {
            Ok(_) => {},
            Err(_) => {
                eprintln!("An error writing to history.json");
                json_error.push_str("Unable to write to history.json");
            }
        };

        Self { 
            runtime: Runtime::new().expect("Failed to create Tokio runtime"), 
            is_processing: false,
            current_tick: 0,
            installing_model: String::new(),
            debug_message: DebugMessage { 
                message: json_error.clone(), 
                is_error: if json_error != String::new() {
                    true
                } else { 
                    false
                }
            },
            system_prompt: SystemPrompt { 
                system_prompts_as_hashmap: system_prompts_as_prompt, 
                system_prompts_as_vec: Arc::new(Mutex::new(system_prompts)), 
                system_prompt: Some(String::new())
            },
            channels: Channels { 
                markdown_channel_reciever: crossbeam_channel::unbounded().1, 
                debug_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<DebugMessage>())), 
                debounce_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<bool>())), 
                logging_channel:  Arc::new(Mutex::new(std::sync::mpsc::channel::<Log>()))
            },
            user_information: UserInformation { 
                chat_history: Arc::new(Mutex::new(CurrentChat {
                    chats: vec![],
                    messages: vec![],
                    bot_responding: false
                })), 
                current_chat_history_enabled: true,
                model: None, 
                think: false,
                temperature: 7.0,
                text_size: 24.0,
                ip_address: HostLocation {
                    ip: "127.0.0.1".to_string(),
                    port: "11434".to_string(),
                }
            },
            response: Response { 
                response_as_string: Arc::new(Mutex::new(String::new())), 
                parsed_markdown: vec![] 
            },
            prompt: Prompt { 
                prompt_time_sent: std::time::Instant::now(),
                prompt: String::new() 
            },
            app_state: AppState { 
                filtering: filtering, 
                gui_state: if info_popup {
                    GUIState::InfoPopup
                } else {
                    GUIState::Main
                },
                dark_mode: dark_mode,
                logs: history, 
                logging: logging, 
                ollama_state: Arc::new(Mutex::new("Offline".to_string())), 
                bots_list:  Arc::new(Mutex::new(vec![]))
            }, 
        }
    }
}

#[tokio::main]
pub async fn main() -> iced::Result {
     let icon = match image::ImageReader::open("./assets/icon.ico") {
        Ok(image_reader) => {
            match image_reader.decode() {
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
                },
                Err(e) => {
                    eprintln!("Failed to decode the image: {}", e);
                    None
                }
            }
        },
        Err(e) => {
            eprintln!("Failed to open the icon file: {}", e);
            None
        }
    };

    let window_settings = iced::window::Settings {
        icon: icon,
        ..iced::window::Settings::default()
    };

    // Reading settings.json  
    let settings = match fs::read_to_string("./config/settings.json") {
        Ok(dp) => dp, 
        Err(_e) => {
            println!("An error occured reading settings"); 
            "[]".to_string()
        }
    };
    let settings_hmap: HashMap<String, bool> = match serde_json::from_str(&settings) {
        Ok(sp) => sp, 
        Err(_e) => {
            println!("An error occured reading settings (bad format)"); 
            HashMap::from([
                ("dark_mode".to_string(), false),
            ])
        }
    };
    let dark_mode = *settings_hmap.get("dark_mode")
        .unwrap_or(&false);
    
    let mode: Theme = if dark_mode {
        Theme::Dark
    } else {
        Theme::Light
    };
    
    // begins the application
    iced::application("Ollama GUI Interface", Program::update, Program::view)
        .window_size(Size::new(700.0, 785.0))
        .subscription(Program::subscription)
        .theme(move |_| mode.clone())
        .window(window_settings)
        .run()
}
