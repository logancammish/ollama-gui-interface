use iced::{
    alignment::{self, Horizontal},
    border::Radius,
    widget::{self, container, Space},
    Background, Border, Color, Element, Length, Shadow, Theme, Vector,
};

use iced_selection::markdown as selectable_markdown;
use iced_widget::{container::Style, markdown};

use crate::{Correspondence, GUIState, Message, Program};

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
    )
}

fn app_bg() -> Color {
    rgb(7, 9, 14)
}

fn panel() -> Color {
    rgb(15, 19, 29)
}

fn panel_soft() -> Color {
    rgb(21, 26, 39)
}

fn panel_lifted() -> Color {
    rgb(28, 34, 49)
}

fn border_soft() -> Color {
    rgb(50, 61, 84)
}

fn border_bright() -> Color {
    rgb(82, 102, 145)
}

fn text_main() -> Color {
    rgb(240, 245, 255)
}

fn text_muted() -> Color {
    rgb(158, 170, 195)
}

fn text_faint() -> Color {
    rgb(110, 122, 148)
}

fn accent() -> Color {
    rgb(82, 140, 255)
}

fn accent_2() -> Color {
    rgb(108, 226, 209)
}

fn danger() -> Color {
    rgb(255, 92, 116)
}

fn success() -> Color {
    rgb(93, 225, 144)
}

fn warning() -> Color {
    rgb(255, 190, 94)
}

fn app_background_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(app_bg())),
        border: Border {
            color: app_bg(),
            width: 0.0,
            radius: Radius::from(0.0),
        },
        shadow: Shadow::default(),
    }
}

fn panel_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(panel())),
        border: Border {
            color: border_soft(),
            width: 1.0,
            radius: Radius::from(20.0),
        },
        shadow: Shadow {
            color: rgb(0, 0, 0),
            offset: Vector::from([0.0, 14.0]),
            blur_radius: 30.0,
        },
    }
}

fn card_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(panel_soft())),
        border: Border {
            color: border_soft(),
            width: 1.0,
            radius: Radius::from(18.0),
        },
        shadow: Shadow {
            color: rgb(0, 0, 0),
            offset: Vector::from([0.0, 8.0]),
            blur_radius: 20.0,
        },
    }
}

fn flat_card_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(panel_lifted())),
        border: Border {
            color: border_soft(),
            width: 1.0,
            radius: Radius::from(16.0),
        },
        shadow: Shadow::default(),
    }
}

fn input_shell_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(rgb(12, 16, 25))),
        border: Border {
            color: rgb(44, 55, 79),
            width: 1.0,
            radius: Radius::from(18.0),
        },
        shadow: Shadow {
            color: rgb(0, 0, 0),
            offset: Vector::from([0.0, 8.0]),
            blur_radius: 18.0,
        },
    }
}

fn user_bubble_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(rgb(48, 93, 190))),
        border: Border {
            color: rgb(88, 139, 250),
            width: 1.0,
            radius: Radius::from(18.0),
        },
        shadow: Shadow {
            color: rgb(0, 0, 0),
            offset: Vector::from([0.0, 8.0]),
            blur_radius: 18.0,
        },
    }
}

fn bot_bubble_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(rgb(23, 28, 42))),
        border: Border {
            color: rgb(54, 66, 94),
            width: 1.0,
            radius: Radius::from(18.0),
        },
        shadow: Shadow {
            color: rgb(0, 0, 0),
            offset: Vector::from([0.0, 8.0]),
            blur_radius: 18.0,
        },
    }
}

fn chip_style(color: Color) -> impl Fn(&Theme) -> Style {
    move |_theme: &Theme| Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(rgb(20, 25, 38))),
        border: Border {
            color,
            width: 1.0,
            radius: Radius::from(999.0),
        },
        shadow: Shadow::default(),
    }
}

fn danger_zone_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(rgb(34, 22, 30))),
        border: Border {
            color: rgb(118, 56, 74),
            width: 1.0,
            radius: Radius::from(18.0),
        },
        shadow: Shadow::default(),
    }
}

fn brighten(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r + amount).min(1.0),
        g: (color.g + amount).min(1.0),
        b: (color.b + amount).min(1.0),
        a: color.a,
    }
}

