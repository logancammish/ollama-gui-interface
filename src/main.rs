//#![windows_subsystem = "windows"]
//std crate imports
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Context};
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
use lazy_static::lazy_static;
use serde_json;
use serde::Serialize;
use std::fs;
use rustrict::Censor;
//local file imports
mod gui; 

// define lazy_static mpsc channels which will
// allow for interaction between threads and runtimes
// only for main.rs file
lazy_static! {
    static ref CHANNEL: (
        std::sync::mpsc::Sender<bool>,
        Arc<Mutex<std::sync::mpsc::Receiver<bool>>>
    ) = {
        let (txprocess, rxprocess) = std::sync::mpsc::channel::<bool>();
        return (txprocess, Arc::new(Mutex::new(rxprocess)));
    };

    static ref DEBUG_CHANNEL: (
        std::sync::mpsc::Sender<String>,
        Arc<Mutex<std::sync::mpsc::Receiver<String>>>
    ) = {
        let (txprocess, rxprocess) = std::sync::mpsc::channel::<String>();
        return (txprocess, Arc::new(Mutex::new(rxprocess)));
    };

    static ref LOGGING_CHANNEL: (
        std::sync::mpsc::Sender<Log>,
        Arc<Mutex<std::sync::mpsc::Receiver<Log>>>
    ) = {
        let (txprocess, rxprocess) = std::sync::mpsc::channel::<Log>();
        return (txprocess, Arc::new(Mutex::new(rxprocess)));
    };
}

// message enum defined to send communications to the GUI logic
#[derive(Debug, Clone)]
enum Message {
    SystemPromptChange(String),
    Prompt(String),
    UpdatePrompt(String),
    None,
    KeyPressed(keyboard::Key),
    KeyReleased(keyboard::Key),
    Tick,
    CopyPressed(String),
    InstallationPrompt,
    ModelChange(String),
    InstallModel(String),
    UpdateInstall(String)
} 

#[derive(Serialize)]
struct Log { 
    filtering: bool,
    time: String, 
    prompt: String, 
    response: Vec<String>, 
    model: Option<String>, 
    systemprompt: Option<String>
}

impl Log { 
    fn create_with_current_time(filtering: bool, model: Option<String>, response: Vec<String>, systemprompt: Option<String>, prompt: String) -> Self {
        // let mut response_as_string: String = String::new();
        // for r in response {
        //     response_as_string.push_str(r.as_str());
        // }

        return Log { 
            filtering: filtering, 
            time: String::from(Local::now().to_rfc3339()), 
            prompt: prompt, 
            response: response, 
            model: model, 
            systemprompt: systemprompt
        }
    }

}

#[derive(Serialize)]
struct History { 
    began_logging: String, 
    version: String, 
    filtering: bool, 
    logs: Vec<Log>
}
impl History { 
    fn push_log(&mut self, log: Log) {
        self.logs.push(log);
    }
}

// program struct, stores the current program state
// e.g., the current prompt, debug message, etc.
struct Program { 
    system_prompts_as_prompt: HashMap<String, String>,
    system_prompts: Arc<Mutex<Vec<String>>>,
    system_prompt: Option<String>,
    filtering: bool,
    prompt: String,
    prompt_time_sent: std::time::Instant,
    runtime: Runtime,
    response: Arc<Mutex<String>>,
    parsed_markdown: Vec<markdown::Item>,
    markdown_receiver: crossbeam_channel::Receiver<Vec<markdown::Item>>,
    is_processing: bool,
    ollama_state: Arc<Mutex<String>>,
    current_tick: i32,
    bots_list: Arc<Mutex<Vec<String>>>,
    model: Option<String>,
    installing_model: String,
    debug_message: String,
    logs: History,
    logging: bool,
}

// impliment the program function with several functions
// to allow the program to function
// e.g. view() is for gui logic
impl Program { 
    fn update_history(history: History) {
        fs::write("history.json", serde_json::to_string_pretty(
            &history 
        ).unwrap()).expect("Unable to write to history.json");
    } 

