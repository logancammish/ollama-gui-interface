use std::sync::Arc;

use iced::{alignment, clipboard, widget::{self, container}, Alignment, Length, Theme};
use iced_widget::{markdown, Space};

use crate::{Program, Message};


impl Program {
    
    pub fn get_ui_information(&self) -> iced::widget::Container<Message> { 
        let bots_list = self.bots_list.lock().unwrap().clone();

       // let parsed_markdown = self.parsed_markdown.clone(); // Arc<Mutex<_>>
        let prompt = iced::widget::TextInput::<Message>::new(
            "Prompt",
            &self.prompt,
        )
            .padding(10)
            .size(20)
            .width(iced::Length::Fill)
            .on_submit(Message::Prompt(self.prompt.clone()))
            .on_input(|input| { Message::UpdatePrompt(input) });
        let model_install = iced::widget::TextInput::<Message>::new(
            "Model name",
            &self.installing_model,
        )
            .padding(10)
            .size(20)
            .width(iced::Length::Fixed(270.0))
            .on_submit(Message::InstallModel(self.installing_model.clone()))
            .on_input(|input| { Message::UpdateInstall(input) });

               
        return container(
            widget::column![
                    // Output from the model
                    widget::scrollable( 
                        markdown::view(
                            &self.parsed_markdown,
                            markdown::Settings::default(),
                            markdown::Style::from_palette(Theme::Dracula.palette())
                        ).map(|_| Message::None)
                    ).height(Length::Fixed(420.0)),
                    // Copy button
                    widget::row!(
                        widget::button("Copy")
                            .on_press(Message::CopyPressed(self.response.lock().unwrap().clone())
                        )  
                    ).width(Length::Fill),   
                    Space::with_height(Length::Fixed(10.0)),
                    // Input prompt
                    widget::row!(prompt),
                    Space::with_height(Length::Fixed(10.0)),
                    // Enter button
                    container(
                        widget::row!(iced::widget::button("Enter").on_press(Message::Prompt(self.prompt.clone()))),
                    ).align_x(alignment::Horizontal::Right),
                    Space::with_height(Length::Fixed(20.0)),
                    // Installation
                    widget::row!( 
                        widget::text("To install Ollama, click "),
                        widget::button("here.")
                        .on_press(Message::InstallationPrompt)
                        
                    ),
                    Space::with_height(Length::Fixed(10.0)),
                    // Show if ollama is detected as online
                    container( 
                        widget::text(format!("Ollama is {}.", self.ollama_state.lock().unwrap().clone()))
                    ),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::pick_list(
                        bots_list,
                        self.model.clone(),
                        Message::ModelChange,
                    ),
                    Space::with_height(Length::Fixed(10.0)),
                    // Install model 
                    widget::row!(
                        widget::text("Model to install (e.g. llama3.2:3b): "),
                        model_install,
                    ),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text(self.debug_message.clone())

                    
                ]
            ).into();
    }
}