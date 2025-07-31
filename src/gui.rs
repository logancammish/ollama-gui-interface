
use iced::{ alignment::{self, Horizontal}, border::Radius, widget::{self, container}, Background, Border, Color, Length, Shadow, Theme, Vector };
use iced_widget::{ container::Style, markdown, Space };

use crate::{Program, Message, GUIState, Correspondence};



impl Program {
    pub fn get_ui_information(&self, gui_state: GUIState) -> iced::widget::Container<Message> { 
        match gui_state {
            GUIState::InfoPopup => {
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
                        widget::text("You can edit the system prompts in the config/defaultprompts.json file."),
                        Space::with_height(Length::Fixed(10.0)),
                        widget::text("The file contains default examples which you can view if you want to see how it works."),
                        Space::with_height(Length::Fixed(10.0)),
                        widget::text("If logging is turned on, it will save to output/history.json."),
                        Space::with_height(Length::Fixed(10.0)),
                        widget::button("I understand").on_press(Message::ToggleInfoPopup),
                ]).padding(10).into();
            }
            // GUIState::Main 
            // let chat_history = self.user_information.chat_history.lock().unwrap().clone();
            // return container ( 
            //     widget::column![
            //         widget::scrollable(
            //             widget::text(chat_history.unravel())
            //         ).height(Length::Fill),
            //         widget::button("Go back").on_press(Message::ViewChatHistory)
            //     ]
            // ).padding(10).into();
            GUIState::Main => {
                let user_information = self.user_information.clone();
                let bots_list = self.app_state.bots_list.lock().unwrap().clone();

                let prompt = iced::widget::TextInput::<Message>::new(
                    "Prompt",
                    &self.prompt.prompt,
                )
                    .padding(10)
                    .size(20)
                    .width(iced::Length::Fill)
                    .on_submit(Message::Prompt(self.prompt.prompt.clone()))
                    .on_input(|input| { Message::UpdatePrompt(input) });
                let local_ollamastate =  self.app_state.ollama_state.lock().unwrap().clone();
                
                let chat_messages = {
                    let chat_history = self.user_information.chat_history.lock().unwrap();
                    chat_history.messages.clone()
                };

                let mut all_widgets: Vec<iced::Element<Message>> = chat_messages.iter().flat_map(|message| {
                    vec![
                        match message { 
                            Correspondence::User(text) => {
                                widget::row!(
                                    Space::with_width(Length::Fill),
                                    widget::container(
                                        widget::text(format!("{}", text))
                                        .size(user_information.text_size as u16)
                                        .align_x(Horizontal::Right)
                                        

                                    )                   
                                    .padding(10)
                                    .width(Length::Shrink)
                                    .style(|_style: &_| Style {
                                        text_color: Some(Color::from_rgb(1.0,1.0,1.0)),
                                        background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                                        border: Border { 
                                            color: Color::from_rgb(0.10, 0.10, 0.10), 
                                            width: 1.5, 
                                            radius: Radius::from(10.0)
                                        },
                                        shadow: Shadow {
                                            color: Color::from_rgb(0.05, 0.05, 0.05),
                                            offset: Vector::from([4.0, 3.0]), 
                                            blur_radius: 5.0

                                        }
                                    })
                                    .align_x(Horizontal::Right),
                                    Space::with_width(Length::Fixed(10.0)),
                                )
                                .into()
                            }
                            Correspondence::Bot(text) => {
                                widget::row!(
                                    Space::with_width(Length::Fixed(10.0)),
                                    widget::container(
                                        widget::text_input("", text.as_str())
                                            .on_input(|_| Message::None)
                                            .on_input_maybe(Some(|_| Message::None))
                                            .style(|_theme, _status| iced_widget::text_input::Style {
                                                background: Background::Color(Color::from_rgb(0.10, 0.10, 0.10)),
                                                border: Border {
                                                    color: Color::from_rgb(0.0, 0.0, 0.0),
                                                    width: 0.0,
                                                    radius: Radius::from(0.0),
                                                },
                                                icon: Color::from_rgb(0.0, 0.0, 0.0),
                                                placeholder: Color::from_rgb(0.0, 0.0, 0.0),
                                                value: Color::from_rgb(1.0, 1.0, 1.0),
                                                selection: Color::from_rgb(0.9, 0.9, 0.9),
                                            })
                                            .size(user_information.text_size as u16)
                                            .align_x(Horizontal::Left)
                                    )
                                    .align_x(Horizontal::Left)
                                    .padding(10.0)
                                    .style(|_style: &_| Style {
                                        text_color: Some(Color::from_rgb(1.0,1.0,1.0)),
                                        background: Some(Background::Color(Color::from_rgb(0.10, 0.10, 0.10))),
                                        border: Border { 
                                            color: Color::from_rgb(0.09, 0.09, 0.09), 
                                            width: 1.5, 
                                            radius: Radius::from(10.0)
                                        },
                                        shadow: Shadow {
                                            color: Color::from_rgb(0.05, 0.05, 0.05),
                                            offset: Vector::from([3.0, 4.0]), 
                                            blur_radius: 5.0

                                        }
                                    }),

                                )
                                .into()
                                
                            }
                        }, 
                        Space::with_height(Length::Fixed(10.0)).into()
                    ]
                }).collect();

                if !self.is_processing {
                    all_widgets.pop(); //remove the trailing space
                    all_widgets.pop(); //remove the current response
                }

                            
                    
                return container(
                    widget::column![
                            widget::scrollable( 
                                widget::column![
                                    widget::Column::with_children(
                                        all_widgets
                                    ),
                                    
                                    widget::row!(
                                        Space::with_width(Length::Fixed(10.0)),
                                        widget::container( 
                                            markdown::view(
                                                &self.response.parsed_markdown,
                                                markdown::Settings {
                                                    text_size: iced::Pixels(user_information.text_size),
                                                    ..markdown::Settings::default()
                                                },
                                                markdown::Style::from_palette(Theme::Dark.palette())
                                            ).map(|_| Message::None)
                                        ) 
                                        .align_x(Horizontal::Left)
                                        .padding(10.0)
                                        .style(|_style: &_| Style {
                                            text_color: Some(Color::from_rgb(1.0,1.0,1.0)),
                                            background: Some(Background::Color(Color::from_rgb(0.10, 0.10, 0.10))),
                                            border: Border { 
                                                color: Color::from_rgb(0.09, 0.09, 0.09), 
                                                width: 1.5, 
                                                radius: Radius::from(10.0)
                                            },
                                            shadow: Shadow {
                                                color: Color::from_rgb(0.05, 0.05, 0.05),
                                                offset: Vector::from([3.0, 4.0]), 
                                                blur_radius: 5.0

                                            }
                                        }),
                                    ),
                                    Space::with_height(Length::Fixed(25.0))

                                ]
                            ).spacing(iced::Pixels(5.0))
                            .height(Length::Fill),
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
                            Space::with_height(Length::Fixed(5.0)),
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
                            Space::with_height(Length::Fixed(5.0)),

                            

                                widget::checkbox(
                                    "Thinking (only for applicable models)",
                                    self.user_information.think,
                                ).on_toggle(|_| Message::ToggleThinking),
                                                        Space::with_height(Length::Fixed(10.0)),
                            widget::button("Settings").on_press(Message::ToggleSettings),

                            
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
                GUIState::Settings => {
                    let user_information = self.user_information.clone();
                    let ip = self.user_information.ip_address.clone();
                    let bots_list = self.app_state.bots_list.lock().unwrap().clone();
                    let prompts_list = self.system_prompt.system_prompts_as_vec.lock().unwrap().clone();
                    
                    return widget::container(
                        widget::column![
                             // Choose bot / enable/disable thinking
                            Space::with_height(Length::Fixed(10.0)),
                            widget::row!(
                                widget::text("Select model:"),
                                Space::with_width(Length::Fixed(280.0)),
                                
                            ),
                                                        Space::with_height(Length::Fixed(5.0)),

                            Space::with_height(Length::Fixed(5.0)),
                            widget::row!(
                                widget::pick_list(
                                    bots_list,
                                    self.user_information.model.clone(),
                                    Message::ModelChange,
                                ).width(Length::Fixed(175.0)),
                                Space::with_width(Length::Fixed(200.0)),
                            ),
                                                        Space::with_height(Length::Fixed(10.0)),

                                widget::checkbox(
                                    "Thinking (only for applicable models)",
                                    self.user_information.think,
                                ).on_toggle(|_| Message::ToggleThinking),
                                                        Space::with_height(Length::Fixed(10.0)),

                            widget::text("Model temperature (output 'randomness'): "),
                            Space::with_width(Length::Fixed(5.0)),
                            widget::slider(
                                0.0..=10.0, 
                                self.user_information.temperature.clone(),
                                Message::UpdateTemperature
                            ),
                            // Choose sys prompt 

                            Space::with_height(Length::Fixed(10.0)),
                            widget::text("Select system prompt:"),
                            Space::with_height(Length::Fixed(5.0)),
                            widget::row!( 
                                widget::pick_list(
                                    prompts_list,
                                    self.system_prompt.system_prompt.clone(),
                                    Message::SystemPromptChange,
                                ),     
                                Space::with_width(Length::Fixed(138.0)),
                            ), 
                                                        Space::with_height(Length::Fixed(10.0)),

                            
                            widget::text("Text size: "),
                            Space::with_width(Length::Fixed(5.0)),
                            widget::slider(
                                1.0..=40.0, 
                                self.user_information.text_size.clone(),
                                Message::UpdateTextSize
                            ),
                            Space::with_height(Length::Fixed(10.0)),

                            widget::row!(
                                widget::checkbox("Enable Chat History", user_information.current_chat_history_enabled)
                                    .on_toggle(|_| Message::ToggleChatHistory),
                                Space::with_width(Length::Fixed(50.0)),
                            ),
                                                        Space::with_height(Length::Fixed(10.0)),

                            widget::button("Wipe Chat History").on_press(Message::WipeChatHistory),

                            Space::with_height(Length::Fixed(10.0)),
                            
                            widget::row!(

                                widget::text(self.debug_message.clone().message).color(
                                    if self.debug_message.clone().is_error {
                                        Color::from_rgb(0.8, 0.2, 0.2)
                                    } else { 
                                        Color::from_rgb(0.1,0.8,0.1)
                                    }
                                ),
                            ),
                                                        Space::with_height(Length::Fixed(10.0)),


                            widget::button("Advanced Settings").on_press(Message::ToggleAdvancedSettings),
                            
                                                        Space::with_height(Length::Fixed(7.0)),
                            widget::button("Go back").on_press(Message::ToggleSettings),
                        ]
                    )
                    .padding(20)
                    .into();
                }
                GUIState::AdvancedSettings => {
                    let user_information = self.user_information.clone();
                    let ip = self.user_information.ip_address.clone();
                    let prompts_list = self.system_prompt.system_prompts_as_vec.lock().unwrap().clone();
                    let model_install = iced::widget::TextInput::<Message>::new(
                        "Model name",
                        &self.installing_model,
                    )
                        .padding(10)
                        .size(7)
                        .width(iced::Length::Fixed(270.0))
                        .on_submit(Message::InstallModel(self.installing_model.clone()))
                        .on_input(|input| { Message::UpdateInstall(input) });

                    let change_ip = iced::widget::TextInput::<Message>::new(
                        ip.ip.as_str(),
                        &ip.ip,
                    )
                        .padding(10)
                        .size(7)
                        .width(iced::Length::Fixed(270.0))
                        .on_submit(Message::ChangeIp(ip.ip.clone()))
                        .on_input(|input| { Message::ChangeIp(input) });

                    let change_port = iced::widget::TextInput::<Message>::new(
                        ip.port.as_str(),
                        &ip.port,
                    )
                        .padding(10)
                        .size(7)
                        .width(iced::Length::Fixed(270.0))
                        .on_submit(Message::ChangePort(ip.port.clone()))
                        .on_input(|input| { Message::ChangePort(input) });

                    return widget::container(
                        widget::column![
                            // Choose sys prompt 

                            Space::with_height(Length::Fixed(10.0)),
                            widget::text("Select system prompt:"),
                            Space::with_height(Length::Fixed(5.0)),
                            widget::row!( 
                                widget::pick_list(
                                    prompts_list,
                                    self.system_prompt.system_prompt.clone(),
                                    Message::SystemPromptChange,
                                ),     
                                Space::with_width(Length::Fixed(138.0)),
                            ), 
                            Space::with_height(Length::Fixed(10.0)),
                            // Install model / 
                            widget::row!(
                                widget::text("Model to install (e.g. llama3.2:3b): "),
                                model_install,
                            ),
                            // Change IP
                            Space::with_height(Length::Fixed(10.0)),

                            widget::row!(
                                widget::text("Change Ollama IP address: "),
                                change_ip,
                                widget::text(":"),
                                change_port
                            ),

                            Space::with_height(Length::Fixed(10.0)),
                            widget::text(format!("IP address is currently {}", 
                                format!("{}:{}", user_information.ip_address.ip, user_information.ip_address.port))),
                            Space::with_height(Length::Fixed(10.0)),


                            widget::button("Go back").on_press(Message::ToggleSettings),
                        ]
                    )
                    .padding(20)
                    .into();                
                }
        }
    }
}