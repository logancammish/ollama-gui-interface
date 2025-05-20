use std::pin::Pin;
use std::process::Command;
use std::task::{Poll, Context};

use futures::Stream;
use iced::keyboard::key::Named;
use iced::{ clipboard, keyboard, Element, Size, Subscription, Task, Theme};
use iced_widget::markdown;
use ollama_rs::generation::chat::request;
use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use regex::Regex;
use tokio::runtime::Runtime; 
use iced_native::subscription::Recipe;
use futures::stream::StreamExt;
use webbrowser;

mod gui; 

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Prompt(String),
    UpdatePrompt(String),
    None,
    KeyPressed(keyboard::Key),
    KeyReleased(keyboard::Key),
    Tick,
    CopyPressed(String),
    InstallationPrompt
} 

struct Program { 
    prompt: String,
    runtime: Runtime,
    response: String,
    parsed_markdown: Vec<markdown::Item>,
}

impl Program {  
    fn prompt(prompt: String) -> impl std::future::Future<Output = Result<String, Box<dyn std::error::Error>>> + Send { 
        async move {
            println!("Received prompt: {}", prompt);
            let ollama = Ollama::default();
            let request = GenerationRequest::new(String::from("deepseek-r1:1.5b"), prompt)
                .options(ModelOptions::default()
                    .temperature(0.6)
                )
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
                ");
            let response = ollama.generate(request).await?;
            let data = response.response;
            let re = Regex::new(r"(?s)<think>.*?</think>").unwrap();
            let cleaned_response = re.replace_all(&data, "").trim().to_string();
            println!("Generated response: {} (Clean) \n {} (Unclean)", cleaned_response, data);
            Ok(cleaned_response)
        }
    }

    fn request_response(&mut self, prompt: String)  { 
        println!("Prompt: {}", prompt);
        let runtime_handle = self.runtime.handle().clone();
        let (response_sender, response_receiver) = std::sync::mpsc::channel();
        runtime_handle.spawn(async move {
            if let Ok(response) = Program::prompt(prompt).await {
                let _ = response_sender.send(response);
            }
        });
        if let Ok(response) = response_receiver.recv() {
            println!("{}", response);
            self.response = response;
        }
        self.parsed_markdown = markdown::parse(&self.response).collect();
    }

    fn update(&mut self, message: Message) -> Task<Message>  {
        match message { 
            Message::None => {
                Task::none()
            }
            Message::Tick => { 
                //println!("Program tick")
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

            Message::KeyPressed(key) => {   
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


            Message::KeyReleased(key) => { 
                Task::none()
            }

            Message::Prompt(prompt) => {
                Self::request_response(self, prompt.clone());
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
            response: String::from("Responses will appear here"),
            parsed_markdown: vec![],
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