    fn prompt(&mut self, prompt: String) {
        // invalid case handler
        if self.model == None {
            CHANNEL.0.send(false).unwrap();
            DEBUG_CHANNEL.0.send("Model selected is invalid, have you selected a model?".to_string()).unwrap();
            println!("Model is None");
            return; 
        }

        self.prompt_time_sent = std::time::Instant::now();

        let (markdown_sender, markdown_receiver) = crossbeam_channel::unbounded();
        self.markdown_receiver = markdown_receiver;
        let runtime_handle = self.runtime.handle().clone();
        let response_arc = Arc::clone(&self.response);
        let (tx, rx) = std::sync::mpsc::channel::<GenerationResponse>();

        // create a new thread to prevent blocking
        std::thread::spawn(move || {
            for token in rx {
                let mut resp = response_arc.lock().unwrap();
                resp.push_str(&token.response);
                let md = markdown::parse(&resp).collect();
                markdown_sender.send(md).unwrap();
            }
        });

        let system_prompt: String;
        let model = self.model.clone();
        if self.system_prompts_as_prompt.get(&self.system_prompt.clone().unwrap()).is_some() {
            system_prompt = self.system_prompts_as_prompt.get(&self.system_prompt.clone().unwrap())
                .expect("System prompt not found")
                .to_string();
        } else { 
            println!("system prompt is None");
            CHANNEL.0.send(false).unwrap();
            DEBUG_CHANNEL.0.send("System prompt not selected or is invalid".to_string()).unwrap();
            return; 
        }
        
        let filtering: bool = self.filtering.clone();
        let logging = self.logging.clone();
        // create a new tokio runtime
        // this is done because the function is not async
        // but async programming must be done for the REST API calls
        runtime_handle.spawn(async move {
            println!("Received prompt: {}", prompt.clone());
            let ollama = Ollama::default();
            let request = GenerationRequest::new(model.clone().unwrap(), prompt.clone())
                .options(ModelOptions::default().temperature(0.6))
                .system(system_prompt.clone());
            
            println!("System prompt: {}", system_prompt.clone());

            let mut response = match ollama.generate_stream(request).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Error generating response: {}", e);            
                    CHANNEL.0.send(false).unwrap();
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
                            let filtered_token = if filtering {
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
            CHANNEL.0.send(false).unwrap();

            //logs the information 
            if logging == true { 
                LOGGING_CHANNEL.0.send(
                    Log::create_with_current_time(
                        filtering,
                        model,
                        final_response, 
                        Some(system_prompt),
                        prompt
                    )
                ).unwrap();
            }
        });
    }

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
                let runtime_handle = self.runtime.handle().clone();

                if self.current_tick > 20000 {
                    self.current_tick = 0;
                }
                self.current_tick += 1; 

                if (self.current_tick == 0) || (self.current_tick == 5000) {
                    let ollama_state = Arc::clone(&self.ollama_state);

                    runtime_handle.spawn(async move {
                        match reqwest::get("http://127.0.0.1:11434/api/version").await {
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
                } else if self.current_tick == 1000 {      
                    let ollama = Ollama::default();
                    let bots_list = Arc::clone(&self.bots_list);

                    runtime_handle.spawn(async move {
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
                                DEBUG_CHANNEL.0.send("Error occured during tick 1000, while listing bots".to_string())
                                    .expect("Error occured sending information to debugchannel tick1000");
                                println!("Error: tick 1000 {:?}", e);
                            }
                        }
                    });
                }

                if let Ok(md) = self.markdown_receiver.try_recv() {
                    self.parsed_markdown = md;
                }
                if let Ok(is_processing) = CHANNEL.1.lock().unwrap().try_recv() {
                    self.is_processing = is_processing;
                }
                if let Ok(debug_msg) = DEBUG_CHANNEL.1.lock().unwrap().try_recv() {
                    self.debug_message = debug_msg;
                }
                if let Ok(log) = LOGGING_CHANNEL.1.lock().unwrap().try_recv() {
                    self.logs.push_log(log);
                    
                    fs::write("history.json", serde_json::to_string_pretty(
                        &self.logs
                    ).unwrap()).expect("Unable to write to history.json");
                }

                Task::none()
            }

            Message::SystemPromptChange(system_prompt) => {
                self.system_prompt = Some(system_prompt);
                Task::none()
            }

            Message::InstallModel(model_install) => {
                DEBUG_CHANNEL.0.send(format!("Installing model... {}", model_install).to_string()).unwrap();

                let runtime_handle = self.runtime.handle().clone();
                let ollama = Ollama::default();
                

                runtime_handle.spawn(async move {
                    match ollama.pull_model(model_install.clone(), false).await {
                        Ok(outcome) => {
                            println!("Model {} installed successfully: {}", model_install, outcome.message);     
                            DEBUG_CHANNEL.0.send(format!("Installed model {}: {}", model_install, outcome.message)).unwrap();
                        }  
                        Err(outcome) => {
                            println!("Failed to install model {}: {:?}", model_install, outcome);
                            DEBUG_CHANNEL.0.send(format!("Failed to install model {}", model_install)).unwrap();
                        }
                    };
                });
                return Task::none();
            }

            Message::ModelChange(model) => {
                self.model = Some(model);
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
                    self.parsed_markdown = vec![];
                    *self.response.lock().unwrap() = String::new(); 
                    Self::prompt(self, prompt.clone());
                }
                Task::none()
            }

