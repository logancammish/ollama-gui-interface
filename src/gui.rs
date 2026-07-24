use iced::{
    Background, Border, Color, Element, Length, Shadow, Theme, Vector,
    alignment::Horizontal,
    border::Radius,
    widget::{self, Space, container},
};

use iced_selection::markdown as selectable_markdown;
use iced_widget::{container::Style, core::text::Wrapping, markdown};
use std::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    ChatImage, Correspondence, GUIState, Language, Message, Program, ThinkingLevel,
    split_thinking_text,
    web_search::{WebSearchState, WebSource},
};

/// Spanish is intentionally shipped as an experimental machine-generated
/// translation. Unknown/new strings fall back to English instead of disappearing.
fn tr(language: Language, english: &'static str) -> &'static str {
    if language == Language::English {
        return english;
    }
    match english {
        "A polished desktop interface for chatting with local Ollama models." => {
            "Una interfaz de escritorio cuidada para conversar con modelos locales de Ollama."
        }
        "HELP" => "AYUDA",
        "Chat locally" => "Chat local",
        "Select one of your installed Ollama models, type a prompt, and press Enter to generate a response." => {
            "Selecciona uno de tus modelos de Ollama instalados, escribe un mensaje y pulsa Intro para generar una respuesta."
        }
        "Manage models" => "Gestionar modelos",
        "Use Advanced Settings to install models by name, change the Ollama address, or tune response rendering." => {
            "Usa la configuración avanzada para instalar modelos por nombre, cambiar la dirección de Ollama o ajustar la presentación de respuestas."
        }
        "System prompts" => "Indicaciones del sistema",
        "System prompts let you switch the assistant's behaviour or personality without rewriting your prompt each time." => {
            "Las indicaciones del sistema permiten cambiar el comportamiento o la personalidad del asistente sin reescribirlas cada vez."
        }
        "Chat history" => "Historial de chats",
        "When enabled, conversations can be saved locally. You can wipe the current history from Settings." => {
            "Cuando está activado, las conversaciones se guardan localmente. Puedes borrar el contexto actual en Configuración."
        }
        "Files and configuration" => "Archivos y configuración",
        "User settings, logs, generated images, and chats are stored in your local application-data folder. Installed assets remain read-only." => {
            "Los ajustes, registros, imágenes generadas y chats se guardan en la carpeta local de datos de la aplicación. Los recursos instalados son de solo lectura."
        }
        "Back to chat" => "Volver al chat",
        "No model selected" => "Ningún modelo seleccionado",
        "Ask something..." => "Escribe algo...",
        "No models installed" => "No hay modelos instalados",
        "Thinking" => "Razonamiento",
        "Ready when you are." => "Listo cuando quieras.",
        "Choose a model, type a prompt, and start chatting locally." => {
            "Elige un modelo, escribe un mensaje y empieza a conversar localmente."
        }
        "Ollama was not detected." => "No se detectó Ollama.",
        "Install Ollama or check your connection settings." => {
            "Instala Ollama o revisa la configuración de conexión."
        }
        "Install Ollama" => "Instalar Ollama",
        "No models were detected." => "No se detectaron modelos.",
        "Install a model before sending prompts." => "Instala un modelo antes de enviar mensajes.",
        "Find models" => "Buscar modelos",
        "＋ New chat" => "＋ Nuevo chat",
        "Leave temporary chat" => "Salir del chat temporal",
        "Temporary chat" => "Chat temporal",
        "Temporary chats" => "Chats temporales",
        "Temporary · not saved" => "Temporal · no guardado",
        "Saved chats" => "Chats guardados",
        "Unpin" => "Desfijar",
        "Pin" => "Fijar",
        "Chats" => "Chats",
        "Local workspace" => "Espacio local",
        "Online" => "En línea",
        "Offline" => "Sin conexión",
        "Images" => "Imágenes",
        "＋ Image" => "＋ Imagen",
        "Settings" => "Configuración",
        "Enable Web Search" => "Activar búsqueda web",
        "Web search may send search queries and webpage URLs to the selected external provider." => {
            "La búsqueda web puede enviar consultas y direcciones de páginas al proveedor externo seleccionado."
        }
        "Search provider" => "Proveedor de búsqueda",
        "API key" => "Clave de API",
        "Prefer BRAVE_SEARCH_API_KEY for secret storage. A key entered here is stored in the local settings file and never printed in logs." => {
            "Es preferible usar BRAVE_SEARCH_API_KEY. Las claves introducidas aquí se guardan en el archivo local de configuración y nunca se muestran en los registros."
        }
        "Search result limit" => "Límite de resultados",
        "Allow a follow-up search" => "Permitir una búsqueda adicional",
        "Lets the model run one additional, distinct search when the first results are insufficient." => {
            "Permite que el modelo haga otra búsqueda distinta cuando los primeros resultados no sean suficientes."
        }
        "Web search" => "Búsqueda web",
        "Web on" => "Web activada",
        "Web off" => "Web desactivada",
        "Searching the web…" => "Buscando en la web…",
        "Fetching webpage…" => "Leyendo página web…",
        "Web search activity" => "Actividad de búsqueda web",
        "Searching" => "Buscando",
        "Reviewing results" => "Revisando resultados",
        "Reading website" => "Leyendo sitio web",
        "found" => "encontrados",
        "Websites found" => "Sitios encontrados",
        "The model is choosing which result to read." => {
            "El modelo está eligiendo qué resultado leer."
        }
        "Preparing the answer from these sources." => "Preparando la respuesta con estas fuentes.",
        "Search query" => "Consulta",
        "Details" => "Detalles",
        "ERROR" => "ERROR",
        "READING" => "LEYENDO",
        "Sources" => "Fuentes",
        "WEB" => "WEB",
        "MODEL" => "MODELO",
        "SYSTEM PROMPT" => "INDICACIÓN DEL SISTEMA",
        "System prompt" => "Indicación del sistema",
        "REASONING" => "RAZONAMIENTO",
        "Stop" => "Detener",
        "Send" => "Enviar",
        "Paste image" => "Pegar imagen",
        "Copy response" => "Copiar respuesta",
        "Remove" => "Quitar",
        "Copied ✓" => "Copiado ✓",
        "Copy code" => "Copiar código",
        "You" => "Tú",
        "▾ Hide thinking" => "▾ Ocultar razonamiento",
        "▸ Show thinking" => "▸ Mostrar razonamiento",
        "Describe an image, or ask a question about the attached image…" => {
            "Describe una imagen o pregunta sobre la imagen adjunta…"
        }
        "Add an image for vision" => "Añade una imagen para visión",
        "Paste from the clipboard or choose a local image." => {
            "Pega desde el portapapeles o elige una imagen local."
        }
        "Choose image" => "Elegir imagen",
        "Copy image" => "Copiar imagen",
        "Vision model is responding…" => "El modelo de visión está respondiendo…",
        "Vision response" => "Respuesta de visión",
        "Generate image" => "Generar imagen",
        "Generating…" => "Generando…",
        "Describe the image you want to generate…" => "Describe la imagen que quieres generar…",
        "Use vision models to inspect images or experimental image models to create them." => {
            "Usa modelos de visión para analizar imágenes o modelos experimentales para crearlas."
        }
        "Model" => "Modelo",
        "Ask about image" => "Preguntar sobre la imagen",
        "Generation requires an Ollama image-generation model and supported runtime." => {
            "La generación requiere un modelo de imágenes de Ollama y un entorno compatible."
        }
        "Analyze images with a vision model. Experimental image generation appears only for models that report support." => {
            "Analiza imágenes con un modelo de visión. La generación experimental solo aparece en modelos compatibles."
        }
        "Vision analysis" => "Análisis visual",
        "Attach an image and ask a vision-capable model to describe, classify, read, or reason about it." => {
            "Adjunta una imagen y pide a un modelo con visión que la describa, clasifique, lea o analice."
        }
        "This model can inspect images." => "Este modelo puede analizar imágenes.",
        "This model does not support image input." => "Este modelo no admite imágenes de entrada.",
        "Checking image capabilities…" => "Comprobando capacidades de imagen…",
        "Experimental image generation" => "Generación de imágenes experimental",
        "Ollama reports that this model can generate images. Output is requested through /api/generate at 1024 × 1024." => {
            "Ollama indica que este modelo puede generar imágenes. Se solicita la salida mediante /api/generate a 1024 × 1024."
        }
        "Generated images" => "Imágenes generadas",
        "Tune model behaviour, prompt selection, and chat preferences." => {
            "Ajusta el comportamiento del modelo, las indicaciones y las preferencias del chat."
        }
        "Go back" => "Volver",
        "Choose the Ollama model used for new responses." => {
            "Elige el modelo de Ollama para las respuestas nuevas."
        }
        "Thinking effort" => "Nivel de razonamiento",
        "Choose how much reasoning the model should use." => {
            "Elige cuánto razonamiento debe usar el modelo."
        }
        "Reasoning" => "Razonamiento",
        "This model does not offer adjustable reasoning." => {
            "Este modelo no ofrece razonamiento ajustable."
        }
        "Select a model and wait while reasoning support is checked." => {
            "Selecciona un modelo mientras se comprueba la compatibilidad con razonamiento."
        }
        "Temperature" => "Temperatura",
        "Higher values make output more random." => {
            "Los valores altos producen respuestas más aleatorias."
        }
        "Maximum response" => "Respuesta máxima",
        "Caps generated output in tokens, including hidden reasoning. The default is 10,240 tokens." => {
            "Limita la salida generada en tokens, incluido el razonamiento oculto. El valor predeterminado es 10.240 tokens."
        }
        "Context window" => "Ventana de contexto",
        "Controls how much conversation and generated output the model can hold. Larger values use substantially more memory." => {
            "Controla cuánta conversación y salida puede mantener el modelo. Los valores grandes usan bastante más memoria."
        }
        "Choose the personality or instruction profile." => {
            "Elige el perfil de personalidad o instrucciones."
        }
        "Text size" => "Tamaño del texto",
        "Adjust chat and response readability." => {
            "Ajusta la legibilidad del chat y las respuestas."
        }
        "Dark mode" => "Modo oscuro",
        "Switch between the dark and light interface themes." => {
            "Cambia entre los temas oscuro y claro de la interfaz."
        }
        "Chat storage" => "Almacenamiento de chats",
        "Saved chats use this folder. The full path is shown so you can always locate them." => {
            "Los chats guardados usan esta carpeta. Se muestra la ruta completa para que puedas encontrarlos."
        }
        "Choose folder" => "Elegir carpeta",
        "Model conversation context" => "Contexto de conversación del modelo",
        "Include earlier messages from this chat in the next model request. Saved chats are managed in the left menu." => {
            "Incluye mensajes anteriores de este chat en la próxima solicitud. Los chats guardados se gestionan en el menú izquierdo."
        }
        "Enabled" => "Activado",
        "Interface language" => "Idioma de la interfaz",
        "Spanish is experimental and machine-generated. It will be replaced with a human translation in a future update." => {
            "El español es experimental y ha sido generado automáticamente. Se sustituirá por una traducción humana en una actualización futura."
        }
        "Maintenance" => "Mantenimiento",
        "Clear local conversation data or open deeper configuration options." => {
            "Borra el contexto local o abre opciones de configuración adicionales."
        }
        "Clear current context" => "Borrar contexto actual",
        "Advanced settings" => "Configuración avanzada",
        "Model name, e.g. llama3.2:3b" => "Nombre del modelo, p. ej. llama3.2:3b",
        "Install models, change connection settings, and tune rendering." => {
            "Instala modelos, cambia la conexión y ajusta la presentación."
        }
        "Back to settings" => "Volver a configuración",
        "Change the active prompt profile." => "Cambia el perfil de indicaciones activo.",
        "Install model" => "Instalar modelo",
        "Enter an Ollama model name and press Enter." => {
            "Escribe el nombre de un modelo de Ollama y pulsa Intro."
        }
        "Batch tokens" => "Lote de tokens",
        "Tokens per visual update when fast streaming is off. Higher values reduce rendering work." => {
            "Tokens por actualización visual cuando la transmisión rápida está desactivada. Los valores altos reducen el trabajo de presentación."
        }
        "Fast streaming" => "Transmisión rápida",
        "Render as soon as the API yields output. Turn off to use token batching." => {
            "Muestra la respuesta en cuanto la API produce contenido. Desactívalo para usar lotes de tokens."
        }
        "Content filtering" => "Filtro de contenido",
        "Censor offensive, profane, sexual, and severely inappropriate words with # characters." => {
            "Censura palabras ofensivas, malsonantes, sexuales y gravemente inapropiadas con caracteres #."
        }
        "Ollama address" => "Dirección de Ollama",
        "Change the IP address and port used to connect to Ollama." => {
            "Cambia la dirección IP y el puerto usados para conectar con Ollama."
        }
        "Fastest · no extra reasoning" => "Más rápido · sin razonamiento adicional",
        "Quick reasoning for everyday questions" => "Razonamiento rápido para preguntas cotidianas",
        "Balanced for multi-step tasks" => "Equilibrado para tareas de varios pasos",
        "Most thorough · slower responses" => "Más exhaustivo · respuestas más lentas",
        _ => english,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ThinkingChoice {
    level: ThinkingLevel,
    language: Language,
}

impl ThinkingChoice {
    fn all(language: Language) -> [Self; 4] {
        [
            ThinkingLevel::Off,
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
        ]
        .map(|level| Self { level, language })
    }
}

impl fmt::Display for ThinkingChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match (self.language, self.level) {
            (Language::Spanish, ThinkingLevel::Off) => "Desactivado",
            (Language::Spanish, ThinkingLevel::Low) => "Bajo",
            (Language::Spanish, ThinkingLevel::Medium) => "Medio",
            (Language::Spanish, ThinkingLevel::High) => "Alto",
            (_, ThinkingLevel::Off) => "Off",
            (_, ThinkingLevel::Low) => "Low",
            (_, ThinkingLevel::Medium) => "Medium",
            (_, ThinkingLevel::High) => "High",
        };
        formatter.write_str(label)
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

static DARK_MODE: AtomicBool = AtomicBool::new(true);

pub(crate) fn set_dark_mode(enabled: bool) {
    DARK_MODE.store(enabled, Ordering::Relaxed);
}

fn is_dark_mode() -> bool {
    DARK_MODE.load(Ordering::Relaxed)
}

fn app_bg() -> Color {
    if is_dark_mode() {
        rgb(9, 12, 18)
    } else {
        rgb(242, 245, 250)
    }
}

fn panel() -> Color {
    if is_dark_mode() {
        rgb(15, 19, 28)
    } else {
        rgb(255, 255, 255)
    }
}

fn panel_soft() -> Color {
    if is_dark_mode() {
        rgb(20, 25, 36)
    } else {
        rgb(247, 249, 252)
    }
}

fn panel_lifted() -> Color {
    if is_dark_mode() {
        rgb(24, 30, 43)
    } else {
        rgb(251, 252, 254)
    }
}

fn border_soft() -> Color {
    if is_dark_mode() {
        rgb(39, 48, 65)
    } else {
        rgb(207, 214, 225)
    }
}

fn border_bright() -> Color {
    if is_dark_mode() {
        rgb(83, 101, 135)
    } else {
        rgb(143, 155, 177)
    }
}

fn text_main() -> Color {
    if is_dark_mode() {
        rgb(238, 241, 246)
    } else {
        rgb(25, 31, 43)
    }
}

fn text_muted() -> Color {
    if is_dark_mode() {
        rgb(165, 177, 198)
    } else {
        rgb(82, 94, 114)
    }
}

fn text_faint() -> Color {
    if is_dark_mode() {
        rgb(128, 139, 154)
    } else {
        rgb(105, 116, 135)
    }
}

fn accent() -> Color {
    rgb(111, 139, 255)
}

fn accent_2() -> Color {
    rgb(91, 211, 255)
}

fn danger() -> Color {
    rgb(255, 92, 116)
}

fn success() -> Color {
    rgb(91, 203, 151)
}

fn warning() -> Color {
    rgb(255, 190, 94)
}

fn shadow_color() -> Color {
    Color {
        a: if is_dark_mode() { 0.30 } else { 0.12 },
        ..rgb(0, 0, 0)
    }
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
            radius: Radius::from(14.0),
        },
        shadow: Shadow {
            color: shadow_color(),
            offset: Vector::from([0.0, 6.0]),
            blur_radius: 18.0,
        },
    }
}

fn pick_list_style(_theme: &Theme, status: widget::pick_list::Status) -> widget::pick_list::Style {
    let active = !matches!(status, widget::pick_list::Status::Active);
    widget::pick_list::Style {
        text_color: text_main(),
        placeholder_color: text_faint(),
        handle_color: if active { accent() } else { text_muted() },
        background: Background::Color(if active {
            if is_dark_mode() {
                rgb(29, 37, 57)
            } else {
                rgb(232, 237, 248)
            }
        } else {
            panel_soft()
        }),
        border: Border {
            color: if active { accent() } else { border_soft() },
            width: if active { 1.5 } else { 1.0 },
            radius: Radius::from(12.0),
        },
    }
}

fn pick_list_menu_style(_theme: &Theme) -> widget::overlay::menu::Style {
    widget::overlay::menu::Style {
        background: Background::Color(panel_lifted()),
        border: Border {
            color: border_bright(),
            width: 1.0,
            radius: Radius::from(14.0),
        },
        text_color: text_main(),
        selected_text_color: Color::WHITE,
        selected_background: Background::Color(if is_dark_mode() {
            rgb(49, 67, 122)
        } else {
            rgb(75, 99, 205)
        }),
        shadow: Shadow {
            color: shadow_color(),
            offset: Vector::from([0.0, 8.0]),
            blur_radius: 22.0,
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
            radius: Radius::from(12.0),
        },
        shadow: Shadow::default(),
    }
}

fn chat_entry_style(active: bool) -> impl Fn(&Theme) -> Style {
    move |_theme| Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(if active {
            if is_dark_mode() {
                rgb(49, 67, 122)
            } else {
                rgb(224, 231, 250)
            }
        } else {
            panel_lifted()
        })),
        border: Border {
            color: if active { accent() } else { border_soft() },
            width: 1.0,
            radius: Radius::from(13.0),
        },
        shadow: Shadow::default(),
    }
}

