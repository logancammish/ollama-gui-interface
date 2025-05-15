use iced::widget::{self, container};

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
        .on_input(|input| { Message::UpdatePrompt(input) });

       
        return container(
            widget::column![
                    widget::row!(prompt),
                    widget::row!(iced::widget::button("Enter").on_press(Message::Prompt(self.prompt.clone()))),
                    widget::row!(iced::widget::text(format!("Response: {}", self.response.clone())))
                ] 
            ).into();
    }
}