fn darken(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r - amount).max(0.0),
        g: (color.g - amount).max(0.0),
        b: (color.b - amount).max(0.0),
        a: color.a,
    }
}

fn button_visual(
    background: Color,
    border: Color,
    text: Color,
    status: widget::button::Status,
) -> widget::button::Style {
    let (background, border, offset_y, blur_radius) = match status {
        widget::button::Status::Hovered => {
            (brighten(background, 0.035), brighten(border, 0.045), 4.0, 12.0)
        }
        widget::button::Status::Pressed => {
            (darken(background, 0.045), brighten(border, 0.025), 1.0, 5.0)
        }
        widget::button::Status::Disabled => {
            (darken(background, 0.055), darken(border, 0.055), 0.0, 0.0)
        }
        _ => (background, border, 5.0, 10.0),
    };

    widget::button::Style {
        snap: true,
        background: Some(Background::Color(background)),
        text_color: text,
        border: Border {
            color: border,
            width: 1.0,
            radius: Radius::from(13.0),
        },
        shadow: Shadow {
            color: rgb(0, 0, 0),
            offset: Vector::from([0.0, offset_y]),
            blur_radius,
        },
    }
}

fn primary_button<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    widget::button(
        widget::text(label)
            .size(14)
            .align_x(Horizontal::Center),
    )
    .padding(12)
    .style(|_theme, _status| {
        button_visual(
            rgb(70, 125, 255),
            rgb(107, 158, 255),
            Color::WHITE,
            _status,
        )
    })
    .on_press(message)
    .into()
}

fn secondary_button<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    widget::button(
        widget::text(label)
            .size(14)
            .align_x(Horizontal::Center),
    )
    .padding(12)
    .style(|_theme, _status| {
        button_visual(
            rgb(28, 35, 52),
            rgb(65, 78, 110),
            text_main(),
            _status,
        )
    })
    .on_press(message)
    .into()
}

fn danger_button<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    widget::button(
        widget::text(label)
            .size(14)
            .align_x(Horizontal::Center),
    )
    .padding(12)
    .style(|_theme, _status| {
        button_visual(
            rgb(104, 38, 55),
            rgb(185, 76, 99),
            Color::WHITE,
            _status,
        )
    })
    .on_press(message)
    .into()
}

fn mini_button<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    widget::button(
        widget::text(label)
            .size(12)
            .align_x(Horizontal::Center),
    )
    .padding(7)
    .style(|_theme, _status| {
        button_visual(
            rgb(28, 35, 52),
            rgb(65, 78, 110),
            text_muted(),
            _status,
        )
    })
    .on_press(message)
    .into()
}

fn copy_code_button<'a>(code: String, copied: bool) -> Element<'a, Message> {
    let label = if copied { "Copied ✓" } else { "Copy code" };

    widget::button(
        widget::text(label)
            .size(12)
            .align_x(Horizontal::Center),
    )
    .padding(8)
    .style(move |_theme, status| {
        if copied {
            button_visual(
                rgb(31, 92, 63),
                rgb(93, 225, 144),
                Color::WHITE,
                status,
            )
        } else {
            button_visual(
                rgb(28, 35, 52),
                rgb(65, 78, 110),
                text_muted(),
                status,
            )
        }
    })
    .on_press(Message::CopyPressed(code))
    .into()
}

fn text_input_style(
    _theme: &Theme,
    _status: widget::text_input::Status,
) -> widget::text_input::Style {
    widget::text_input::Style {
        background: Background::Color(rgb(10, 14, 22)),
        border: Border {
            color: rgb(58, 70, 100),
            width: 1.0,
            radius: Radius::from(14.0),
        },
        icon: text_muted(),
        placeholder: rgb(112, 124, 148),
        value: text_main(),
        selection: rgb(80, 130, 240),
    }
}

fn section_title<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    widget::column![
        widget::text(title)
            .size(27)
            .color(text_main()),
        Space::new().height(Length::Fixed(5.0)),
        widget::text(subtitle)
            .size(14)
            .color(text_muted()),
    ]
    .into()
}

fn setting_label<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    widget::column![
        widget::text(title)
            .size(16)
            .color(text_main()),
        Space::new().height(Length::Fixed(4.0)),
        widget::text(subtitle)
            .size(12)
            .color(text_muted()),
    ]
    .width(Length::Fill)
    .into()
}

