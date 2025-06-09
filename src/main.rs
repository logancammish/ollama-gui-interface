//#![windows_subsystem = "windows"]
//std crate imports
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Context};
//external crate imports
use chrono::Local;
use futures::{channel, Stream};
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
use serde::Serialize;
use std::fs;
use rustrict::Censor;
//local file imports
mod gui; 


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

#[derive(Serialize, Clone)]
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

#[derive(Serialize, Clone)]
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

struct AppState { 
    filtering: bool, 
    logs: History, 
    logging: bool, 
    ollama_state: Arc<Mutex<String>>,
    bots_list: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone)]
struct SystemPrompt {
    system_prompts_as_hashmap: HashMap<String, String>,
    system_prompts_as_vec: Arc<Mutex<Vec<String>>>,
    system_prompt: Option<String>,
}

impl SystemPrompt { 
    fn get_current(program: &Program) -> Option<String> { 
        let system_prompt: Self = program.system_prompt.clone(); 

        if system_prompt.system_prompts_as_hashmap.get(&system_prompt.system_prompt.clone().unwrap()).is_some() {
            return program.system_prompt.system_prompts_as_hashmap.get(&system_prompt.system_prompt.clone().unwrap()).cloned();
        } else { 
            println!("system prompt is None");
            return None; 
        }
    }
}

struct UserInformation {
    model: Option<String> ,
}

#[derive(Clone)]
struct Channels {
    markdown_channel_reciever: crossbeam_channel::Receiver<Vec<markdown::Item>>,
    debug_channel: Arc<Mutex<(std::sync::mpsc::Sender<String>, std::sync::mpsc::Receiver<String>)>>,
    debounce_channel: Arc<Mutex<(std::sync::mpsc::Sender<bool>, std::sync::mpsc::Receiver<bool>)>>,
    logging_channel: Arc<Mutex<(std::sync::mpsc::Sender<Log>, std::sync::mpsc::Receiver<Log>)>>,
}

struct Response { 
    response_as_string: Arc<Mutex<String>>,
    parsed_markdown: Vec<markdown::Item>,
}

struct Prompt { 
    prompt_time_sent: std::time::Instant,
    prompt: String
}

// program struct, stores the current program state
// e.g., the current prompt, debug message, etc.
struct Program { 
    runtime: Runtime,
    is_processing: bool,
    current_tick: i32,
    installing_model: String,
    debug_message: String,

    system_prompt: SystemPrompt,
    app_state: AppState, 
    channels: Channels, 
    user_information: UserInformation,
    response: Response,
    prompt: Prompt
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
        if self.user_information.model == None {
            self.channels.debounce_channel.lock().unwrap().0.send(false).unwrap();
            self.channels.debug_channel.lock().unwrap().0.send("Model selected is invalid, have you selected a model?".to_string()).unwrap();
            println!("Model is None");
            return; 
        }

        self.prompt.prompt_time_sent = std::time::Instant::now();

        let (markdown_sender, markdown_receiver) = crossbeam_channel::unbounded();
        self.channels.markdown_channel_reciever = markdown_receiver;
        //let runtime_handle = self.runtime.handle().clone();
        let response_arc = Arc::clone(&self.response.response_as_string);
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

        let system_prompt: Option<String> = SystemPrompt::get_current(&self);
        if system_prompt.is_none() { 
            self.channels.debounce_channel.lock().unwrap().0.send(false).unwrap();
            self.channels.debug_channel.lock().unwrap().0.send("System prompt not selected or is invalid".to_string()).unwrap();
            return;
        }
        
