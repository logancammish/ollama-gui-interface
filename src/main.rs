use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Context};

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

mod gui; 

lazy_static! {
    static ref CHANNEL: (
        std::sync::mpsc::Sender<bool>,
        Arc<Mutex<std::sync::mpsc::Receiver<bool>>>
    ) = {
        let (txprocess, rxprocess) = std::sync::mpsc::channel::<bool>();
        return (txprocess, Arc::new(Mutex::new(rxprocess)));
    };
}


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
} 

struct Program { 
    prompt: String,
    runtime: Runtime,
    response: Arc<Mutex<String>>,
    parsed_markdown: Vec<markdown::Item>,
    markdown_receiver: crossbeam_channel::Receiver<Vec<markdown::Item>>,
    is_processing: bool,
}

impl Program {  
fn prompt(&mut self, prompt: String) {
    let (markdown_sender, markdown_receiver) = crossbeam_channel::unbounded();
    self.markdown_receiver = markdown_receiver;
    let runtime_handle = self.runtime.handle().clone();
    let response_arc = Arc::clone(&self.response);
    let (tx, rx) = std::sync::mpsc::channel::<GenerationResponse>();

    std::thread::spawn(move || {
        for token in rx {
            let mut resp = response_arc.lock().unwrap();
            resp.push_str(&token.response);
            let md = markdown::parse(&resp).collect();
            markdown_sender.send(md).unwrap();
        }
    });


    runtime_handle.spawn(async move {
        println!("Received prompt: {}", prompt);
        let ollama = Ollama::default();
        let request = GenerationRequest::new("llama3.2:3b".to_string(), prompt)
            .options(ModelOptions::default().temperature(0.6))
            .system("
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
                    return;
                }
            };

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
            CHANNEL.0.send(false).unwrap();
        });
            
        

    }



     //let re = Regex::new(r"(?s)<think>.*?</think>").unwrap();
            // let cleaned_response = re.replace_all(&data, "").trim().to_string();
            // println!("Generated response: {} (Clean) \n {} (Unclean)", cleaned_response, data);
            
            // let runtime_handle = self.runtime.handle().clone();
            // let (response_sender, response_receiver) = std::sync::mpsc::channel();
            // runtime_handle.spawn(async move {
            //     //if let Ok(response) = cleaned_response {
            //         let _ = response_sender.send(response);
            //     //}
            // });
            // if let Ok(response) = response_receiver.recv() {
            //     println!("{}", response);
            //     self.response = response;
            // }
            // self.parsed_markdown = markdown::parse(&self.response).collect();
    
    

    fn update(&mut self, message: Message) -> Task<Message>  {
        match message { 
            Message::None => {
                Task::none()
            }
            Message::Tick => { 
                if let Ok(md) = self.markdown_receiver.try_recv() {
                    self.parsed_markdown = md;
                }
                if let Ok(is_processing) = CHANNEL.1.lock().unwrap().try_recv() {
                    self.is_processing = is_processing;
                }

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
        }
    }

    fn view(&self) -> Element<Message> {
        Self::get_ui_information(self).into()
    }

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
        Self { 
            prompt: String::new(),
            runtime: Runtime::new().expect("Failed to create Tokio runtime"), 
            response: Arc::new(Mutex::new(String::from(""))),
            parsed_markdown: vec![], 
            markdown_receiver: crossbeam_channel::unbounded().1,
            is_processing: false
        }
    }
}

#[tokio::main]
pub async fn main() -> iced::Result {
    let window_settings = iced::window::Settings {
        ..iced::window::Settings::default()
    };


    iced::application("ollama interface", Program::update, Program::view)
        .window_size(Size::new(700.0, 720.0))
        .subscription(Program::subscription)
        .theme(|_| Theme::Dracula)
        .window(window_settings)
        .run()
}