fn help_card<'a>(title: &'a str, body: &'a str, color: Color) -> Element<'a, Message> {
    container(
        widget::column![
            container(
                widget::text(title)
                    .size(16)
                    .color(text_main())
            )
            .padding(8)
            .style(chip_style(color)),

            Space::new().height(Length::Fixed(10.0)),

            widget::text(body)
                .size(14)
                .color(text_muted()),
        ]
    )
    .padding(16)
    .width(Length::Fill)
    .style(flat_card_style)
    .into()
}

fn markdown_with_code_copy<'a>(
    items: &'a Vec<markdown::Item>,
    text_size: f32,
    copied_text: Option<&String>,
) -> Element<'a, Message> {
    let settings = iced::widget::markdown::Settings::with_text_size(
        text_size,
        Theme::Dark,
    );

    let mut children: Vec<Element<'a, Message>> = Vec::new();

    for item in items.iter() {
        children.push(
            selectable_markdown(
                std::iter::once(item),
                settings.clone(),
            )
            .map(|_| Message::None),
        );

        if let markdown::Item::CodeBlock { code, .. } = item {
            let copied = copied_text
                .map(|copied| copied == code)
                .unwrap_or(false);

            children.push(
                widget::row![
                    Space::new().width(Length::Fill),
                    copy_code_button(code.clone(), copied),
                ]
                .into(),
            );
        }
    }

    widget::Column::with_children(children)
        .spacing(iced::Pixels(8.0))
        .into()
}

fn message_bubble<'a>(
    message: Correspondence,
    parsed_markdown: Option<&'a Vec<markdown::Item>>,
    text_size: f32,
    model_name: String,
    copied_text: Option<&String>,
) -> Element<'a, Message> {
    match message {
        Correspondence::User(text) => {
            widget::row![
                Space::new().width(Length::Fill),
                container(
                    widget::column![
                        widget::text("You")
                            .size(12)
                            .color(rgb(205, 221, 255))
                            .align_x(Horizontal::Right),
                        Space::new().height(Length::Fixed(6.0)),
                        widget::text(text)
                            .size(text_size)
                            .align_x(Horizontal::Right),
                    ]
                )
                .padding(14)
                .width(Length::Shrink)
                .style(user_bubble_style),
                Space::new().width(Length::Fixed(8.0)),
            ]
            .into()
        }

        Correspondence::Bot(text) => {
            let fallback_text = text.clone();

            let body: Element<'a, Message> = if let Some(parsed) = parsed_markdown {
                markdown_with_code_copy(parsed, text_size, copied_text)
            } else {
                widget::text(fallback_text)
                    .size(text_size)
                    .color(text_main())
                    .align_x(Horizontal::Left)
                    .into()
            };

            widget::row![
                Space::new().width(Length::Fixed(8.0)),
                container(
                    widget::column![
                        widget::row![
                            widget::text(model_name)
                                .size(12)
                                .color(accent_2()),
                            Space::new().width(Length::Fill),
                        ],
                        Space::new().height(Length::Fixed(7.0)),
                        body,
                    ]
                )
                .padding(14)
                .width(Length::Fill)
                .style(bot_bubble_style),
                Space::new().width(Length::Fixed(42.0)),
            ]
            .into()
        }
    }
}

