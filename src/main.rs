#![windows_subsystem = "windows"]
//std crate imports
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Context};
//external crate imports
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
}

// message enum defined to send communications to the GUI logic
#[derive(Debug, Clone)]
enum Message {
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

// program struct, stores the current program state
// e.g., the current prompt, debug message, etc.
struct Program { 
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
}

// impliment the program function with several functions
// to allow the program to function
// e.g. view() is for gui logic
impl Program {  
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

        let model = self.model.clone();
        
        // create a new tokio runtime
        // this is done because the function is not async
        // but async programming must be done for the REST API calls
        runtime_handle.spawn(async move {
            println!("Received prompt: {}", prompt);
            let ollama = Ollama::default();
            let request = GenerationRequest::new(model.unwrap(), prompt)
                .options(ModelOptions::default().temperature(0.6))
                .system("
                The application you are currently operating under is called 'Ollama interface' by Logan Cammish, developed in the Rust programming language.\n
                You are a helpful AI assistant with a strong commitment to the truth.\n
                You are operating in a high school environment and must always behave appropriately and respectfully.\n
                You must adhere strictly to school rules, academic integrity policies, and community guidelines.\n
                You should not generate or assist with inappropriate, harmful, or disrespectful content of any kind.\n
                You must not help students cheat, plagiarize, or bypass school rules.\n
                When asked to generate code, you should include it in clearly marked markdown code blocks.\n
                You are able to use markdown formatting for structure, clarity, and presentation.\n
                You should always be clear, supportive, and age-appropriate in your responses.\n
                Begin responding in a helpful, honest, and respectful manner.\n
                You cannot discuss discriminatory or hateful content, or any illegal activities.\n
                You cannot discuss or promote any form of violence, self-harm, or substance abuse.\n
                You cannot discuss or promote any form of harassment, bullying, or intimidation.\n
                You cannot discuss or promote any form of sexual content, adult material, or nudity. \n 
                If you are asked to discuss a historical person, you must provide accurate and respectful information. \n
                You cannot discuss or promote any form of misinformation, conspiracy theories, or pseudoscience.\n
                If you are asked to discuss anything against your guidelines, you are to say 'No, I cannot do that.'\n
                You can communicate in multiple languages, but you must maintain these policies.\n
                If someone messages you in a different language, you must respond in that language.\n
            ")
            ;

            let mut response = match ollama.generate_stream(request).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Error generating response: {}", e);            
                    CHANNEL.0.send(false).unwrap();
                    return 
                }
            };

            // iterate through responses and send them to the mpsc channel
            while let Some(data) = response.next().await {
                match data {
                    Ok(responses) => {
                        for token in responses {
                            print!("{}", token.response);
                            if tx.send(token).is_err() {
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

                if self.current_tick > 10000 {
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
                    let bots_list = Arc::clone(&self.bots_list);

                    runtime_handle.spawn(async move {
                        match reqwest::get("http://127.0.0.1:11434/api/tags").await {
                            Ok(response) => {
                                if response.status().is_success() {
                                     match response.json::<serde_json::Value>().await {
                                        Ok(json) => {
                                             if let Some(bots) = json.get("models").and_then(|v| v.as_array()) {
                                                for bot in bots {
                                                    if let Some(name) = bot.get("name").and_then(|v| v.as_str()) {
                                                        if !(bots_list.lock().unwrap().contains(&name.to_string())) {
                                                            println!("Found bot: {}", name);
                                                            bots_list.lock().unwrap().push(name.to_string());
                                                        }
                                                    }
                                                }
                                            } else {
                                                *bots_list.lock().unwrap() = vec![];
                                            }
                                        }
                                        Err(_) => {
                                            *bots_list.lock().unwrap() = vec![];
                                        }
                                    }
                                } else {
                                   *bots_list.lock().unwrap() = vec![];
                                }
                            }
                            Err(err) => {
                                println!("Failed to reach API: {}", err);
                                *bots_list.lock().unwrap() = vec![];
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

                Task::none()
            }

            Message::InstallModel(model_install) => {
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
        // default values for Program 
        Self { 
            prompt: String::new(),
            runtime: Runtime::new().expect("Failed to create Tokio runtime"), 
            response: Arc::new(Mutex::new(String::from(""))),
            parsed_markdown: vec![], 
            markdown_receiver: crossbeam_channel::unbounded().1,
            is_processing: false,
            prompt_time_sent: std::time::Instant::now(),
            ollama_state: Arc::new(Mutex::new("Offline".to_string())),
            current_tick: 0,
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