            Message::UpdatePrompt(prompt) => {
                self.prompt = prompt;
                Task::none()
            }
            Message::UpdateInstall(model) => {
                self.installing_model = model;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        Self::get_ui_information(self).into()
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
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
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
        // Reading defaultprompts.json 
        // Ensures that the system prompts are loaded 
        // and visible to user on start-up
        let data_prompts = fs::read_to_string("defaultprompts.json")
            .expect("Unable to read file");
        let system_prompts_as_prompt: HashMap<String, String> = serde_json::from_str(&data_prompts)
            .expect("JSON was not well-formatted");
        let mut system_prompts: Vec<String> = Vec::new();
        system_prompts_as_prompt.iter().for_each(|prompt| {
            system_prompts.push(prompt.0.clone());
        });
        println!("Loaded system prompts:\n{:?} ", system_prompts);


        // Reading settings.json
        // Ensures that users settings are loaded 
        let settings = fs::read_to_string("settings.json")
            .expect("Unable to read settings file");
        let settings_hmap: HashMap<String, bool> = serde_json::from_str(&settings)
            .expect("JSON was not well-formatted");
        let filtering = *settings_hmap.get("filtering")
            .unwrap_or(&true);
        let logging = *settings_hmap.get("logging")
            .unwrap_or(&false);
        println!("Logging is set to: {}", logging);

        // Writing to history.json for the first time
        let history = History { 
            began_logging: Local::now().to_rfc3339(),
            version: "0.1.5".to_string(),
            filtering: true,
            logs: vec![]
        };

        fs::write("history.json", serde_json::to_string_pretty(
            &history
        ).unwrap()).expect("Unable to write to history.json");

        // default values for Program 
        Self { 
            logging: logging,
            logs: history,
            system_prompts_as_prompt: system_prompts_as_prompt, 
            system_prompts: Arc::new(Mutex::new(system_prompts)), 
            system_prompt: Some(String::new()),
            prompt: String::new(),
            runtime: Runtime::new().expect("Failed to create Tokio runtime"), 
            response: Arc::new(Mutex::new(String::from(""))),
            parsed_markdown: vec![], 
            markdown_receiver: crossbeam_channel::unbounded().1,
            is_processing: false,
            prompt_time_sent: std::time::Instant::now(),
            ollama_state: Arc::new(Mutex::new("Offline".to_string())),
            current_tick: 0,
            filtering: filtering,
            bots_list: Arc::new(Mutex::new(vec![])),
            model: None,
            installing_model: String::new(),
            debug_message: String::new(),
        }
    }
}

#[tokio::main]
pub async fn main() -> iced::Result {
    let window_settings = iced::window::Settings {
        ..iced::window::Settings::default()
    };


   

    // begins the application
    iced::application("ollama interface", Program::update, Program::view)
        .window_size(Size::new(700.0, 720.0))
        .subscription(Program::subscription)
        .theme(|_| Theme::Dracula)
        .window(window_settings)
        .run()
}