fn chat_title_button_style(
    _theme: &Theme,
    status: widget::button::Status,
) -> widget::button::Style {
    widget::button::Style {
        snap: true,
        background: match status {
            widget::button::Status::Hovered => {
                Some(Background::Color(Color::from_rgba8(255, 255, 255, 0.05)))
            }
            _ => None,
        },
        text_color: text_main(),
        border: Border {
            radius: Radius::from(10.0),
            ..Border::default()
        },
        shadow: Shadow::default(),
    }
}

fn input_shell_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(panel_soft())),
        border: Border {
            color: border_soft(),
            width: 1.0,
            radius: Radius::from(14.0),
        },
        shadow: Shadow {
            color: shadow_color(),
            offset: Vector::from([0.0, 4.0]),
            blur_radius: 12.0,
        },
    }
}

fn user_bubble_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(if is_dark_mode() {
            Color::WHITE
        } else {
            text_main()
        }),
        background: Some(Background::Color(if is_dark_mode() {
            rgb(40, 52, 99)
        } else {
            rgb(224, 231, 250)
        })),
        border: Border {
            color: accent(),
            width: 1.0,
            radius: Radius::from(14.0),
        },
        shadow: Shadow {
            color: shadow_color(),
            offset: Vector::from([0.0, 4.0]),
            blur_radius: 12.0,
        },
    }
}

fn bot_bubble_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(panel_soft())),
        border: Border {
            color: border_soft(),
            width: 1.0,
            radius: Radius::from(14.0),
        },
        shadow: Shadow {
            color: shadow_color(),
            offset: Vector::from([0.0, 4.0]),
            blur_radius: 12.0,
        },
    }
}

fn web_activity_style(_theme: &Theme) -> Style {
    Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(if is_dark_mode() {
            rgb(17, 27, 39)
        } else {
            rgb(235, 248, 252)
        })),
        border: Border {
            color: rgb(48, 112, 139),
            width: 1.0,
            radius: Radius::from(12.0),
        },
        shadow: Shadow::default(),
    }
}

fn website_row_style(active: bool) -> impl Fn(&Theme) -> Style {
    move |_theme| Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(if active {
            if is_dark_mode() {
                rgb(27, 55, 68)
            } else {
                rgb(219, 242, 247)
            }
        } else {
            panel_soft()
        })),
        border: Border {
            color: if active { accent_2() } else { border_soft() },
            width: 1.0,
            radius: Radius::from(10.0),
        },
        shadow: Shadow::default(),
    }
}

fn chip_style(color: Color) -> impl Fn(&Theme) -> Style {
    move |_theme: &Theme| Style {
        snap: true,
        text_color: Some(text_main()),
        background: Some(Background::Color(panel_soft())),
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
        background: Some(Background::Color(if is_dark_mode() {
            rgb(34, 22, 30)
        } else {
            rgb(255, 242, 245)
        })),
        border: Border {
            color: if is_dark_mode() {
                rgb(118, 56, 74)
            } else {
                rgb(220, 155, 170)
            },
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
        widget::button::Status::Hovered => (
            brighten(background, 0.035),
            brighten(border, 0.045),
            4.0,
            12.0,
        ),
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
            radius: Radius::from(10.0),
        },
        shadow: Shadow {
            color: shadow_color(),
            offset: Vector::from([0.0, offset_y]),
            blur_radius,
        },
    }
}

