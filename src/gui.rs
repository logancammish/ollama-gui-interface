use iced::{alignment, widget::{self, container}, Length, Theme};
use iced_widget::{markdown, Space};

use crate::{Program, Message};


impl Program {
    pub fn get_ui_information(&self) -> iced::widget::Container<Message> { 
        let bots_list = self.app_state.bots_list.lock().unwrap().clone();
        let prompts_list = self.system_prompt.system_prompts_as_vec.lock().unwrap().clone();

       // let parsed_markdown = self.parsed_markdown.clone(); // Arc<Mutex<_>>
        let prompt = iced::widget::TextInput::<Message>::new(
            "Prompt",
            &self.prompt.prompt,
        )
            .padding(10)
            .size(20)
            .width(iced::Length::Fill)
            .on_submit(Message::Prompt(self.prompt.prompt.clone()))
            .on_input(|input| { Message::UpdatePrompt(input) });
        let model_install = iced::widget::TextInput::<Message>::new(
            "Model name",
            &self.installing_model,
        )
            .padding(10)
            .size(7)
            .width(iced::Length::Fixed(270.0))
            .on_submit(Message::InstallModel(self.installing_model.clone()))
            .on_input(|input| { Message::UpdateInstall(input) });
        let local_ollamastate =  self.app_state.ollama_state.lock().unwrap().clone();

               
        return container(
            widget::column![
                    // Output from the model
                    widget::scrollable( 
                        markdown::view(
                            &self.response.parsed_markdown,
                            markdown::Settings::default(),
                            markdown::Style::from_palette(Theme::Dracula.palette())
                        ).map(|_| Message::None)
                    ).height(Length::Fixed(320.0)),
                    // Copy button
                    widget::row!(
                        widget::button("Copy")
                            .on_press(Message::CopyPressed(self.response.response_as_string.lock().unwrap().clone())
                        )  
                    ).width(Length::Fill),   
                    Space::with_height(Length::Fixed(10.0)),
                    // Input prompt
                    widget::row!(prompt),
                    Space::with_height(Length::Fixed(10.0)),
                    // Enter button
                    container(
                        widget::row!(iced::widget::button("Enter").on_press(Message::Prompt(self.prompt.prompt.clone()))),
                    ).align_x(alignment::Horizontal::Right),
                    Space::with_height(Length::Fixed(20.0)),
                    // Installation
                    {
                        if local_ollamastate == "Offline" {
                            widget::row!( 
                                widget::text("To install Ollama, click "),
                                widget::button("here.")
                                .on_press(Message::InstallationPrompt)
                            )
                        } else {
                            widget::row!()
                        }
                    },
                    Space::with_height(Length::Fixed(10.0)),
                    // Show if ollama is detected as online
                    container( 
                        widget::text(format!("Ollama is {}.", local_ollamastate))
                    ),
                    // Choose bot / enable/disable thinking
                    Space::with_height(Length::Fixed(10.0)),
                    widget::row!(
                        widget::text("Select model:"),
                        Space::with_width(Length::Fixed(280.0)),
                        widget::text("Thinking (only on applicable models):"),
                        Space::with_width(Length::Fixed(5.0)),
                        widget::checkbox(
                            "",
                            self.user_information.think,
                        ).on_toggle(|_| Message::ToggleThinking),
                    ),
                    Space::with_height(Length::Fixed(5.0)),
                    widget::pick_list(
                        bots_list,
                        self.user_information.model.clone(),
                        Message::ModelChange,
                    ),
                    // Choose sys prompt 
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("Select system prompt:"),
                    Space::with_height(Length::Fixed(5.0)),
                    widget::pick_list(
                        prompts_list,
                        self.system_prompt.system_prompt.clone(),
                        Message::SystemPromptChange,
                    ), 
                    Space::with_height(Length::Fixed(10.0)),
                    // Install model / 
                    widget::row!(
                        widget::text("Model to install (e.g. llama3.2:3b): "),
                        model_install,
                    ),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text(self.debug_message.clone())

                    
                ]
            ).padding(10).into();
    }
}