        let model = self.user_information.model.clone(); 
        let logging = self.app_state.logging.clone(); 
        let filtering = self.app_state.filtering.clone(); 
        let channels = self.channels.clone();
        // create a new tokio runtime
        // this is done because the function is not async
        // but async programming must be done for the REST API calls
        self.runtime.spawn(async move {           
            println!("Received prompt: {}", prompt.clone());

            let system_prompt = system_prompt.unwrap(); 
            let ollama = Ollama::default();
            let request = GenerationRequest::new(model.clone().unwrap(), prompt.clone())
                .options(ModelOptions::default().temperature(0.6))
                .system(system_prompt.clone());
            
            println!("System prompt: {}", system_prompt.clone());

            let mut response = match ollama.generate_stream(request).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Error generating response: {}", e);            
                    channels.debounce_channel.lock().unwrap().0.send(false).unwrap();
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
            channels.debounce_channel.lock().unwrap().0.send(false).unwrap();

            //logs the information 
            if logging == true { 
                channels.logging_channel.lock().unwrap().0.send(
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
                //let runtime_handle = self.runtime.handle().clone();

                if self.current_tick > 20000 {
                    self.current_tick = 0;
                }
                self.current_tick += 1; 

                if (self.current_tick == 0) || (self.current_tick == 5000) {
                    let ollama_state = Arc::clone(&self.app_state.ollama_state);

                    self.runtime.spawn(async move {
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
                                channels.debug_channel.lock().unwrap().0.send("Error occured during tick 1000, while listing bots".to_string())
                                    .expect("Error occured sending information to debugchannel tick1000");
                                println!("Error: tick 1000 {:?}", e);
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

                    fs::write("history.json", serde_json::to_string_pretty(
                        &self.app_state.logs
                    ).unwrap()).expect("Unable to write to history.json");
                }

                Task::none()
            }

            Message::SystemPromptChange(system_prompt) => {
                self.system_prompt.system_prompt = Some(system_prompt);
                Task::none()
            }

            Message::InstallModel(model_install) => {
                self.channels.debug_channel.lock().unwrap().0.send(format!("Installing model... {}", model_install).to_string()).unwrap();

                //let runtime_handle = self.runtime.handle().clone();
                let ollama = Ollama::default();
                let channels = self.channels.clone();

                self.runtime.spawn(async move {
                    match ollama.pull_model(model_install.clone(), false).await {
                        Ok(outcome) => {
                            println!("Model {} installed successfully: {}", model_install, outcome.message);     
                            channels.debug_channel.lock().unwrap().0.send(format!("Installed model {}: {}", model_install, outcome.message)).unwrap();
                        }  
                        Err(outcome) => {
                            println!("Failed to install model {}: {:?}", model_install, outcome);
                            channels.debug_channel.lock().unwrap().0.send(format!("Failed to install model {}", model_install)).unwrap();
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
            version: "0.2.0".to_string(),
            filtering: true,
            logs: vec![]
        };

        fs::write("history.json", serde_json::to_string_pretty(
            &history
        ).unwrap()).expect("Unable to write to history.json");

        // default values for Program 
        // Self { 
        //     logging: logging,
        //     logs: history,
        //     system_prompts_as_prompt: system_prompts_as_prompt, 
        //     system_prompts: Arc::new(Mutex::new(system_prompts)), 
        //     system_prompt: Some(String::new()),
        //     prompt: String::new(),
        //     runtime: Runtime::new().expect("Failed to create Tokio runtime"), 
        //     response: Arc::new(Mutex::new(String::from(""))),
        //     parsed_markdown: vec![], 
        //     markdown_receiver: crossbeam_channel::unbounded().1,
        //     is_processing: false,
        //     prompt_time_sent: std::time::Instant::now(),
        //     ollama_state: Arc::new(Mutex::new("Offline".to_string())),
        //     current_tick: 0,
        //     filtering: filtering,
        //     bots_list: Arc::new(Mutex::new(vec![])),
        //     model: None,
        //     installing_model: String::new(),
        //     debug_message: String::new(),
        // }
        Self { 
            runtime: Runtime::new().expect("Failed to create Tokio runtime"), 
            is_processing: false,
            current_tick: 0,
            installing_model: String::new(),
            debug_message: String::new(),
            system_prompt: SystemPrompt { 
                system_prompts_as_hashmap: system_prompts_as_prompt, 
                system_prompts_as_vec: Arc::new(Mutex::new(system_prompts)), 
                system_prompt: Some(String::new())
            },
            channels: Channels { 
                markdown_channel_reciever: crossbeam_channel::unbounded().1, 
                debug_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<String>())), 
                debounce_channel: Arc::new(Mutex::new(std::sync::mpsc::channel::<bool>())), 
                logging_channel:  Arc::new(Mutex::new(std::sync::mpsc::channel::<Log>()))
            },
            user_information: UserInformation { 
                model: None, 
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