fn primary_button<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    widget::button(widget::text(label).size(14).align_x(Horizontal::Center))
        .padding(12)
        .style(|_theme, _status| button_visual(rgb(75, 84, 205), accent(), Color::WHITE, _status))
        .on_press(message)
        .into()
}

fn secondary_button<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    widget::button(widget::text(label).size(14).align_x(Horizontal::Center))
        .padding(12)
        .style(|_theme, _status| button_visual(panel_soft(), border_soft(), text_main(), _status))
        .on_press(message)
        .into()
}

fn danger_button<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    widget::button(widget::text(label).size(14).align_x(Horizontal::Center))
        .padding(12)
        .style(|_theme, _status| {
            button_visual(rgb(104, 38, 55), rgb(185, 76, 99), Color::WHITE, _status)
        })
        .on_press(message)
        .into()
}

fn mini_button<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    widget::button(widget::text(label).size(12).align_x(Horizontal::Center))
        .padding(7)
        .style(|_theme, _status| button_visual(panel_soft(), border_soft(), text_muted(), _status))
        .on_press(message)
        .into()
}

fn mini_button_owned(label: String, message: Message) -> Element<'static, Message> {
    widget::button(widget::text(label).size(12).align_x(Horizontal::Center))
        .padding(7)
        .style(|_theme, _status| button_visual(panel_soft(), border_soft(), text_muted(), _status))
        .on_press(message)
        .into()
}

fn thinking_control<'a>(selected: ThinkingLevel, language: Language) -> Element<'a, Message> {
    let description = match selected {
        ThinkingLevel::Off => "Fastest · no extra reasoning",
        ThinkingLevel::Low => "Quick reasoning for everyday questions",
        ThinkingLevel::Medium => "Balanced for multi-step tasks",
        ThinkingLevel::High => "Most thorough · slower responses",
    };

    widget::column![
        widget::pick_list(
            ThinkingChoice::all(language),
            Some(ThinkingChoice {
                level: selected,
                language,
            }),
            |choice| Message::ThinkingLevelChange(choice.level),
        )
        .placeholder(tr(language, "Thinking"))
        .padding([12, 14])
        .text_size(14)
        .style(pick_list_style)
        .menu_style(pick_list_menu_style)
        .width(Length::Fill),
        Space::new().height(Length::Fixed(6.0)),
        widget::text(tr(language, description))
            .size(11)
            .color(text_faint()),
    ]
    .into()
}

fn image_preview<'a>(
    image: &ChatImage,
    remove_index: Option<usize>,
    language: Language,
) -> Element<'a, Message> {
    let preview = widget::image(image.preview_handle.clone())
        .width(Length::Fixed(160.0))
        .height(Length::Fixed(110.0))
        .content_fit(iced::ContentFit::Contain);
    let footer: Element<Message> = if let Some(index) = remove_index {
        widget::row![
            widget::text(ellipsize_chat_title(&image.name, 16))
                .size(11)
                .color(text_muted())
                .wrapping(Wrapping::None),
            Space::new().width(Length::Fill),
            mini_button(tr(language, "Remove"), Message::RemoveImage(index)),
        ]
        .into()
    } else {
        widget::text(ellipsize_chat_title(
            &format!("{} · {}", image.name, image.mime_type),
            22,
        ))
        .size(11)
        .color(text_muted())
        .wrapping(Wrapping::None)
        .into()
    };
    container(widget::column![
        preview,
        Space::new().height(Length::Fixed(6.0)),
        footer
    ])
    .padding(8)
    .width(Length::Fixed(176.0))
    .clip(true)
    .style(flat_card_style)
    .into()
}

fn image_previews<'a>(
    images: &[ChatImage],
    removable: bool,
    language: Language,
) -> Element<'a, Message> {
    if images.is_empty() {
        return widget::column![].into();
    }

    let previews = widget::Column::with_children(
        images
            .iter()
            .enumerate()
            .map(|(index, image)| image_preview(image, removable.then_some(index), language))
            .collect::<Vec<_>>(),
    )
    .spacing(iced::Pixels(6.0));

    widget::scrollable(previews)
        .height(Length::Fixed(150.0))
        .into()
}

fn copy_code_button<'a>(code: String, copied: bool, language: Language) -> Element<'a, Message> {
    let label = tr(language, if copied { "Copied ✓" } else { "Copy code" });

    widget::button(widget::text(label).size(12).align_x(Horizontal::Center))
        .padding(8)
        .style(move |_theme, status| {
            if copied {
                button_visual(rgb(31, 92, 63), rgb(93, 225, 144), Color::WHITE, status)
            } else {
                button_visual(panel_soft(), border_soft(), text_muted(), status)
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
        background: Background::Color(panel_soft()),
        border: Border {
            color: border_soft(),
            width: 1.0,
            radius: Radius::from(14.0),
        },
        icon: text_muted(),
        placeholder: text_faint(),
        value: text_main(),
        selection: accent(),
    }
}

fn section_title<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    widget::column![
        widget::text(title).size(27).color(text_main()),
        Space::new().height(Length::Fixed(5.0)),
        widget::text(subtitle).size(14).color(text_muted()),
    ]
    .into()
}

fn setting_label<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    widget::column![
        widget::text(title).size(16).color(text_main()),
        Space::new().height(Length::Fixed(4.0)),
        widget::text(subtitle).size(12).color(text_muted()),
    ]
    .width(Length::Fill)
    .into()
}

fn help_card<'a>(title: &'a str, body: &'a str, color: Color) -> Element<'a, Message> {
    container(widget::column![
        container(widget::text(title).size(16).color(text_main()))
            .padding(8)
            .style(chip_style(color)),
        Space::new().height(Length::Fixed(10.0)),
        widget::text(body).size(14).color(text_muted()),
    ])
    .padding(16)
    .width(Length::Fill)
    .style(flat_card_style)
    .into()
}

fn website_host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .map(|host| host.strip_prefix("www.").unwrap_or(&host).to_string())
        .unwrap_or_else(|| url.to_string())
}

fn ellipsize_chat_title(title: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let single_line = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = single_line.chars();
    let mut shortened = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        shortened.pop();
        shortened.push('…');
    }

    shortened
}

fn website_result_row<'a>(
    index: usize,
    source: WebSource,
    active: bool,
    language: Language,
) -> Element<'a, Message> {
    let host = website_host(&source.url);
    let title = ellipsize_chat_title(&source.title, 64);
    let host = ellipsize_chat_title(&host, 64);
    let trailing = if active {
        tr(language, "READING").to_string()
    } else {
        "↗".to_string()
    };
    let trailing_color = if active { accent_2() } else { text_faint() };

    container(
        widget::button(widget::row![
            container(
                widget::text((index + 1).to_string())
                    .size(11)
                    .color(if active { accent_2() } else { text_muted() })
                    .align_x(Horizontal::Center)
            )
            .padding([4, 7])
            .style(chip_style(if active {
                accent_2()
            } else {
                border_bright()
            })),
            Space::new().width(Length::Fixed(9.0)),
            widget::column![
                widget::text(title)
                    .size(12)
                    .color(text_main())
                    .wrapping(Wrapping::None),
                Space::new().height(Length::Fixed(2.0)),
                widget::text(host)
                    .size(11)
                    .color(text_muted())
                    .wrapping(Wrapping::None),
            ]
            .width(Length::Fill),
            widget::text(trailing).size(10).color(trailing_color),
        ])
        .on_press(Message::OpenSource(source.url))
        .padding(0)
        .style(chat_title_button_style)
        .clip(true)
        .width(Length::Fill),
    )
    .padding(9)
    .width(Length::Fill)
    .style(website_row_style(active))
    .into()
}

fn web_search_activity<'a>(state: WebSearchState, language: Language) -> Element<'a, Message> {
    let (status, detail, count, status_color) = match state {
        WebSearchState::Searching { query } => {
            (tr(language, "Searching the web…"), query, None, accent_2())
        }
        WebSearchState::Results { query, websites } => (
            tr(language, "Reviewing results"),
            query,
            Some(websites.len()),
            warning(),
        ),
        WebSearchState::Fetching {
            url,
            query,
            websites,
        } => (
            tr(language, "Reading website"),
            if query.trim().is_empty() {
                website_host(&url)
            } else {
                query
            },
            Some(websites.len()),
            success(),
        ),
        WebSearchState::Failed { message } => (tr(language, "Web search"), message, None, danger()),
        WebSearchState::Idle | WebSearchState::Completed => return widget::column![].into(),
    };
    let detail = ellipsize_chat_title(&detail, 72);
    let result_count: Element<'a, Message> = count.map_or_else(
        || widget::column![].into(),
        |count| {
            container(
                widget::text(format!("{count} {}", tr(language, "found")))
                    .size(10)
                    .color(status_color),
            )
            .padding([4, 7])
            .style(chip_style(status_color))
            .into()
        },
    );

    container(widget::row![
        widget::text("●").size(10).color(status_color),
        Space::new().width(Length::Fixed(7.0)),
        widget::text(status)
            .size(12)
            .color(status_color)
            .wrapping(Wrapping::None),
        Space::new().width(Length::Fixed(9.0)),
        widget::text(detail)
            .size(12)
            .color(text_muted())
            .wrapping(Wrapping::None)
            .width(Length::Fill),
        result_count,
    ])
    .padding([8, 10])
    .width(Length::Fill)
    .clip(true)
    .style(web_activity_style)
    .into()
}

fn markdown_with_code_copy<'a>(
    items: &'a [markdown::Item],
    text_size: f32,
    copied_text: Option<&String>,
    language: Language,
) -> Element<'a, Message> {
    let settings = iced::widget::markdown::Settings::with_text_size(
        text_size,
        if is_dark_mode() {
            Theme::Dark
        } else {
            Theme::Light
        },
    );

    let mut children: Vec<Element<'a, Message>> = Vec::new();

    for item in items.iter() {
        children.push(selectable_markdown(std::iter::once(item), settings).map(|_| Message::None));

        if let markdown::Item::CodeBlock { code, .. } = item {
            let copied = copied_text.map(|copied| copied == code).unwrap_or(false);

            children.push(
                widget::row![
                    Space::new().width(Length::Fill),
                    copy_code_button(code.clone(), copied, language),
                ]
                .into(),
            );
        }
    }

    widget::Column::with_children(children)
        .spacing(iced::Pixels(8.0))
        .into()
}