impl Program {
    pub fn get_ui_information<'a>(
        &'a self,
        gui_state: &'a GUIState,
    ) -> iced::widget::Container<'a, Message> {
        match gui_state {
            GUIState::InfoPopup => {
                let content = container(
                    widget::column![
                        container(
                            widget::row![
                                widget::column![
                                    widget::text("Ollama GUI Interface")
                                        .size(30)
                                        .color(text_main()),
                                    Space::new().height(Length::Fixed(6.0)),
                                    widget::text("A polished desktop interface for chatting with local Ollama models.")
                                        .size(14)
                                        .color(text_muted()),
                                ]
                                .width(Length::Fill),

                                container(
                                    widget::text("HELP")
                                        .size(13)
                                        .color(text_main())
                                )
                                .padding(10)
                                .style(chip_style(accent_2())),
                            ]
                        )
                        .padding(20)
                        .width(Length::Fill)
                        .style(panel_style),

                        Space::new().height(Length::Fixed(14.0)),

                        container(
                            widget::column![
                                widget::row![
                                    help_card(
                                        "Chat locally",
                                        "Select one of your installed Ollama models, type a prompt, and press Enter to generate a response.",
                                        accent(),
                                    ),
                                    Space::new().width(Length::Fixed(12.0)),
                                    help_card(
                                        "Manage models",
                                        "Use Advanced Settings to install models by name, change the Ollama address, or tune response rendering.",
                                        accent_2(),
                                    ),
                                ],

                                Space::new().height(Length::Fixed(12.0)),

                                widget::row![
                                    help_card(
                                        "System prompts",
                                        "System prompts let you switch the assistant's behaviour or personality without rewriting your prompt each time.",
                                        warning(),
                                    ),
                                    Space::new().width(Length::Fixed(12.0)),
                                    help_card(
                                        "Chat history",
                                        "When enabled, conversations can be saved locally. You can wipe the current history from Settings.",
                                        danger(),
                                    ),
                                ],

                                Space::new().height(Length::Fixed(16.0)),

                                container(
                                    widget::column![
                                        widget::text("Files and configuration")
                                            .size(17)
                                            .color(text_main()),
                                        Space::new().height(Length::Fixed(8.0)),
                                        widget::text(
                                            "Settings are stored in config/settings.json. System prompts are stored in config/defaultprompts.json. If logging is enabled, chat history saves to output/history.json."
                                        )
                                        .size(14)
                                        .color(text_muted()),
                                    ]
                                )
                                .padding(16)
                                .width(Length::Fill)
                                .style(flat_card_style),
                            ]
                        )
                        .padding(18)
                        .width(Length::Fill)
                        .style(panel_style),

                        Space::new().height(Length::Fixed(14.0)),

                        widget::row![
                            Space::new().width(Length::Fill),
                            primary_button("Back to chat", Message::ToggleInfoPopup),
                        ],
                    ]
                )
                .padding(0)
                .width(Length::Fill);

                return container(content)
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(app_background_style)
                    .into();
            }

            GUIState::Main => {
                let user_information = self.user_information.clone();
                let bots_list = self.app_state.bots_list.lock().unwrap().clone();
                let prompts_list = self
                    .system_prompt
                    .system_prompts_as_vec
                    .lock()
                    .unwrap()
                    .clone();
                let copied_text = self.last_copied_text.clone();
                let local_ollamastate = self.app_state.ollama_state.lock().unwrap().clone();

                let response_text = self
                    .response
                    .response_as_string
                    .lock()
                    .unwrap()
                    .clone();

                let chat_messages = {
                    let chat_history = self.user_information.chat_history.lock().unwrap();
                    chat_history.messages.clone()
                };

                let latest_bot_text = chat_messages
                    .iter()
                    .rev()
                    .find_map(|message| match message {
                        Correspondence::Bot(text) => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let latest_response_to_copy = if !response_text.trim().is_empty() {
                    response_text.clone()
                } else {
                    latest_bot_text
                };

                let selected_model = self.user_information.model.clone();
                let active_model_name = selected_model
                    .clone()
                    .unwrap_or_else(|| "No model selected".to_string());

                let prompt = iced::widget::TextInput::<Message>::new(
                    "Ask something...",
                    &self.prompt.prompt,
                )
                .padding(14)
                .size(18)
                .width(Length::Fill)
                .on_submit(Message::Prompt(self.prompt.prompt.clone()))
                .on_input(|input| Message::UpdatePrompt(input))
                .style(text_input_style);

                let mut chat_widgets: Vec<Element<Message>> = chat_messages
                    .iter()
                    .enumerate()
                    .flat_map(|(index, message)| {
                        let parsed_markdown = self.chat_markdown_cache.get(index);
                        let message_model_name = self
                            .chat_model_name_cache
                            .get(index)
                            .cloned()
                            .flatten()
                            .unwrap_or_else(|| active_model_name.clone());

                        vec![
                            message_bubble(
                                message.clone(),
                                parsed_markdown,
                                user_information.text_size,
                                message_model_name,
                                copied_text.as_ref(),
                            ),
                            Space::new().height(Length::Fixed(10.0)).into(),
                        ]
                    })
                    .collect();

                if !self.is_processing
                    && !response_text.trim().is_empty()
                    && matches!(chat_messages.last(), Some(Correspondence::Bot(_)))
                {
                    chat_widgets.pop();
                    chat_widgets.pop();
                }

                let online = local_ollamastate.to_lowercase() != "offline";
                let status_color = if online { success() } else { danger() };
                let debug_color = if self.debug_message.clone().is_error {
                    danger()
                } else {
                    success()
                };

                let model_selector: Element<Message> = if bots_list.is_empty() {
                    container(
                        widget::text("No models installed")
                            .size(13)
                            .color(text_muted())
                    )
                    .padding(10)
                    .style(chip_style(danger()))
                    .into()
                } else {
                    widget::pick_list(
                        bots_list.clone(),
                        selected_model.clone(),
                        Message::ModelChange,
                    )
                    .width(Length::Fixed(250.0))
                    .into()
                };

                let system_prompt_selector: Element<Message> = if prompts_list.is_empty() {
                    container(
                        widget::text("No prompts found")
                            .size(13)
                            .color(text_muted())
                    )
                    .padding(10)
                    .style(chip_style(warning()))
                    .into()
                } else {
                    widget::pick_list(
                        prompts_list.clone(),
                        self.system_prompt.system_prompt.clone(),
                        Message::SystemPromptChange,
                    )
                    .width(Length::Fixed(230.0))
                    .into()
                };

                let live_response: Element<Message> = if self.is_processing
                    || !response_text.trim().is_empty()
                {
                    let response_model_name = self
                        .active_response_model_name
                        .clone()
                        .unwrap_or_else(|| active_model_name.clone());

                    let label = if self.is_processing {
                        format!("{} is responding...", response_model_name)
                    } else {
                        response_model_name
                    };

                    widget::row![
                        Space::new().width(Length::Fixed(8.0)),
                        container(
                            widget::column![
                                widget::row![
                                    widget::text(label)
                                        .size(12)
                                        .color(accent_2()),
                                    Space::new().width(Length::Fill),
                                ],
                                Space::new().height(Length::Fixed(8.0)),
                                markdown_with_code_copy(
                                    &self.response.parsed_markdown,
                                    user_information.text_size,
                                    copied_text.as_ref(),
                                ),
                            ]
                        )
                        .padding(14)
                        .width(Length::Fill)
                        .style(bot_bubble_style),
                        Space::new().width(Length::Fixed(42.0)),
                    ]
                    .into()
                } else if chat_messages.is_empty() {
                    container(
                        widget::column![
                            widget::text("Ready when you are.")
                                .size(22)
                                .color(text_main())
                                .align_x(Horizontal::Center),
                            Space::new().height(Length::Fixed(8.0)),
                            widget::text("Choose a model, type a prompt, and start chatting locally.")
                                .size(14)
                                .color(text_muted())
                                .align_x(Horizontal::Center),
                        ]
                        .align_x(Horizontal::Center)
                    )
                    .padding(30)
                    .width(Length::Fill)
                    .style(flat_card_style)
                    .into()
                } else {
                    widget::column![].into()
                };

                let offline_hint: Element<Message> = if !online {
                    container(
                        widget::row![
                            widget::column![
                                widget::text("Ollama was not detected.")
                                    .size(14)
                                    .color(text_main()),
                                Space::new().height(Length::Fixed(3.0)),
                                widget::text("Install Ollama or check your connection settings.")
                                    .size(12)
                                    .color(text_muted()),
                            ]
                            .width(Length::Fill),
                            secondary_button("Install Ollama", Message::InstallationPrompt),
                        ]
                    )
                    .padding(14)
                    .width(Length::Fill)
                    .style(flat_card_style)
                    .into()
                } else {
                    widget::column![].into()
                };

                let missing_bots_hint: Element<Message> = if bots_list.is_empty() {
                    container(
                        widget::row![
                            widget::column![
                                widget::text("No models were detected.")
                                    .size(14)
                                    .color(text_main()),
                                Space::new().height(Length::Fixed(3.0)),
                                widget::text("Install a model before sending prompts.")
                                    .size(12)
                                    .color(text_muted()),
                            ]
                            .width(Length::Fill),
                            secondary_button("Find models", Message::ListPrompt),
                        ]
                    )
                    .padding(14)
                    .width(Length::Fill)
                    .style(flat_card_style)
                    .into()
                } else {
                    widget::column![].into()
                };

                let content = widget::column![
                    container(
                        widget::row![
                            widget::column![
                                widget::text("Ollama GUI")
                                    .size(29)
                                    .color(text_main()),
                                Space::new().height(Length::Fixed(4.0)),
                                widget::text("Local AI chat, model control, and prompt management.")
                                    .size(14)
                                    .color(text_muted()),
                            ]
                            .width(Length::Fill),

                            container(
                                widget::row![
                                    widget::text("●")
                                        .size(13)
                                        .color(status_color),
                                    Space::new().width(Length::Fixed(7.0)),
                                    widget::text(format!("Ollama {}", local_ollamastate))
                                        .size(13)
                                        .color(text_main()),
                                ]
                            )
                            .padding(10)
                            .style(chip_style(status_color)),

                            Space::new().width(Length::Fixed(8.0)),
                            secondary_button("Settings", Message::ToggleSettings),
                            Space::new().width(Length::Fixed(8.0)),
                            secondary_button("Help", Message::ToggleInfoPopup),
                        ]
                    )
                    .padding(18)
                    .width(Length::Fill)
                    .style(panel_style),

                    Space::new().height(Length::Fixed(14.0)),

                    container(
                        widget::scrollable(
                            widget::column![
                                widget::Column::with_children(chat_widgets)
                                    .spacing(iced::Pixels(3.0)),
                                live_response,
                                Space::new().height(Length::Fixed(18.0)),
                            ]
                            .spacing(iced::Pixels(6.0))
                        )
                        .height(Length::Fill)
                        .anchor_bottom()
                    )
                    .padding(14)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(panel_style),

                    Space::new().height(Length::Fixed(14.0)),

                    container(
                        widget::column![
                            widget::row![
                                widget::column![
                                    widget::text("Model")
                                        .size(12)
                                        .color(text_faint()),
                                    Space::new().height(Length::Fixed(5.0)),
                                    model_selector,
                                ],

                                Space::new().width(Length::Fixed(12.0)),

                                widget::column![
                                    widget::text("System prompt")
                                        .size(12)
                                        .color(text_faint()),
                                    Space::new().height(Length::Fixed(5.0)),
                                    system_prompt_selector,
                                ],

                                Space::new().width(Length::Fixed(12.0)),

                                container(
                                    widget::row![
                                        widget::text("Thinking")
                                            .size(13)
                                            .color(text_muted()),
                                        Space::new().width(Length::Fixed(8.0)),
                                        widget::checkbox(self.user_information.think)
                                            .label("")
                                            .on_toggle(|_| Message::ToggleThinking),
                                    ]
                                )
                                .padding(10)
                                .style(chip_style(if self.user_information.think {
                                    accent_2()
                                } else {
                                    border_bright()
                                })),

                                Space::new().width(Length::Fill),

                                secondary_button(
                                    "Copy latest response",
                                    Message::CopyPressed(latest_response_to_copy)
                                ),
                            ],

                            Space::new().height(Length::Fixed(12.0)),

                            container(
                                widget::row![
                                    prompt,
                                    Space::new().width(Length::Fixed(10.0)),
                                    primary_button(
                                        "Send",
                                        Message::Prompt(self.prompt.prompt.clone())
                                    ),
                                ]
                            )
                            .padding(8)
                            .style(input_shell_style),

                            Space::new().height(Length::Fixed(10.0)),
                            offline_hint,
                            missing_bots_hint,

                            widget::row![
                                widget::text(self.debug_message.clone().message)
                                    .size(13)
                                    .color(debug_color),
                            ],
                        ]
                    )
                    .padding(16)
                    .width(Length::Fill)
                    .style(panel_style),
                ]
                .spacing(iced::Pixels(0.0));

                return container(content)
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(app_background_style)
                    .into();
            }

            GUIState::Settings => {
                let user_information = self.user_information.clone();
                let bots_list = self.app_state.bots_list.lock().unwrap().clone();
                let prompts_list = self
                    .system_prompt
                    .system_prompts_as_vec
                    .lock()
                    .unwrap()
                    .clone();

                let debug_color = if self.debug_message.clone().is_error {
                    danger()
                } else {
                    success()
                };

                let content = widget::column![
                    container(
                        widget::row![
                            section_title(
                                "Settings",
                                "Tune model behaviour, prompt selection, and chat preferences."
                            ),
                            Space::new().width(Length::Fill),
                            secondary_button("Go back", Message::ToggleSettings),
                        ]
                    )
                    .padding(18)
                    .width(Length::Fill)
                    .style(panel_style),

                    Space::new().height(Length::Fixed(14.0)),

                    container(
                        widget::column![
                            container(
                                widget::row![
                                    setting_label(
                                        "Model",
                                        "Choose the Ollama model used for new responses."
                                    ),
                                    widget::pick_list(
                                        bots_list,
                                        self.user_information.model.clone(),
                                        Message::ModelChange,
                                    )
                                    .width(Length::Fixed(280.0)),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::row![
                                    setting_label(
                                        "Thinking",
                                        "Enable thinking mode for models that support it."
                                    ),
                                    widget::checkbox(self.user_information.think)
                                        .label("Enabled")
                                        .on_toggle(|_| Message::ToggleThinking),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::column![
                                    setting_label(
                                        "Temperature",
                                        "Higher values make output more random."
                                    ),
                                    Space::new().height(Length::Fixed(10.0)),
                                    widget::row![
                                        widget::slider(
                                            0.0..=10.0,
                                            self.user_information.temperature.clone(),
                                            Message::UpdateTemperature,
                                        ),
                                        Space::new().width(Length::Fixed(12.0)),
                                        container(
                                            widget::text(format!(
                                                "{:.1}",
                                                self.user_information.temperature
                                            ))
                                            .size(13)
                                            .color(text_main())
                                        )
                                        .padding(8)
                                        .style(chip_style(accent())),
                                    ],
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::row![
                                    setting_label(
                                        "System prompt",
                                        "Choose the personality or instruction profile."
                                    ),
                                    widget::pick_list(
                                        prompts_list,
                                        self.system_prompt.system_prompt.clone(),
                                        Message::SystemPromptChange,
                                    )
                                    .width(Length::Fixed(280.0)),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::column![
                                    setting_label(
                                        "Text size",
                                        "Adjust chat and response readability."
                                    ),
                                    Space::new().height(Length::Fixed(10.0)),
                                    widget::row![
                                        widget::slider(
                                            1.0..=40.0,
                                            self.user_information.text_size.clone(),
                                            Message::UpdateTextSize,
                                        ),
                                        Space::new().width(Length::Fixed(12.0)),
                                        container(
                                            widget::text(format!(
                                                "{:.0}px",
                                                self.user_information.text_size
                                            ))
                                            .size(13)
                                            .color(text_main())
                                        )
                                        .padding(8)
                                        .style(chip_style(accent_2())),
                                    ],
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::row![
                                    setting_label(
                                        "Chat history",
                                        "Save the current chat history locally."
                                    ),
                                    widget::checkbox(
                                        user_information.current_chat_history_enabled
                                    )
                                    .label("Enabled")
                                    .on_toggle(|_| Message::ToggleChatHistory),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(14.0)),

                            container(
                                widget::row![
                                    widget::column![
                                        widget::text("Maintenance")
                                            .size(16)
                                            .color(text_main()),
                                        Space::new().height(Length::Fixed(4.0)),
                                        widget::text("Clear local conversation data or open deeper configuration options.")
                                            .size(12)
                                            .color(text_muted()),
                                    ]
                                    .width(Length::Fill),

                                    danger_button(
                                        "Wipe chat history",
                                        Message::WipeChatHistory
                                    ),

                                    Space::new().width(Length::Fixed(10.0)),

                                    secondary_button(
                                        "Advanced settings",
                                        Message::ToggleAdvancedSettings
                                    ),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(danger_zone_style),

                            Space::new().height(Length::Fixed(12.0)),

                            widget::text(self.debug_message.clone().message)
                                .size(13)
                                .color(debug_color),
                        ]
                    )
                    .padding(18)
                    .width(Length::Fill)
                    .style(panel_style),
                ];

                return container(content)
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(app_background_style)
                    .into();
            }

            GUIState::AdvancedSettings => {
                let user_information = self.user_information.clone();
                let ip = self.user_information.ip_address.clone();

                let prompts_list = self
                    .system_prompt
                    .system_prompts_as_vec
                    .lock()
                    .unwrap()
                    .clone();

                let model_install = iced::widget::TextInput::<Message>::new(
                    "Model name, e.g. llama3.2:3b",
                    &self.installing_model,
                )
                .padding(12)
                .size(15)
                .width(Length::Fixed(310.0))
                .on_submit(Message::InstallModel(self.installing_model.clone()))
                .on_input(|input| Message::UpdateInstall(input))
                .style(text_input_style);

                let change_ip = iced::widget::TextInput::<Message>::new(
                    ip.ip.as_str(),
                    &ip.ip,
                )
                .padding(12)
                .size(15)
                .width(Length::Fixed(230.0))
                .on_submit(Message::ChangeIp(ip.ip.clone()))
                .on_input(|input| Message::ChangeIp(input))
                .style(text_input_style);

                let change_port = iced::widget::TextInput::<Message>::new(
                    ip.port.as_str(),
                    &ip.port,
                )
                .padding(12)
                .size(15)
                .width(Length::Fixed(110.0))
                .on_submit(Message::ChangePort(ip.port.clone()))
                .on_input(|input| Message::ChangePort(input))
                .style(text_input_style);

                let content = widget::column![
                    container(
                        widget::row![
                            section_title(
                                "Advanced settings",
                                "Install models, change connection settings, and tune rendering."
                            ),
                            Space::new().width(Length::Fill),
                            secondary_button("Back to settings", Message::ToggleAdvancedSettings),
                        ]
                    )
                    .padding(18)
                    .width(Length::Fill)
                    .style(panel_style),

                    Space::new().height(Length::Fixed(14.0)),

                    container(
                        widget::column![
                            container(
                                widget::row![
                                    setting_label(
                                        "System prompt",
                                        "Change the active prompt profile."
                                    ),
                                    widget::pick_list(
                                        prompts_list,
                                        self.system_prompt.system_prompt.clone(),
                                        Message::SystemPromptChange,
                                    )
                                    .width(Length::Fixed(310.0)),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::row![
                                    setting_label(
                                        "Install model",
                                        "Enter an Ollama model name and press Enter."
                                    ),
                                    model_install,
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::column![
                                    setting_label(
                                        "Batch tokens",
                                        "Number of tokens to process before rendering. Recommended: 3."
                                    ),
                                    Space::new().height(Length::Fixed(10.0)),
                                    widget::row![
                                        widget::slider(
                                            1.0..=10.0,
                                            self.batch_tokens as f32,
                                            |value| Message::ChangeBatchTokens(value as i32),
                                        ),
                                        Space::new().width(Length::Fixed(12.0)),
                                        container(
                                            widget::text(format!("{}", self.batch_tokens))
                                                .size(13)
                                                .color(text_main())
                                        )
                                        .padding(8)
                                        .style(chip_style(accent())),
                                    ],
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::column![
                                    setting_label(
                                        "Ollama address",
                                        "Change the IP address and port used to connect to Ollama."
                                    ),
                                    Space::new().height(Length::Fixed(12.0)),
                                    widget::row![
                                        change_ip,
                                        Space::new().width(Length::Fixed(8.0)),
                                        widget::text(":")
                                            .size(20)
                                            .color(text_muted()),
                                        Space::new().width(Length::Fixed(8.0)),
                                        change_port,
                                    ],
                                    Space::new().height(Length::Fixed(12.0)),
                                    container(
                                        widget::text(format!(
                                            "Current address: {}:{}",
                                            user_information.ip_address.ip,
                                            user_information.ip_address.port
                                        ))
                                        .size(13)
                                        .color(text_main())
                                    )
                                    .padding(10)
                                    .style(chip_style(accent_2())),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),
                        ]
                    )
                    .padding(18)
                    .width(Length::Fill)
                    .style(panel_style),
                ];

                return container(content)
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(app_background_style)
                    .into();
            }
        }
    }
}
