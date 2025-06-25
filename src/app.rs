use std::{collections::HashMap, sync::{Arc, Mutex}};

use chrono::Local;
use iced_widget::markdown;
use serde::Serialize;
use crate::{Program};


#[derive(Clone)]
pub struct DebugMessage{ 
    pub message: String, 
    pub is_error: bool 
}

// log struct allows for easy JSON creation 
#[derive(Serialize, Clone)]
pub struct Log { 
    pub filtering: bool,
    pub time: String, 
    pub prompt: String, 
    pub response: Vec<String>, 
    pub model: Option<String>, 
    pub systemprompt: Option<String>
}

impl Log { 
    // this function will create a new Log with the information specified on the current time
    pub fn create_with_current_time(filtering: bool, model: Option<String>, response: Vec<String>, systemprompt: Option<String>, prompt: String) -> Self {
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

// History struct allows for easy JSON creation 
#[derive(Serialize, Clone)]
pub struct History { 
    pub began_logging: String, 
    pub version: String, 
    pub filtering: bool, 
    pub logs: Vec<Log>
}
impl History { 
    // will push a Log to the History.logs
    pub fn push_log(&mut self, log: Log) {
        self.logs.push(log);
    }
}

#[derive(Clone, Debug)]
pub struct CurrentChat { 
    pub chats: Vec<String>
}
impl CurrentChat { 
    fn push_chat(&mut self, chat: String) {
        self.chats.push(chat);
    }
    fn generate_new_message(user_message: String, bot_response: String) -> String {
        return format!("User: {}\nAI Language Model: {}", user_message, bot_response);
    }
    pub fn generate_and_push(&mut self, user_message: String, bot_response: String) {
        let new_message = Self::generate_new_message(user_message, bot_response);
        self.push_chat(new_message);
    }
    pub fn unravel(&self) -> String {
        self.chats.join("\n")
    }
}


// AppState keeps information on certain important information
pub struct AppState { 
    pub filtering: bool, 
    pub logs: History, 
    pub logging: bool, 
    pub ollama_state: Arc<Mutex<String>>,
    pub bots_list: Arc<Mutex<Vec<String>>>,
    pub show_info_popup: bool,
    pub dark_mode: bool,
}

// SystemPrompt saves the current system prompts and the currently selected system prompt
#[derive(Clone)]
pub struct SystemPrompt {
    pub system_prompts_as_hashmap: HashMap<String, String>,
    pub system_prompts_as_vec: Arc<Mutex<Vec<String>>>,
    pub system_prompt: Option<String>,
}

impl SystemPrompt { 
    // gets the currently selected system prompt
    pub fn get_current(program: &Program) -> Option<String> {
        let system_prompt: SystemPrompt = program.system_prompt.clone(); 
        let system_prompt_as_string: String = match system_prompt.system_prompt { 
            Some(system_prompt) => system_prompt,
            None => {
                println!("Error getting system prompt");
                Channels::send_request_to_channel(Arc::clone(&program.channels.debug_channel), 
                    DebugMessage {
                         message: "Could not get system prompt, is it selected?".to_string(), 
                         is_error: true 
                    }
                );
                Channels::send_request_to_channel(Arc::clone(&program.channels.debounce_channel), false);
                return None; 
            }
        }; 

        if system_prompt.system_prompts_as_hashmap.get(&system_prompt_as_string).is_some() {
            return system_prompt.system_prompts_as_hashmap.get(&system_prompt_as_string).cloned();
        } else { 
            println!("system prompt is None");
            Channels::send_request_to_channel(Arc::clone(&program.channels.debug_channel), 
            DebugMessage {
                         message: "Could not get system prompt, is it selected?".to_string(), 
                         is_error: true 
                    }
            );
            Channels::send_request_to_channel(Arc::clone(&program.channels.debounce_channel), false);
            return None; 
        }
    }
}

// UserInformation saves certain important information about the program specific to the current user 
#[derive(Clone)]
pub struct UserInformation {
    pub model: Option<String>,
    pub think: bool,
    pub temperature: f32,
    pub text_size: f32,
    pub chat_history: Arc<Mutex<CurrentChat>>,
    pub current_chat_history_enabled: bool,
    pub viewing_chat_history: bool
}

/// Channels
/// These channels are either crossbeam or mpsc channels designed for easy communication 
/// between runtimes. 
/// markdown_channel_reciever: Crossbeam channel reciever for markdown content to the GUI
/// debug_channel: mpsc channel for sending debug information to GUI
/// debounce_channel: mpsc channel for preventing certain things from occuring at the same time
/// logging_channel: mpsc channel for communication with the logging feature of the program

#[derive(Clone)]
pub struct Channels {
    pub markdown_channel_reciever: crossbeam_channel::Receiver<Vec<markdown::Item>>,
    pub debug_channel: Arc<Mutex<(std::sync::mpsc::Sender<DebugMessage>, std::sync::mpsc::Receiver<DebugMessage>)>>,
    pub debounce_channel: Arc<Mutex<(std::sync::mpsc::Sender<bool>, std::sync::mpsc::Receiver<bool>)>>,
    pub logging_channel: Arc<Mutex<(std::sync::mpsc::Sender<Log>, std::sync::mpsc::Receiver<Log>)>>,
}

impl Channels {
    pub fn send_request_to_channel<T: Send + Clone> (channel: Arc<Mutex<(std::sync::mpsc::Sender<T>, std::sync::mpsc::Receiver<T>)>>, message: T) {        
        match channel.lock() {
            Ok(channel) => {
                if let Err(e) = channel.0.send(message) {
                    eprintln!("Failed to send: {}", e);
                }
            }
            Err(e) => {                
                eprintln!("Failed to send: {}", e);
            }
        }
    }
}
// Response saves the current response as both parsed markdown and a string
pub struct Response { 
    pub response_as_string: Arc<Mutex<String>>,
    pub parsed_markdown: Vec<markdown::Item>,
}

// Prompt saves the current prompt and time sent 
pub struct Prompt { 
    pub prompt_time_sent: std::time::Instant,
    pub prompt: String
}