#[allow(clippy::too_many_arguments)]
fn message_bubble<'a>(
    index: usize,
    message: Correspondence,
    parsed_markdown: Option<&'a [markdown::Item]>,
    text_size: f32,
    model_name: String,
    copied_text: Option<&String>,
    thinking_expanded: bool,
    language: Language,
) -> Element<'a, Message> {
    match message {
        Correspondence::User { text, images } => widget::row![
            Space::new().width(Length::Fill),
            container(widget::column![
                widget::text(tr(language, "You"))
                    .size(12)
                    .color(if is_dark_mode() {
                        rgb(205, 221, 255)
                    } else {
                        rgb(55, 72, 150)
                    })
                    .align_x(Horizontal::Right),
                Space::new().height(Length::Fixed(6.0)),
                image_previews(&images, false, language),
                widget::text(text)
                    .size(text_size)
                    .align_x(Horizontal::Right),
            ])
            .padding(14)
            .width(Length::Shrink)
            .style(user_bubble_style),
            Space::new().width(Length::Fixed(8.0)),
        ]
        .into(),

        Correspondence::Bot {
            text,
            thinking_seconds,
            sources,
            web_search_used,
            ..
        } => {
            let (thinking, fallback_text) = split_thinking_text(&text);

            let body: Element<'a, Message> = if let Some(parsed) = parsed_markdown {
                markdown_with_code_copy(parsed, text_size, copied_text, language)
            } else {
                widget::text(fallback_text)
                    .size(text_size)
                    .color(text_main())
                    .align_x(Horizontal::Left)
                    .into()
            };

            let reasoning: Element<'a, Message> = if thinking.is_empty() {
                widget::column![].into()
            } else {
                let label = if let Some(seconds) = thinking_seconds {
                    if thinking_expanded {
                        format!(
                            "▾ {}",
                            if language == Language::Spanish {
                                format!("Razonó durante {seconds} segundos")
                            } else {
                                format!("Thought for {seconds} seconds")
                            }
                        )
                    } else if language == Language::Spanish {
                        format!("▸ Razonó durante {seconds} segundos")
                    } else {
                        format!("▸ Thought for {seconds} seconds")
                    }
                } else if thinking_expanded {
                    tr(language, "▾ Hide thinking").to_string()
                } else {
                    tr(language, "▸ Show thinking").to_string()
                };
                let details: Element<'a, Message> = if thinking_expanded {
                    container(
                        widget::text(thinking)
                            .size(text_size - 1.0)
                            .color(text_muted()),
                    )
                    .padding(12)
                    .width(Length::Fill)
                    .style(flat_card_style)
                    .into()
                } else {
                    widget::column![].into()
                };
                widget::column![
                    mini_button_owned(label, Message::ToggleThinking(index)),
                    details,
                    Space::new().height(Length::Fixed(7.0)),
                ]
                .into()
            };

            let source_list: Element<'a, Message> = if sources.is_empty() {
                widget::column![].into()
            } else {
                let entries = sources
                    .into_iter()
                    .enumerate()
                    .map(|(index, source)| website_result_row(index, source, false, language))
                    .collect::<Vec<Element<'a, Message>>>();
                widget::column![
                    Space::new().height(Length::Fixed(12.0)),
                    widget::text(tr(language, "Sources"))
                        .size(11)
                        .color(text_muted()),
                    Space::new().height(Length::Fixed(5.0)),
                    widget::Column::with_children(entries).spacing(iced::Pixels(5.0)),
                ]
                .into()
            };

            widget::row![
                Space::new().width(Length::Fixed(8.0)),
                container(widget::column![
                    widget::row![
                        widget::text(model_name).size(12).color(accent_2()),
                        Space::new().width(Length::Fill),
                        if web_search_used {
                            container(widget::text(tr(language, "WEB")).size(10).color(success()))
                                .padding([4, 7])
                                .style(chip_style(success()))
                        } else {
                            container(widget::text("")).padding(0)
                        },
                    ],
                    Space::new().height(Length::Fixed(7.0)),
                    reasoning,
                    body,
                    source_list,
                ])
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
        let language = self.user_information.language;
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
                                    widget::text(tr(language, "A polished desktop interface for chatting with local Ollama models."))
                                        .size(14)
                                        .color(text_muted()),
                                ]
                                .width(Length::Fill),

                                container(
                                    widget::text(tr(language, "HELP"))
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
                                        tr(language, "Chat locally"),
                                        tr(language, "Select one of your installed Ollama models, type a prompt, and press Enter to generate a response."),
                                        accent(),
                                    ),
                                    Space::new().width(Length::Fixed(12.0)),
                                    help_card(
                                        tr(language, "Manage models"),
                                        tr(language, "Use Advanced Settings to install models by name, change the Ollama address, or tune response rendering."),
                                        accent_2(),
                                    ),
                                ],

                                Space::new().height(Length::Fixed(12.0)),

                                widget::row![
                                    help_card(
                                        tr(language, "System prompts"),
                                        tr(language, "System prompts let you switch the assistant's behaviour or personality without rewriting your prompt each time."),
                                        warning(),
                                    ),
                                    Space::new().width(Length::Fixed(12.0)),
                                    help_card(
                                        tr(language, "Chat history"),
                                        tr(language, "When enabled, conversations can be saved locally. You can wipe the current history from Settings."),
                                        danger(),
                                    ),
                                ],

                                Space::new().height(Length::Fixed(16.0)),

                                container(
                                    widget::column![
                                    widget::text(tr(language, "Files and configuration"))
                                            .size(17)
                                            .color(text_main()),
                                        Space::new().height(Length::Fixed(8.0)),
                                        widget::text(
                                            tr(language, "User settings, logs, generated images, and chats are stored in your local application-data folder. Installed assets remain read-only.")
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
                            primary_button(tr(language, "Back to chat"), Message::ToggleInfoPopup),
                        ],
                    ]
                )
                .padding(0)
                .width(Length::Fill);

                container(widget::scrollable(content).height(Length::Fill))
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(app_background_style)
            }

            GUIState::Main => {
                let user_information = self.user_information.clone();
                let bots_list = self.app_state.bots_list.lock().unwrap().clone();
                let copied_text = self.last_copied_text.clone();
                let local_ollamastate = self.app_state.ollama_state.lock().unwrap().clone();
                let active_prompt = self.current_active_prompt();
                let is_processing = active_prompt.is_some();
                let current_web_search_enabled = active_prompt
                    .map(|job| job.web_search_enabled)
                    .unwrap_or(self.web_search_for_chat);
                let web_search_state = active_prompt
                    .map(|job| job.web_search_state.clone())
                    .unwrap_or(WebSearchState::Idle);
                let response_text = active_prompt
                    .and_then(|job| job.response_as_string.lock().ok())
                    .map(|response| response.clone())
                    .unwrap_or_default();
                let (live_thinking, _) = split_thinking_text(&response_text);

                let chat_messages = {
                    let chat_history = self.user_information.chat_history.lock().unwrap();
                    chat_history.messages.clone()
                };

                let latest_bot_text = chat_messages
                    .iter()
                    .rev()
                    .find_map(|message| match message {
                        Correspondence::Bot { text, .. } => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let (_, latest_visible_bot_text) = split_thinking_text(&latest_bot_text);
                let (_, live_visible_response) = split_thinking_text(&response_text);

                let latest_response_to_copy = if !response_text.trim().is_empty() {
                    live_visible_response
                } else {
                    latest_visible_bot_text
                };

                let selected_model = self.user_information.model.clone();
                let active_model_name = selected_model
                    .clone()
                    .unwrap_or_else(|| tr(language, "No model selected").to_string());

                let prompt = iced::widget::TextInput::<Message>::new(
                    tr(language, "Ask something..."),
                    &self.prompt.prompt,
                )
                .padding(14)
                .size(18)
                .width(Length::Fill)
                .on_submit(Message::Prompt(self.prompt.prompt.clone()))
                .on_input(Message::UpdatePrompt)
                .style(text_input_style);
                let web_toggle: Element<Message> = if is_processing {
                    container(
                        widget::text(if current_web_search_enabled {
                            tr(language, "Web on")
                        } else {
                            tr(language, "Web off")
                        })
                        .size(12)
                        .color(text_muted()),
                    )
                    .padding([7, 9])
                    .style(chip_style(text_muted()))
                    .into()
                } else {
                    mini_button(
                        if current_web_search_enabled {
                            tr(language, "Web on")
                        } else {
                            tr(language, "Web off")
                        },
                        Message::ToggleChatWebSearch,
                    )
                };

                let chat_widgets: Vec<Element<Message>> = chat_messages
                    .iter()
                    .enumerate()
                    .flat_map(|(index, message)| {
                        let parsed_markdown =
                            self.chat_markdown_cache.get(index).map(Vec::as_slice);
                        let message_model_name = self
                            .chat_model_name_cache
                            .get(index)
                            .cloned()
                            .flatten()
                            .unwrap_or_else(|| active_model_name.clone());

                        vec![
                            message_bubble(
                                index,
                                message.clone(),
                                parsed_markdown,
                                user_information.text_size,
                                message_model_name,
                                copied_text.as_ref(),
                                self.expanded_thinking.contains(&index),
                                language,
                            ),
                            Space::new().height(Length::Fixed(10.0)).into(),
                        ]
                    })
                    .collect();

                let online = local_ollamastate.to_lowercase() != "offline";
                let status_color = if online { success() } else { danger() };
                let visible_debug = self.current_debug_message().clone();
                let debug_color = if visible_debug.is_error {
                    danger()
                } else {
                    success()
                };

                let model_selector: Element<Message> = if bots_list.is_empty() {
                    container(
                        widget::text(tr(language, "No models installed"))
                            .size(13)
                            .color(text_muted()),
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
                    .padding([12, 14])
                    .text_size(14)
                    .style(pick_list_style)
                    .menu_style(pick_list_menu_style)
                    .width(Length::Fill)
                    .into()
                };

                let thinking_selector: Element<Message> = widget::pick_list(
                    ThinkingChoice::all(language),
                    Some(ThinkingChoice {
                        level: self.user_information.thinking_level,
                        language,
                    }),
                    |choice| Message::ThinkingLevelChange(choice.level),
                )
                .placeholder(tr(language, "Thinking"))
                .padding([12, 14])
                .text_size(14)
                .style(pick_list_style)
                .menu_style(pick_list_menu_style)
                .width(Length::Fixed(132.0))
                .into();

                let live_response: Element<Message> = if let Some(active_prompt) = active_prompt {
                    let response_model_name = active_prompt.model_name.clone();
                    let elapsed_seconds = active_prompt.started_at.elapsed().as_secs();
                    let activity = match &web_search_state {
                        WebSearchState::Searching { .. } => tr(language, "Searching"),
                        WebSearchState::Results { .. } => tr(language, "Reviewing results"),
                        WebSearchState::Fetching { .. } => tr(language, "Reading website"),
                        WebSearchState::Failed { .. } => tr(language, "Web search"),
                        WebSearchState::Idle | WebSearchState::Completed => {
                            if language == Language::Spanish {
                                "Pensando"
                            } else {
                                "Thinking"
                            }
                        }
                    };
                    let label = ellipsize_chat_title(
                        &format!("{response_model_name} · {activity} ({elapsed_seconds}s)"),
                        64,
                    );
                    let web_search_activity_visible = !matches!(
                        &web_search_state,
                        WebSearchState::Idle | WebSearchState::Completed
                    );
                    let search_activity = web_search_activity(web_search_state.clone(), language);
                    let search_gap: Element<Message> = if web_search_activity_visible {
                        Space::new().height(Length::Fixed(8.0)).into()
                    } else {
                        widget::column![].into()
                    };

                    let live_reasoning: Element<Message> = if live_thinking.is_empty() {
                        widget::column![].into()
                    } else {
                        let expanded = self.expanded_thinking.contains(&usize::MAX);
                        let details: Element<Message> = if expanded {
                            container(
                                widget::text(live_thinking.clone())
                                    .size(user_information.text_size - 1.0)
                                    .color(text_muted()),
                            )
                            .padding(12)
                            .width(Length::Fill)
                            .style(flat_card_style)
                            .into()
                        } else {
                            widget::column![].into()
                        };
                        widget::column![
                            mini_button(
                                tr(
                                    language,
                                    if expanded {
                                        "▾ Hide thinking"
                                    } else {
                                        "▸ Show thinking"
                                    }
                                ),
                                Message::ToggleThinking(usize::MAX),
                            ),
                            details,
                            Space::new().height(Length::Fixed(7.0)),
                        ]
                        .into()
                    };

                    widget::row![
                        Space::new().width(Length::Fixed(8.0)),
                        container(widget::column![
                            widget::row![
                                container(
                                    widget::text(label)
                                        .size(12)
                                        .color(accent_2())
                                        .wrapping(Wrapping::None)
                                )
                                .width(Length::Fill)
                                .clip(true),
                                Space::new().width(Length::Fixed(10.0)),
                                widget::progress_bar(0.0..=1.0, self.prompt_progress())
                                    .length(Length::Fixed(96.0))
                                    .girth(Length::Fixed(5.0)),
                            ],
                            Space::new().height(Length::Fixed(8.0)),
                            search_activity,
                            search_gap,
                            live_reasoning,
                            markdown_with_code_copy(
                                &active_prompt.parsed_markdown,
                                user_information.text_size,
                                copied_text.as_ref(),
                                language,
                            ),
                        ])
                        .padding(14)
                        .width(Length::Fill)
                        .style(bot_bubble_style),
                        Space::new().width(Length::Fixed(42.0)),
                    ]
                    .into()
                } else if chat_messages.is_empty() {
                    container(
                        widget::column![
                            widget::text(tr(language, "Ready when you are."))
                                .size(22)
                                .color(text_main())
                                .align_x(Horizontal::Center),
                            Space::new().height(Length::Fixed(8.0)),
                            widget::text(tr(
                                language,
                                "Choose a model, type a prompt, and start chatting locally."
                            ))
                            .size(14)
                            .color(text_muted())
                            .align_x(Horizontal::Center),
                        ]
                        .align_x(Horizontal::Center),
                    )
                    .padding(30)
                    .width(Length::Fill)
                    .style(flat_card_style)
                    .into()
                } else {
                    widget::column![].into()
                };

                let offline_hint: Element<Message> = if !online {
                    container(widget::row![
                        widget::column![
                            widget::text(tr(language, "Ollama was not detected."))
                                .size(14)
                                .color(text_main()),
                            Space::new().height(Length::Fixed(3.0)),
                            widget::text(tr(
                                language,
                                "Install Ollama or check your connection settings."
                            ))
                            .size(12)
                            .color(text_muted()),
                        ]
                        .width(Length::Fill),
                        secondary_button(
                            tr(language, "Install Ollama"),
                            Message::InstallationPrompt
                        ),
                    ])
                    .padding(14)
                    .width(Length::Fill)
                    .style(flat_card_style)
                    .into()
                } else {
                    widget::column![].into()
                };

                let missing_bots_hint: Element<Message> = if bots_list.is_empty() {
                    container(widget::row![
                        widget::column![
                            widget::text(tr(language, "No models were detected."))
                                .size(14)
                                .color(text_main()),
                            Space::new().height(Length::Fixed(3.0)),
                            widget::text(tr(language, "Install a model before sending prompts."))
                                .size(12)
                                .color(text_muted()),
                        ]
                        .width(Length::Fill),
                        secondary_button(tr(language, "Find models"), Message::ListPrompt),
                    ])
                    .padding(14)
                    .width(Length::Fill)
                    .style(flat_card_style)
                    .into()
                } else {
                    widget::column![].into()
                };

                let chat_sidebar: Element<Message> = if self.chat_menu_open {
                    let mut entries: Vec<Element<Message>> = vec![
                        primary_button(tr(language, "＋ New chat"), Message::NewChat),
                        Space::new().height(Length::Fixed(6.0)).into(),
                        secondary_button(
                            if self.temporary_chat {
                                tr(language, "Leave temporary chat")
                            } else {
                                tr(language, "Temporary chat")
                            },
                            Message::ToggleTemporaryChat,
                        ),
                        Space::new().height(Length::Fixed(14.0)).into(),
                    ];
                    let has_temporary_chats = self.temporary_chat
                        || !self.temporary_chats.is_empty()
                        || self.active_prompts.values().any(|job| job.temporary);
                    if has_temporary_chats {
                        entries.push(
                            widget::text(tr(language, "Temporary chats"))
                                .size(13)
                                .color(text_muted())
                                .into(),
                        );
                    }
                    let mut temporary_jobs = self
                        .active_prompts
                        .iter()
                        .filter(|(_, job)| job.temporary)
                        .collect::<Vec<_>>();
                    temporary_jobs.sort_by_key(|(_, job)| job.started_at);
                    for (chat_id, job) in temporary_jobs {
                        let title = job
                            .chat_history
                            .lock()
                            .ok()
                            .and_then(|chat| {
                                chat.messages.iter().find_map(|message| match message {
                                    Correspondence::User { text, .. } => Some(text.clone()),
                                    Correspondence::Bot { .. } => None,
                                })
                            })
                            .unwrap_or_else(|| tr(language, "Temporary chat").to_string());
                        let title = format!("T · {}", ellipsize_chat_title(&title, 16));
                        entries.push(
                            container(widget::row![
                                widget::button(
                                    widget::text(title).size(13).wrapping(Wrapping::None)
                                )
                                .on_press(Message::OpenChat(chat_id.clone()))
                                .style(chat_title_button_style)
                                .clip(true)
                                .width(Length::Fill),
                                widget::progress_bar(0.0..=1.0, self.prompt_progress())
                                    .length(Length::Fixed(42.0))
                                    .girth(Length::Fixed(4.0)),
                            ])
                            .padding(4)
                            .width(Length::Fill)
                            .style(chat_entry_style(chat_id == &self.current_chat_id))
                            .into(),
                        );
                    }
                    let mut temporary_sessions = self.temporary_chats.iter().collect::<Vec<_>>();
                    temporary_sessions.sort_by_key(|(chat_id, _)| *chat_id);
                    for (chat_id, session) in temporary_sessions {
                        let title = session
                            .chat_history
                            .lock()
                            .ok()
                            .and_then(|chat| {
                                chat.messages.iter().find_map(|message| match message {
                                    Correspondence::User { text, .. } => Some(text.clone()),
                                    Correspondence::Bot { .. } => None,
                                })
                            })
                            .unwrap_or_else(|| tr(language, "Temporary chat").to_string());
                        let title = format!("T · {}", ellipsize_chat_title(&title, 16));
                        entries.push(
                            container(widget::row![
                                widget::button(
                                    widget::text(title).size(13).wrapping(Wrapping::None)
                                )
                                .on_press(Message::OpenChat(chat_id.clone()))
                                .style(chat_title_button_style)
                                .clip(true)
                                .width(Length::Fill),
                                mini_button("×", Message::DeleteTemporaryChat(chat_id.clone())),
                            ])
                            .padding(4)
                            .width(Length::Fill)
                            .style(chat_entry_style(chat_id == &self.current_chat_id))
                            .into(),
                        );
                    }
                    if has_temporary_chats {
                        entries.push(Space::new().height(Length::Fixed(8.0)).into());
                    }
                    entries.push(
                        widget::text(tr(language, "Saved chats"))
                            .size(13)
                            .color(text_muted())
                            .into(),
                    );
                    for saved in &self.saved_chats {
                        let selected = saved.id == self.current_chat_id;
                        let title = ellipsize_chat_title(&saved.title, 20);
                        let working = self.active_prompts.contains_key(&saved.id);
                        let working_progress: Element<Message> = if working {
                            widget::progress_bar(0.0..=1.0, self.prompt_progress())
                                .length(Length::Fixed(42.0))
                                .girth(Length::Fixed(4.0))
                                .into()
                        } else {
                            widget::column![].into()
                        };
                        entries.push(
                            container(widget::row![
                                widget::button(
                                    widget::text(title).size(13).wrapping(Wrapping::None)
                                )
                                .on_press(Message::OpenChat(saved.id.clone()))
                                .style(chat_title_button_style)
                                .clip(true)
                                .width(Length::Fill),
                                working_progress,
                                mini_button(
                                    tr(language, if saved.pinned { "Unpin" } else { "Pin" }),
                                    Message::ToggleChatPin(saved.id.clone()),
                                ),
                                mini_button("×", Message::DeleteChat(saved.id.clone())),
                            ])
                            .padding(4)
                            .width(Length::Fill)
                            .style(chat_entry_style(selected))
                            .into(),
                        );
                    }
                    container(widget::column![
                        widget::row![
                            widget::text(tr(language, "Chats"))
                                .size(18)
                                .color(text_main()),
                            Space::new().width(Length::Fill),
                            mini_button("‹", Message::ToggleChatMenu),
                        ],
                        Space::new().height(Length::Fixed(10.0)),
                        widget::scrollable(
                            widget::Column::with_children(entries).spacing(iced::Pixels(6.0))
                        ),
                    ])
                    .padding(10)
                    .width(Length::Fixed(270.0))
                    .height(Length::Fill)
                    .style(panel_style)
                    .into()
                } else {
                    let mut compact_entries: Vec<Element<Message>> = vec![
                        mini_button("☰", Message::ToggleChatMenu),
                        mini_button("+", Message::NewChat),
                        mini_button(
                            if self.temporary_chat { "T ✓" } else { "T" },
                            Message::ToggleTemporaryChat,
                        ),
                        Space::new().height(Length::Fixed(4.0)).into(),
                    ];
                    let mut temporary_jobs = self
                        .active_prompts
                        .iter()
                        .filter(|(_, job)| job.temporary)
                        .collect::<Vec<_>>();
                    temporary_jobs.sort_by_key(|(_, job)| job.started_at);
                    for (chat_id, _) in temporary_jobs {
                        compact_entries.push(
                            container(widget::row![
                                widget::button(
                                    widget::text("T · work").size(11).wrapping(Wrapping::None),
                                )
                                .on_press(Message::OpenChat(chat_id.clone()))
                                .padding([6, 4])
                                .style(chat_title_button_style)
                                .clip(true)
                                .width(Length::Fill),
                                widget::progress_bar(0.0..=1.0, self.prompt_progress())
                                    .length(Length::Fixed(14.0))
                                    .girth(Length::Fixed(3.0)),
                            ])
                            .padding(2)
                            .width(Length::Fill)
                            .style(chat_entry_style(chat_id == &self.current_chat_id))
                            .into(),
                        );
                    }
                    let mut temporary_sessions = self.temporary_chats.keys().collect::<Vec<_>>();
                    temporary_sessions.sort();
                    for chat_id in temporary_sessions {
                        compact_entries.push(
                            container(
                                widget::button(
                                    widget::text("T · done").size(11).wrapping(Wrapping::None),
                                )
                                .on_press(Message::OpenChat(chat_id.clone()))
                                .padding([6, 4])
                                .style(chat_title_button_style)
                                .clip(true)
                                .width(Length::Fill),
                            )
                            .padding(2)
                            .width(Length::Fill)
                            .style(chat_entry_style(chat_id == &self.current_chat_id))
                            .into(),
                        );
                    }
                    for saved in &self.saved_chats {
                        let short_title = ellipsize_chat_title(&saved.title, 8);
                        let working = self.active_prompts.contains_key(&saved.id);
                        let working_progress: Element<Message> = if working {
                            widget::progress_bar(0.0..=1.0, self.prompt_progress())
                                .length(Length::Fixed(14.0))
                                .girth(Length::Fixed(3.0))
                                .into()
                        } else {
                            widget::column![].into()
                        };
                        compact_entries.push(
                            container(widget::row![
                                widget::button(
                                    widget::text(short_title).size(11).wrapping(Wrapping::None),
                                )
                                .on_press(Message::OpenChat(saved.id.clone()))
                                .padding([6, 4])
                                .style(chat_title_button_style)
                                .clip(true)
                                .width(Length::Fill),
                                working_progress,
                            ])
                            .padding(2)
                            .width(Length::Fill)
                            .style(chat_entry_style(saved.id == self.current_chat_id))
                            .into(),
                        );
                    }
                    container(widget::scrollable(
                        widget::Column::with_children(compact_entries).spacing(iced::Pixels(5.0)),
                    ))
                    .padding(6)
                    .width(Length::Fixed(92.0))
                    .height(Length::Fill)
                    .style(panel_style)
                    .into()
                };

                let content = widget::column![
                    container(widget::column![
                        widget::row![
                            widget::column![
                                widget::text("OLLAMA DESKTOP").size(11).color(accent_2()),
                                widget::text(tr(language, "Local workspace"))
                                    .size(20)
                                    .color(text_main()),
                            ],
                            Space::new().width(Length::Fill),
                            container(widget::row![
                                widget::text("●").size(13).color(status_color),
                                Space::new().width(Length::Fixed(6.0)),
                                widget::text(tr(
                                    language,
                                    if online { "Online" } else { "Offline" }
                                ))
                                .size(12)
                                .color(text_muted()),
                            ])
                            .padding([8, 11])
                            .style(chip_style(status_color)),
                            Space::new().width(Length::Fixed(8.0)),
                            container(
                                widget::text(if current_web_search_enabled {
                                    tr(language, "Web on")
                                } else {
                                    tr(language, "Web off")
                                })
                                .size(11)
                                .color(
                                    if current_web_search_enabled {
                                        success()
                                    } else {
                                        text_muted()
                                    }
                                ),
                            )
                            .padding([8, 11])
                            .style(chip_style(
                                if current_web_search_enabled {
                                    success()
                                } else {
                                    text_muted()
                                }
                            )),
                            Space::new().width(Length::Fixed(8.0)),
                            mini_button(tr(language, "Images"), Message::ToggleImages),
                            Space::new().width(Length::Fixed(6.0)),
                            mini_button(tr(language, "Settings"), Message::ToggleSettings),
                        ],
                        Space::new().height(Length::Fixed(14.0)),
                        widget::row![
                            widget::column![
                                widget::text(tr(language, "MODEL"))
                                    .size(10)
                                    .color(text_faint()),
                                Space::new().height(Length::Fixed(5.0)),
                                container(model_selector).width(Length::Fill),
                            ]
                            .width(Length::FillPortion(5)),
                            Space::new().width(Length::Fixed(10.0)),
                            widget::column![
                                widget::text(tr(language, "SYSTEM PROMPT"))
                                    .size(10)
                                    .color(text_faint()),
                                Space::new().height(Length::Fixed(5.0)),
                                widget::pick_list(
                                    self.system_prompt
                                        .system_prompts_as_vec
                                        .lock()
                                        .unwrap()
                                        .clone(),
                                    self.system_prompt.system_prompt.clone(),
                                    Message::SystemPromptChange,
                                )
                                .placeholder(tr(language, "System prompt"))
                                .padding([12, 14])
                                .text_size(14)
                                .style(pick_list_style)
                                .menu_style(pick_list_menu_style)
                                .width(Length::Fill),
                            ]
                            .width(Length::FillPortion(3)),
                            Space::new().width(Length::Fixed(10.0)),
                            widget::column![
                                widget::text(tr(language, "REASONING"))
                                    .size(10)
                                    .color(text_faint()),
                                Space::new().height(Length::Fixed(5.0)),
                                thinking_selector,
                            ]
                            .width(Length::Fixed(150.0)),
                        ],
                    ])
                    .padding(16)
                    .width(Length::Fill)
                    .style(panel_style),
                    Space::new().height(Length::Fixed(10.0)),
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
                    .padding(16)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(panel_style),
                    Space::new().height(Length::Fixed(10.0)),
                    container(widget::column![
                        image_previews(&self.pending_images, true, language),
                        container(widget::row![
                            mini_button(tr(language, "＋ Image"), Message::PickImage),
                            Space::new().width(Length::Fixed(6.0)),
                            web_toggle,
                            Space::new().width(Length::Fixed(6.0)),
                            prompt,
                            Space::new().width(Length::Fixed(6.0)),
                            if is_processing {
                                primary_button(tr(language, "Stop"), Message::StopResponse)
                            } else {
                                primary_button(
                                    tr(language, "Send"),
                                    Message::Prompt(self.prompt.prompt.clone()),
                                )
                            },
                        ])
                        .padding(6)
                        .style(input_shell_style),
                        widget::row![
                            Space::new().width(Length::Fill),
                            mini_button(tr(language, "Paste image"), Message::PasteImage),
                            Space::new().width(Length::Fixed(6.0)),
                            mini_button(
                                tr(language, "Copy response"),
                                Message::CopyPressed(latest_response_to_copy)
                            ),
                        ],
                        offline_hint,
                        missing_bots_hint,
                        widget::row![
                            widget::text(visible_debug.message)
                                .size(13)
                                .color(debug_color),
                        ],
                    ])
                    .padding(12)
                    .width(Length::Fill)
                    .style(panel_style),
                ]
                .spacing(iced::Pixels(0.0));

                container(widget::row![
                    chat_sidebar,
                    Space::new().width(Length::Fixed(8.0)),
                    content
                ])
                .padding(8)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(app_background_style)
            }

            GUIState::Images => {
                let bots_list = self.app_state.bots_list.lock().unwrap().clone();
                let selected_model = self.user_information.model.clone();
                let active_prompt = self.current_active_prompt();
                let vision_is_live = active_prompt.is_some_and(|job| job.had_image);
                let completed_vision = self.vision_responses.get(&self.current_chat_id);
                let visible_debug = self.current_debug_message().clone();
                let model_selector =
                    widget::pick_list(bots_list, selected_model, Message::ModelChange)
                        .padding([12, 14])
                        .text_size(14)
                        .style(pick_list_style)
                        .menu_style(pick_list_menu_style)
                        .width(Length::Fill);

                let attachment: Element<Message> = if !self.pending_images.is_empty() {
                    image_previews(&self.pending_images, true, language)
                } else {
                    container(widget::column![
                        widget::text(tr(language, "Add an image for vision"))
                            .size(17)
                            .color(text_main()),
                        Space::new().height(Length::Fixed(5.0)),
                        widget::text(tr(
                            language,
                            "Paste from the clipboard or choose a local image."
                        ))
                        .size(12)
                        .color(text_muted()),
                        Space::new().height(Length::Fixed(14.0)),
                        widget::row![
                            secondary_button(tr(language, "Choose image"), Message::PickImage),
                            Space::new().width(Length::Fixed(8.0)),
                            secondary_button(tr(language, "Paste image"), Message::PasteImage),
                        ],
                    ])
                    .padding(22)
                    .width(Length::Fill)
                    .style(flat_card_style)
                    .into()
                };

                let capability_status: Element<Message> = match self
                    .user_information
                    .vision_supported
                {
                    Some(true) => {
                        container(widget::text(tr(language, "This model can inspect images.")))
                            .padding(9)
                            .style(chip_style(success()))
                            .into()
                    }
                    Some(false) => container(widget::text(tr(
                        language,
                        "This model does not support image input.",
                    )))
                    .padding(9)
                    .style(chip_style(danger()))
                    .into(),
                    None => container(widget::text(tr(language, "Checking image capabilities…")))
                        .padding(9)
                        .style(chip_style(warning()))
                        .into(),
                };

                let vision_prompt = iced::widget::TextInput::<Message>::new(
                    tr(
                        language,
                        "Describe an image, or ask a question about the attached image…",
                    ),
                    &self.prompt.prompt,
                )
                .padding(14)
                .size(16)
                .width(Length::Fill)
                .on_input(Message::UpdatePrompt)
                .style(text_input_style);

                let vision_action: Element<Message> = if vision_is_live {
                    primary_button(tr(language, "Stop"), Message::StopResponse)
                } else if self.user_information.vision_supported != Some(false)
                    && !self.pending_images.is_empty()
                {
                    primary_button(
                        tr(language, "Ask about image"),
                        Message::Prompt(self.prompt.prompt.clone()),
                    )
                } else {
                    widget::column![].into()
                };

                let vision_response: Element<Message> = if vision_is_live
                    || completed_vision.is_some()
                {
                    let markdown = active_prompt
                        .filter(|job| job.had_image)
                        .map(|job| job.parsed_markdown.as_slice())
                        .or_else(|| completed_vision.map(|response| response.markdown.as_slice()))
                        .unwrap_or_default();
                    container(widget::column![
                        widget::row![
                            widget::text(tr(
                                language,
                                if vision_is_live {
                                    "Vision model is responding…"
                                } else {
                                    "Vision response"
                                }
                            ))
                            .size(12)
                            .color(accent_2()),
                            Space::new().width(Length::Fill),
                            if vision_is_live {
                                widget::progress_bar(0.0..=1.0, self.prompt_progress())
                                    .length(Length::Fixed(96.0))
                                    .girth(Length::Fixed(5.0))
                            } else {
                                widget::progress_bar(0.0..=1.0, 0.0)
                                    .length(Length::Fixed(0.0))
                                    .girth(Length::Fixed(0.0))
                            },
                        ],
                        Space::new().height(Length::Fixed(8.0)),
                        markdown_with_code_copy(
                            markdown,
                            self.user_information.text_size,
                            self.last_copied_text.as_ref(),
                            language,
                        ),
                    ])
                    .padding(14)
                    .width(Length::Fill)
                    .style(bot_bubble_style)
                    .into()
                } else {
                    widget::column![].into()
                };

                let generated_cards: Vec<Element<Message>> = self
                    .generated_images
                    .iter()
                    .rev()
                    .map(|path| {
                        container(widget::column![
                            widget::image(iced::widget::image::Handle::from_path(path))
                                .height(Length::Fixed(280.0))
                                .width(Length::Fill)
                                .content_fit(iced::ContentFit::Contain),
                            Space::new().height(Length::Fixed(8.0)),
                            widget::row![
                                widget::text(path.clone())
                                    .size(11)
                                    .color(text_muted())
                                    .wrapping(Wrapping::WordOrGlyph)
                                    .width(Length::Fill),
                                mini_button(
                                    tr(language, "Copy image"),
                                    Message::CopyImage(path.clone())
                                ),
                            ],
                        ])
                        .padding(12)
                        .width(Length::Fill)
                        .style(flat_card_style)
                        .into()
                    })
                    .collect();

                let generation_panel: Element<Message> = if self
                    .user_information
                    .image_generation_supported
                    == Some(true)
                {
                    let generation_prompt = iced::widget::TextInput::<Message>::new(
                        tr(language, "Describe the image you want to generate…"),
                        &self.prompt.prompt,
                    )
                    .padding(14)
                    .size(16)
                    .width(Length::Fill)
                    .on_input(Message::UpdatePrompt)
                    .style(text_input_style);
                    container(widget::column![
                            setting_label(
                                tr(language, "Experimental image generation"),
                                tr(language, "Ollama reports that this model can generate images. Output is requested through /api/generate at 1024 × 1024.")
                            ),
                            Space::new().height(Length::Fixed(12.0)),
                            generation_prompt,
                            Space::new().height(Length::Fixed(10.0)),
                            primary_button(
                                tr(
                                    language,
                                    if self.is_generating_image {
                                        "Generating…"
                                    } else {
                                        "Generate image"
                                    }
                                ),
                                Message::GenerateImage
                            ),
                        ])
                        .padding(18)
                        .width(Length::Fill)
                        .style(panel_style)
                        .into()
                } else {
                    widget::column![].into()
                };

                let generated_gallery: Element<Message> = if generated_cards.is_empty() {
                    widget::column![].into()
                } else {
                    container(widget::column![
                        widget::text(tr(language, "Generated images"))
                            .size(18)
                            .color(text_main()),
                        Space::new().height(Length::Fixed(10.0)),
                        widget::Column::with_children(generated_cards).spacing(iced::Pixels(12.0)),
                    ])
                    .padding(14)
                    .width(Length::Fill)
                    .style(panel_style)
                    .into()
                };

                let content = widget::column![
                    container(widget::row![
                        section_title(
                            tr(language, "Images"),
                            tr(language, "Analyze images with a vision model. Experimental image generation appears only for models that report support.")
                        ),
                        Space::new().width(Length::Fill),
                        secondary_button(tr(language, "Back to chat"), Message::ToggleImages),
                    ]).padding(18).width(Length::Fill).style(panel_style),
                    Space::new().height(Length::Fixed(14.0)),
                    container(widget::column![
                        setting_label(
                            tr(language, "Vision analysis"),
                            tr(language, "Attach an image and ask a vision-capable model to describe, classify, read, or reason about it.")
                        ),
                        Space::new().height(Length::Fixed(12.0)),
                        widget::text(tr(language, "Model"))
                            .size(12)
                            .color(text_faint()),
                        Space::new().height(Length::Fixed(5.0)),
                        model_selector,
                        Space::new().height(Length::Fixed(10.0)),
                        capability_status,
                        Space::new().height(Length::Fixed(14.0)),
                        attachment,
                        Space::new().height(Length::Fixed(12.0)),
                        vision_prompt,
                        Space::new().height(Length::Fixed(10.0)),
                        vision_action,
                    ]).padding(18).width(Length::Fill).style(panel_style),
                    Space::new().height(Length::Fixed(14.0)),
                    vision_response,
                    Space::new().height(Length::Fixed(14.0)),
                    generation_panel,
                    Space::new().height(Length::Fixed(14.0)),
                    generated_gallery,
                    widget::text(visible_debug.message)
                        .size(13)
                        .color(if visible_debug.is_error { danger() } else { success() }),
                ];

                container(widget::scrollable(content))
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(app_background_style)
            }

            GUIState::Settings => {
                let user_information = self.user_information.clone();
                let web_api_key = self.web_search_settings.api_key.clone().unwrap_or_default();
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
                                tr(language, "Settings"),
                                tr(language, "Tune model behaviour, prompt selection, and chat preferences.")
                            ),
                            Space::new().width(Length::Fill),
                            secondary_button(tr(language, "Go back"), Message::ToggleSettings),
                        ]
                    )
                    .padding(18)
                    .width(Length::Fill)
                    .style(panel_style),

                    Space::new().height(Length::Fixed(14.0)),

                    container(
                        widget::column![
                            container(
                                widget::column![
                                    setting_label(
                                        tr(language, "Interface language"),
                                        tr(language, "Spanish is experimental and machine-generated. It will be replaced with a human translation in a future update.")
                                    ),
                                    widget::pick_list(
                                        [Language::English, Language::Spanish],
                                        Some(language),
                                        Message::LanguageChange,
                                    )
                                    .padding([12, 14])
                                    .text_size(14)
                                    .style(pick_list_style)
                                    .menu_style(pick_list_menu_style)
                                    .width(Length::Fill),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::column![
                                    setting_label(
                                        tr(language, "Maximum response"),
                                        tr(language, "Caps generated output in tokens, including hidden reasoning. The default is 10,240 tokens.")
                                    ),
                                    Space::new().height(Length::Fixed(10.0)),
                                    widget::row![
                                        widget::slider(
                                            512.0..=65_536.0,
                                            self.user_information.max_response_tokens as f32,
                                            Message::UpdateMaxResponseTokens,
                                        )
                                        .step(512.0),
                                        Space::new().width(Length::Fixed(12.0)),
                                        container(
                                            widget::text(format!(
                                                "{} tokens",
                                                self.user_information.max_response_tokens
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
                                widget::column![
                                    setting_label(
                                        tr(language, "Context window"),
                                        tr(language, "Controls how much conversation and generated output the model can hold. Larger values use substantially more memory.")
                                    ),
                                    Space::new().height(Length::Fixed(10.0)),
                                    widget::row![
                                        widget::slider(
                                            4_096.0..=262_144.0,
                                            self.user_information.context_tokens as f32,
                                            Message::UpdateContextTokens,
                                        )
                                        .step(1_024.0),
                                        Space::new().width(Length::Fixed(12.0)),
                                        container(
                                            widget::text(format!(
                                                "{} tokens",
                                                self.user_information.context_tokens
                                            ))
                                            .size(13)
                                            .color(text_main())
                                        )
                                        .padding(8)
                                        .style(chip_style(warning())),
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
                                        tr(language, "Model"),
                                        tr(language, "Choose the Ollama model used for new responses.")
                                    ),
                                    widget::pick_list(
                                        bots_list,
                                        self.user_information.model.clone(),
                                        Message::ModelChange,
                                    )
                                    .padding([12, 14])
                                    .text_size(14)
                                    .style(pick_list_style)
                                    .menu_style(pick_list_menu_style)
                                    .width(Length::Fill),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            if self.user_information.thinking_supported == Some(true) {
                                container(
                                    widget::column![
                                        setting_label(
                                            tr(language, "Thinking effort"),
                                            tr(language, "Choose how much reasoning the model should use.")
                                        ),
                                        Space::new().height(Length::Fixed(10.0)),
                                        thinking_control(
                                            self.user_information.thinking_level,
                                            language,
                                        ),
                                    ]
                                )
                                .padding(16)
                                .width(Length::Fill)
                                .style(flat_card_style)
                            } else {
                                container(
                                    setting_label(
                                        tr(language, "Reasoning"),
                                        if self.user_information.thinking_supported == Some(false) {
                                            tr(language, "This model does not offer adjustable reasoning.")
                                        } else {
                                            tr(language, "Select a model and wait while reasoning support is checked.")
                                        }
                                    )
                                )
                                .padding(16)
                                .width(Length::Fill)
                                .style(flat_card_style)
                            },

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::column![
                                    setting_label(
                                        tr(language, "Temperature"),
                                        tr(language, "Higher values make output more random.")
                                    ),
                                    Space::new().height(Length::Fixed(10.0)),
                                    widget::row![
                                        widget::slider(
                                            0.0..=10.0,
                                            self.user_information.temperature,
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
                                widget::column![
                                    setting_label(
                                        tr(language, "System prompt"),
                                        tr(language, "Choose the personality or instruction profile.")
                                    ),
                                    widget::pick_list(
                                        prompts_list,
                                        self.system_prompt.system_prompt.clone(),
                                        Message::SystemPromptChange,
                                    )
                                    .padding([12, 14])
                                    .text_size(14)
                                    .style(pick_list_style)
                                    .menu_style(pick_list_menu_style)
                                    .width(Length::Fill),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::column![
                                    setting_label(
                                        tr(language, "Text size"),
                                        tr(language, "Adjust chat and response readability.")
                                    ),
                                    Space::new().height(Length::Fixed(10.0)),
                                    widget::row![
                                        widget::slider(
                                            1.0..=40.0,
                                            self.user_information.text_size,
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
                                        tr(language, "Dark mode"),
                                        tr(language, "Switch between the dark and light interface themes.")
                                    ),
                                    widget::checkbox(self.app_state.dark_mode)
                                        .label(tr(language, "Enabled"))
                                        .on_toggle(|_| Message::ToggleDarkMode),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::column![
                                    widget::row![
                                        setting_label(
                                            tr(language, "Enable Web Search"),
                                            tr(language, "Web search may send search queries and webpage URLs to the selected external provider.")
                                        ),
                                        widget::checkbox(self.web_search_settings.enabled)
                                            .label(tr(language, "Enabled"))
                                            .on_toggle(|_| Message::ToggleWebSearch),
                                    ],
                                    Space::new().height(Length::Fixed(12.0)),
                                    setting_label(
                                        tr(language, "Search provider"),
                                        "The provider is contacted only while web search is enabled."
                                    ),
                                    widget::pick_list(
                                        [crate::web_search::WebSearchProviderKind::Brave],
                                        Some(self.web_search_settings.provider),
                                        Message::WebSearchProviderChange,
                                    )
                                    .padding([12, 14])
                                    .text_size(14)
                                    .style(pick_list_style)
                                    .menu_style(pick_list_menu_style)
                                    .width(Length::Fill),
                                    Space::new().height(Length::Fixed(12.0)),
                                    setting_label(
                                        tr(language, "API key"),
                                        tr(language, "Prefer BRAVE_SEARCH_API_KEY for secret storage. A key entered here is stored in the local settings file and never printed in logs.")
                                    ),
                                    iced::widget::TextInput::<Message>::new(
                                        "Brave Search API key",
                                        &web_api_key,
                                    )
                                    .secure(true)
                                    .padding(12)
                                    .on_input(Message::WebSearchApiKeyChange)
                                    .style(text_input_style),
                                    Space::new().height(Length::Fixed(12.0)),
                                    setting_label(
                                        tr(language, "Search result limit"),
                                        "Limits results returned by each model-requested search."
                                    ),
                                    widget::row![
                                        widget::slider(
                                            1.0..=crate::web_search::MAX_RESULT_LIMIT as f32,
                                            self.web_search_settings.result_limit as f32,
                                            Message::WebSearchResultLimitChange,
                                        )
                                        .step(1.0),
                                        Space::new().width(Length::Fixed(12.0)),
                                        container(
                                            widget::text(format!(
                                                "{}",
                                                self.web_search_settings.result_limit
                                            ))
                                            .size(13)
                                            .color(text_main())
                                        )
                                        .padding(8)
                                        .style(chip_style(accent_2())),
                                    ],
                                    Space::new().height(Length::Fixed(12.0)),
                                    widget::row![
                                        setting_label(
                                            tr(language, "Allow a follow-up search"),
                                            tr(language, "Lets the model run one additional, distinct search when the first results are insufficient.")
                                        ),
                                        widget::checkbox(
                                            self.web_search_settings.allow_multiple_searches
                                        )
                                        .label(tr(language, "Enabled"))
                                        .on_toggle(|_| Message::ToggleMultipleWebSearches),
                                    ],
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::row![
                                    widget::column![
                                        setting_label(
                                            tr(language, "Chat storage"),
                                            tr(language, "Saved chats use this folder. The full path is shown so you can always locate them.")
                                        ),
                                        widget::text(self.chat_storage_dir.display().to_string())
                                            .size(12)
                                            .color(text_muted())
                                            .wrapping(Wrapping::WordOrGlyph),
                                    ]
                                    .width(Length::Fill),
                                    secondary_button(
                                        tr(language, "Choose folder"),
                                        Message::ChooseChatFolder
                                    ),
                                ]
                            )
                            .padding(16)
                            .width(Length::Fill)
                            .style(flat_card_style),

                            Space::new().height(Length::Fixed(10.0)),

                            container(
                                widget::row![
                                    setting_label(
                                        tr(language, "Model conversation context"),
                                        tr(language, "Include earlier messages from this chat in the next model request. Saved chats are managed in the left menu.")
                                    ),
                                    widget::checkbox(
                                        user_information.current_chat_history_enabled
                                    )
                                    .label(tr(language, "Enabled"))
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
                                        widget::text(tr(language, "Maintenance"))
                                            .size(16)
                                            .color(text_main()),
                                        Space::new().height(Length::Fixed(4.0)),
                                        widget::text(tr(language, "Clear local conversation data or open deeper configuration options."))
                                            .size(12)
                                            .color(text_muted()),
                                    ]
                                    .width(Length::Fill),

                                    danger_button(
                                        tr(language, "Clear current context"),
                                        Message::WipeChatHistory
                                    ),

                                    Space::new().width(Length::Fixed(10.0)),

                                    secondary_button(
                                        tr(language, "Advanced settings"),
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

                container(widget::scrollable(content).height(Length::Fill))
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(app_background_style)
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
                    tr(language, "Model name, e.g. llama3.2:3b"),
                    &self.installing_model,
                )
                .padding(12)
                .size(15)
                .width(Length::Fill)
                .on_submit(Message::InstallModel(self.installing_model.clone()))
                .on_input(Message::UpdateInstall)
                .style(text_input_style);

                let change_ip = iced::widget::TextInput::<Message>::new(ip.ip.as_str(), &ip.ip)
                    .padding(12)
                    .size(15)
                    .width(Length::FillPortion(3))
                    .on_submit(Message::ChangeIp(ip.ip.clone()))
                    .on_input(Message::ChangeIp)
                    .style(text_input_style);

                let change_port =
                    iced::widget::TextInput::<Message>::new(ip.port.as_str(), &ip.port)
                        .padding(12)
                        .size(15)
                        .width(Length::FillPortion(1))
                        .on_submit(Message::ChangePort(ip.port.clone()))
                        .on_input(Message::ChangePort)
                        .style(text_input_style);

                let content = widget::column![
                    container(widget::row![
                        section_title(
                            tr(language, "Advanced settings"),
                            tr(language, "Install models, change connection settings, and tune rendering.")
                        ),
                        Space::new().width(Length::Fill),
                        secondary_button(tr(language, "Back to settings"), Message::ToggleAdvancedSettings),
                    ])
                    .padding(18)
                    .width(Length::Fill)
                    .style(panel_style),
                    Space::new().height(Length::Fixed(14.0)),
                    container(widget::column![
                        container(widget::column![
                            setting_label(tr(language, "System prompt"), tr(language, "Change the active prompt profile.")),
                            widget::pick_list(
                                prompts_list,
                                self.system_prompt.system_prompt.clone(),
                                Message::SystemPromptChange,
                            )
                            .padding([12, 14])
                            .text_size(14)
                            .style(pick_list_style)
                            .menu_style(pick_list_menu_style)
                            .width(Length::Fill),
                        ])
                        .padding(16)
                        .width(Length::Fill)
                        .style(flat_card_style),
                        Space::new().height(Length::Fixed(10.0)),
                        container(widget::column![
                            setting_label(
                                tr(language, "Install model"),
                                tr(language, "Enter an Ollama model name and press Enter.")
                            ),
                            model_install,
                        ])
                        .padding(16)
                        .width(Length::Fill)
                        .style(flat_card_style),
                        Space::new().height(Length::Fixed(10.0)),
                        container(widget::column![
                            setting_label(
                                tr(language, "Batch tokens"),
                                tr(language, "Tokens per visual update when fast streaming is off. Higher values reduce rendering work.")
                            ),
                            Space::new().height(Length::Fixed(10.0)),
                            widget::row![
                                widget::slider(1.0..=10.0, self.batch_tokens as f32, |value| {
                                    Message::ChangeBatchTokens(value as i32)
                                },),
                                Space::new().width(Length::Fixed(12.0)),
                                container(
                                    widget::text(format!("{}", self.batch_tokens))
                                        .size(13)
                                        .color(text_main())
                                )
                                .padding(8)
                                .style(chip_style(accent())),
                            ],
                        ])
                        .padding(16)
                        .width(Length::Fill)
                        .style(flat_card_style),
                        Space::new().height(Length::Fixed(10.0)),
                        container(widget::row![
                            setting_label(
                                tr(language, "Fast streaming"),
                                tr(language, "Render as soon as the API yields output. Turn off to use token batching.")
                            ),
                            widget::checkbox(self.fast_streaming)
                                .label(tr(language, "Enabled"))
                                .on_toggle(|_| Message::ToggleFastStreaming),
                        ])
                        .padding(16)
                        .width(Length::Fill)
                        .style(flat_card_style),
                        Space::new().height(Length::Fixed(10.0)),
                        container(widget::row![
                            setting_label(
                                tr(language, "Content filtering"),
                                tr(language, "Censor offensive, profane, sexual, and severely inappropriate words with # characters.")
                            ),
                            widget::checkbox(self.app_state.filtering)
                                .label(tr(language, "Enabled"))
                                .on_toggle(|_| Message::ToggleFiltering),
                        ])
                        .padding(16)
                        .width(Length::Fill)
                        .style(flat_card_style),
                        Space::new().height(Length::Fixed(10.0)),
                        container(widget::column![
                            setting_label(
                                tr(language, "Ollama address"),
                                tr(language, "Change the IP address and port used to connect to Ollama.")
                            ),
                            Space::new().height(Length::Fixed(12.0)),
                            widget::row![
                                change_ip,
                                Space::new().width(Length::Fixed(8.0)),
                                widget::text(":").size(20).color(text_muted()),
                                Space::new().width(Length::Fixed(8.0)),
                                change_port,
                            ],
                            Space::new().height(Length::Fixed(12.0)),
                            container(
                                widget::text(if language == Language::Spanish {
                                    format!(
                                        "Dirección actual: {}:{}",
                                        user_information.ip_address.ip,
                                        user_information.ip_address.port
                                    )
                                } else {
                                    format!(
                                        "Current address: {}:{}",
                                        user_information.ip_address.ip,
                                        user_information.ip_address.port
                                    )
                                })
                                .size(13)
                                .color(text_main())
                            )
                            .padding(10)
                            .style(chip_style(accent_2())),
                        ])
                        .padding(16)
                        .width(Length::Fill)
                        .style(flat_card_style),
                    ])
                    .padding(18)
                    .width(Length::Fill)
                    .style(panel_style),
                ];

                container(widget::scrollable(content).height(Length::Fill))
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(app_background_style)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ellipsize_chat_title;

    #[test]
    fn chat_title_ellipsis_is_unicode_safe() {
        assert_eq!(ellipsize_chat_title("🦀 Rustaceans unite", 6), "🦀 Rus…");
    }

    #[test]
    fn chat_title_ellipsis_keeps_short_titles_unchanged() {
        assert_eq!(ellipsize_chat_title("Short title", 20), "Short title");
        assert_eq!(ellipsize_chat_title("12345678", 8), "12345678");
    }

    #[test]
    fn chat_title_ellipsis_forces_titles_onto_one_line() {
        assert_eq!(
            ellipsize_chat_title("first line\nsecond\tline", 18),
            "first line second…"
        );
        assert_eq!(ellipsize_chat_title("anything", 0), "");
    }
}
