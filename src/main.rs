use iced::{Element, Size, Subscription, Theme};
use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use regex::Regex; 

mod gui; 

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Prompt(String)
} 

struct Program { 
    prompt: String 
}

impl Program {  
    fn update(&mut self, message: Message) {
        match message { 
            Message::Prompt(prompt) => {
                self.prompt = prompt;
            }
        }
    }

    fn view(&self) -> Element<Message> {
        Self::get_ui_information(self).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        return Subscription::none(); 
    }
}

impl Default for Program {
    fn default() -> Self {
        Self { 
            prompt: String::new()
        }
    }
}

#[tokio::main]
pub async fn main() -> iced::Result {
    let prompt = "top ten aura moments";
    let ollama = Ollama::default();
    
    
    // let request = GenerationRequest::new(String::from("deepseek-r1:1.5b"), prompt)
    //     .options(ModelOptions::default())
    //     .system("You are a helpful AI assistant who has a strong devotion to the truth.\nYou are in a school environment, and you are to adhere to certain policies related to this. Begin talking now.");
    // match ollama.generate(request).await {
    //     Ok(response) => { 
    //         let data = response.response;
    //         let re = Regex::new(r"(?s)<think>.*?</think>").unwrap(); // Remove instances of <think> tags with regex
    //         let cleaned_response = re.replace_all(&data, "").trim().to_string();
    //         println!("Response: {:?}", cleaned_response)
    //     },
    //     Err(err) => eprintln!("Generation Error: {:?}", err),
    // }

    let window_settings = iced::window::Settings {
        ..iced::window::Settings::default()
    };


    iced::application("ollama interface", Program::update, Program::view)
        .window_size(Size::new(700.0, 720.0))
        .subscription(Program::subscription)
        .theme(|_| Theme::TokyoNight)
        .window(window_settings)
        .run()
    
}
