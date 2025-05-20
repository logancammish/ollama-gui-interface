use iced::{alignment, clipboard, widget::{self, container}, Alignment, Length, Theme};
use iced_widget::markdown;

use crate::{Program, Message};


impl Program {
    
    pub fn get_ui_information(&self) -> iced::widget::Container<Message> { 
        let prompt = iced::widget::TextInput::<Message>::new(
            "Prompt",
            &self.prompt,
        )
            .padding(10)
            .size(20)
            .width(iced::Length::Fill)
            .on_submit(Message::Prompt(self.prompt.clone()))
            .on_input(|input| { Message::UpdatePrompt(input) });

               
        return container(
            widget::column![
                    // Output from the model
                    container( 
                        markdown::view(
                            &self.parsed_markdown,
                            markdown::Settings::default(),
                            markdown::Style::from_palette(Theme::Dracula.palette())
                        ).map(|_| Message::None)
                    ).align_x(alignment::Horizontal::Left),
                    // Copy button
                    widget::row!(
                        widget::button("Copy")
                            .on_press(Message::CopyPressed(self.response.clone())  
                        )  
                    ).width(Length::Fill),   
                    // Input prompt
                    widget::row!(prompt),
                    // Enter button
                    container(
                        widget::row!(iced::widget::button("Enter").on_press(Message::Prompt(self.prompt.clone()))),
                    ).align_x(alignment::Horizontal::Right),
                    // Installation
                    container( 
                        widget::button("Installation")
                        .on_press(Message::InstallationPrompt)
                    )
                    
                ]
            ).into();
    }
}