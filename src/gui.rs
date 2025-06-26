use iced::{ alignment, widget::{self, container}, Color, Length, Theme };
use iced_widget::{ markdown, Space };

use crate::{Program, Message};



impl Program {
    pub fn get_ui_information(&self, info_popup: bool) -> iced::widget::Container<Message> { 
        if info_popup {
            return container(
                widget::column![
                    widget::text("Ollama GUI Interface is a simple GUI interface for Ollama, a local AI model serving tool.").size(30),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("This allows you to interact with AI models, install new models, and manage system prompts."),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("This application can be used both offline and online, and provides enchanced security compared to online models."),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("Starting to use").size(20),
                    Space::with_height(Length::Fixed(5.0)),
                    widget::text("You could start by using a model like llama3.2:3b, which is incredibly advanced for its features."),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("Start by selecting a model, selecting a system prompt, entering a prompt, and pressing Enter!"),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("Configuring the settings").size(20),
                    Space::with_height(Length::Fixed(5.0)),
                    widget::text("You can also update settings in the file config/settings.json, and the system prompts in config/defaultprompts.json."),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("When editing the system prompts, after entering the name of the prompt, the first text you add is the password (set to None if there is none) and then the actual prompt."),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("The file contains default examples which you can view if you want to see how it works."),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::text("If logging is turned on, it will save to output/history.json."),
                    Space::with_height(Length::Fixed(10.0)),
                    widget::button("I understand").on_press(Message::ToggleInfoPopup),
            ]).padding(10).into();
        } else if self.user_information.viewing_chat_history {
            let chat_history = self.user_information.chat_history.lock().unwrap().clone();
            return container ( 
                widget::column![
                    widget::scrollable(
                        widget::text(chat_history.unravel())
                    ).height(Length::Fill),
                    widget::button("Go back").on_press(Message::ViewChatHistory)
                ]
            ).padding(10).into();
        } else { 
            let user_information = self.user_information.clone();
            let bots_list = self.app_state.bots_list.lock().unwrap().clone();
            let prompts_list = self.system_prompt.system_prompts_as_vec.lock().unwrap().clone();

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
                                markdown::Settings { 
                                    text_size: iced::Pixels(user_information.text_size),
                                    ..markdown::Settings::default()
                                },
                                markdown::Style::from_palette(Theme::Dracula.palette())
                            ).map(|_| Message::None)
                        ).height(Length::Fill),
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
                        
                        // Bot list prompt
                        {
                            if bots_list.clone().is_empty() {
                                widget::row!( 
                                    widget::text("No bots were detected, you can find them "),
                                    widget::button("here.")
                                    .on_press(Message::ListPrompt)
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
                        widget::row!(
                            widget::pick_list(
                                bots_list,
                                self.user_information.model.clone(),
                                Message::ModelChange,
                            ).width(Length::Fixed(175.0)),
                            Space::with_width(Length::Fixed(200.0)),
                            widget::text("Model temperature (output 'randomness'): "),
                            Space::with_width(Length::Fixed(5.0)),
                            widget::slider(
                                0.0..=10.0, 
                                self.user_information.temperature.clone(),
                                Message::UpdateTemperature
                            )
                        ),
                        // Choose sys prompt 
                        Space::with_height(Length::Fixed(10.0)),
                        widget::row!(
                            widget::text("Select system prompt:"),
                            Space::with_width(Length::Fixed(220.0)),
                            widget::checkbox("Enable Chat History", user_information.current_chat_history_enabled)
                                .on_toggle(|_| Message::ToggleChatHistory),
                            Space::with_width(Length::Fixed(50.0)),
                            widget::button("Wipe Chat History").on_press(Message::WipeChatHistory),
                            Space::with_width(Length::Fixed(50.0)),
                            widget::button("View Chat History").on_press(Message::ViewChatHistory)
                        ),
                        Space::with_height(Length::Fixed(5.0)),
                        widget::row!( 
                            widget::pick_list(
                                prompts_list,
                                self.system_prompt.system_prompt.clone(),
                                Message::SystemPromptChange,
                            ),     
                            Space::with_width(Length::Fixed(138.0)),
                            widget::text("Text size: "),
                            Space::with_width(Length::Fixed(5.0)),
                            widget::slider(
                                1.0..=40.0, 
                                self.user_information.text_size.clone(),
                                Message::UpdateTextSize
                            ),
                        ), 
                        Space::with_height(Length::Fixed(10.0)),
                        // Install model / 
                        widget::row!(
                            widget::text("Model to install (e.g. llama3.2:3b): "),
                            model_install,
                        ),
                        Space::with_height(Length::Fixed(10.0)),
                        //Debug message
                        widget::row!(

                            widget::text(self.debug_message.clone().message).color(
                                if self.debug_message.clone().is_error {
                                    Color::from_rgb(0.8, 0.2, 0.2)
                                } else { 
                                    Color::from_rgb(0.1,0.8,0.1)
                                }
                            ),
                        ),
                        widget::button("Help me").on_press(Message::ToggleInfoPopup),


                        
                    ]
                ).padding(10).into();
        }
    }
}