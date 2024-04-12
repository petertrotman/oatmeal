use anyhow::Result;
use std::future::Future;

use itertools::Itertools as _;
use std::io::{BufRead as _, BufReader, Seek as _, Write as _};
use std::pin::Pin;

use crate::domain::models::{Event, Message};

#[cfg(test)]
#[path = "edit_prompt_test.rs"]
mod tests;

pub type ActiveService = TypestateService<Parseable>;

pub trait State {}
pub struct TypestateService<S: State> {
    state: S,
}

type EventTx = tokio::sync::mpsc::UnboundedSender<Event>;
type LaunchFnReturn = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
type LaunchFn = Pin<Box<dyn Fn(std::path::PathBuf) -> LaunchFnReturn>>;

pub struct Buildable {}
pub struct Startable<'a> {
    event_tx: EventTx,
    prompt: &'a str,
    messages: &'a [Message],
}
pub struct Launchable<F> {
    event_tx: EventTx,
    temp_file: tempfile::NamedTempFile,
    original_prompt: String,
    launch_editor_fn: F,
}
pub struct Parseable {
    event_tx: EventTx,
    temp_file: tempfile::NamedTempFile,
    original_prompt: String,
}

impl State for Buildable {}
impl State for Startable<'_> {}
impl<F> State for Launchable<F> {}
impl State for Parseable {}

impl<S> TypestateService<S>
where
    S: State,
{
    pub fn build() -> TypestateService<Buildable> {
        return TypestateService {
            state: Buildable {},
        };
    }
}

impl TypestateService<Buildable> {
    pub fn event_tx(self, event_tx: &EventTx) -> TypestateService<Startable<'_>> {
        return TypestateService {
            state: Startable {
                event_tx: event_tx.clone(),
                prompt: "",
                messages: &[],
            },
        };
    }
}

impl<'a> TypestateService<Startable<'a>> {
    pub fn prompt(self, prompt: &'a str) -> TypestateService<Startable<'a>> {
        return TypestateService {
            state: Startable {
                prompt,
                ..self.state
            },
        };
    }

    pub fn messages(self, messages: &'a [Message]) -> TypestateService<Startable<'a>> {
        return TypestateService {
            state: Startable {
                messages,
                ..self.state
            },
        };
    }

    fn create_temp_file(self) -> Result<TypestateService<Launchable<LaunchFn>>> {
        let Startable {
            event_tx,
            prompt,
            messages,
        } = self.state;

        let temp_file = create_temp_file(prompt, messages)?;

        return Ok(TypestateService {
            state: Launchable {
                event_tx,
                original_prompt: prompt.to_owned(),
                temp_file,
                launch_editor_fn: Box::pin(move |p| return Box::pin(launch_editor(p))),
            },
        });
    }

    pub async fn start(self) -> Option<TypestateService<Parseable>> {
        let event_tx = &self.state.event_tx.clone();
        let launchable = match self.create_temp_file() {
            Ok(new_service) => new_service,
            Err(err) => {
                let _ = send_error(event_tx, &err, "could not create temp file");
                return None;
            }
        };
        let parseable = match launchable.launch().await {
            Ok(new_service) => new_service,
            Err(err) => {
                let _ = send_error(event_tx, &err, "could not launch editor");
                return None;
            }
        };
        // try to read the file once
        let maybe_parseable = match parseable.parse() {
            Ok(opt_service) => opt_service,
            Err(err) => {
                let _ = send_error(event_tx, &err, "could not parse prompt file");
                return None;
            }
        };

        return maybe_parseable;
    }
}

impl TypestateService<Launchable<LaunchFn>> {
    async fn launch(self) -> Result<TypestateService<Parseable>> {
        let Launchable {
            event_tx,
            temp_file,
            original_prompt,
            launch_editor_fn,
        } = self.state;

        // Blocking here until the editor process returns. The process will return when the user
        // closes a terminal editor, but it will also return after the initial launch of a
        // gui text editor (e.g. vscode). Therefore, we cannot assume that the user has
        // finished editing the prompt just because the editor process has returned.
        let temp_file_path = temp_file.path().to_owned();
        launch_editor_fn(temp_file_path).await?;

        return Ok(TypestateService {
            state: Parseable {
                event_tx,
                temp_file,
                original_prompt,
            },
        });
    }
}

impl TypestateService<Parseable> {
    fn parse(self) -> Result<Option<TypestateService<Parseable>>> {
        let Parseable {
            event_tx,
            mut temp_file,
            original_prompt,
        } = self.state;

        let prompt = parse_prompt_file(temp_file.as_file_mut())?;
        if prompt == original_prompt {
            // If the prompt has not been changed, we should assume that the user is still editing it
            // in a graphical text editor and we should wait for them to interact with Oatmeal before
            // updating the prompt.
            return Ok(Some(TypestateService {
                state: Parseable {
                    event_tx,
                    temp_file,
                    original_prompt,
                },
            }));
        } else {
            event_tx.send(Event::NewPrompt(prompt))?;
            return Ok(None);
        };
    }

    pub fn finish(self) -> Option<TypestateService<Parseable>> {
        let event_tx = &self.state.event_tx.clone();
        if let Err(err) = self.parse() {
            let _ = send_error(event_tx, &err, "could not parse prompt file");
        }
        return None;
    }

    pub fn widget(&self) -> ratatui::widgets::Paragraph<'static> {
        use ratatui::prelude::Alignment;
        use ratatui::widgets::Block;
        use ratatui::widgets::BorderType;
        use ratatui::widgets::Borders;
        use ratatui::widgets::Padding;
        use ratatui::widgets::Paragraph;

        return Paragraph::new("Waiting for editor, press Enter to continue.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .padding(Padding::new(1, 1, 0, 0)),
            )
            .alignment(Alignment::Center);
    }
}

// #[derive(Debug, Default)]
// enum State {
//     #[default]
//     Inactive,
//     Waiting(EventTx, NamedTempFile, String),
// }
//
// #[derive(Default)]
// pub struct Service {
//     state: State,
// }
//
// impl Service {
//     pub fn is_active(&self) -> bool {
//         return !matches!(self.state, State::Inactive);
//     }
//
//     pub async fn start(
//         &mut self,
//         event_tx: &EventTx,
//         prompt: &[String],
//         messages: &[Message],
//     ) -> Result<()> {
//         // Create the temp file for writing and launch the editor.
//         // Need to block here because a terminal editor will have to render over the
//         // top of us, any event capturing or attempts to render will affect the editor's usability.
//         // This also means that this function must be awaited in the main UI loop.
//
//         if self.is_active() {
//             let err_msg = "Edit prompt already in progress";
//             return send_error(event_tx, &anyhow::anyhow!(err_msg), err_msg);
//         }
//
//         let original_prompt = prompt.join("\n");
//         let mut temp_file = match create_temp_file(&original_prompt, messages) {
//             Ok(temp_file) => temp_file,
//             Err(err) => return send_error(event_tx, &err, "Could not create temp file"),
//         };
//
//         let editor = match get_editor() {
//             Ok(editor) => editor,
//             Err(err) => return send_error(event_tx, &err, "Could not get editor"),
//         };
//
//         // Blocking here until the editor process returns. The process will return when the user
//         // closes a terminal editor, but it will also return after the initial launch of a
//         // gui text editor (e.g. vscode). Therefore, we cannot assume that the user has
//         // finished editing the prompt just because the editor process has returned.
//         if let Err(err) = editor.edit_prompt(temp_file.path()).await {
//             return send_error(event_tx, &err, "Could not launch editor");
//         }
//
//         let prompt = match parse_prompt_file(temp_file.as_file_mut()) {
//             Ok(prompt) => prompt,
//             Err(err) => return send_error(event_tx, &err, "Could not parse prompt file"),
//         };
//
//         if prompt == original_prompt {
//             // If the prompt has not been changed, we should assume that the user is still editing it
//             // in a graphical text editor and we should wait for them to interact with Oatmeal before
//             // updating the prompt.
//             self.state = State::Waiting(event_tx.clone(), temp_file, original_prompt);
//         } else {
//             // If the prompt file has already been changed, then the editor must have been a
//             // terminal text editor and the user has finished editing.
//             event_tx.send(Event::NewPrompt(prompt))?;
//             self.state = State::Inactive;
//         };
//
//         return Ok(());
//     }
//
//     pub fn finish(&mut self) -> Result<()> {
//         let State::Waiting(event_tx, temp_file, original_prompt) = &mut self.state else {
//             return Err(anyhow::anyhow!("Edit prompt service not ready to finish"));
//         };
//
//         let prompt = match parse_prompt_file(temp_file.as_file_mut()) {
//             Ok(prompt) => prompt,
//             Err(err) => return send_error(event_tx, &err, "Could not parse prompt file"),
//         };
//
//         if &prompt != original_prompt {
//             event_tx.send(Event::NewPrompt(prompt))?;
//         }
//
//         self.state = State::Inactive;
//         return Ok(());
//     }
//
//     pub fn cancel(&mut self) -> Result<()> {
//         self.state = State::Inactive;
//         return Ok(());
//     }
//
//     pub fn widget(&self) -> Result<ratatui::widgets::Paragraph<'static>> {
//         use ratatui::prelude::Alignment;
//         use ratatui::widgets::Block;
//         use ratatui::widgets::BorderType;
//         use ratatui::widgets::Borders;
//         use ratatui::widgets::Padding;
//         use ratatui::widgets::Paragraph;
//
//         if !self.is_active() {
//             return Err(anyhow::anyhow!("Edit prompt service is not active"));
//         }
//
//         return Ok(
//             Paragraph::new("Waiting for editor, press Enter to continue.")
//                 .block(
//                     Block::default()
//                         .borders(Borders::ALL)
//                         .border_type(BorderType::Double)
//                         .padding(Padding::new(1, 1, 0, 0)),
//                 )
//                 .alignment(Alignment::Center),
//         );
//     }
// }

const PROMPT_DELIMETER: &str =
    "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~";
const HINT_TEXT: &str =
    "Write your prompt above the line and save to have it updated in Oatmeal\n\n";

fn create_temp_file(prompt: &str, messages: &[Message]) -> Result<tempfile::NamedTempFile> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("oatmeal-prompt")
        .tempfile()?;

    let reversed_messages = messages
        .iter()
        .rev()
        .take(100) // TODO: Make this configurable
        .map(|Message { author, text, .. }| format!("{author}:\n{text}\n"))
        .collect::<Vec<_>>();

    let prompt_with_newline = if prompt.is_empty() { "\n" } else { prompt };
    let initial_content = prompt_with_newline
        .lines()
        .chain([PROMPT_DELIMETER, HINT_TEXT])
        .chain(reversed_messages.iter().map(String::as_str))
        .join("\n");

    temp_file.write_all(initial_content.as_bytes())?;
    return Ok(temp_file);
}

fn error_event(message: &str) -> Event {
    return Event::EditPromptMessage(Message::new_with_type(
        crate::domain::models::Author::Oatmeal,
        crate::domain::models::MessageType::Error,
        message,
    ));
}

fn send_error(event_tx: &EventTx, error: &anyhow::Error, message: &str) -> Result<()> {
    tracing::error!("{message}: {error}");
    event_tx.send(error_event(message))?;
    return Ok(());
}

fn get_editor() -> Result<crate::domain::models::EditorBox> {
    use crate::configuration::{Config, ConfigKey};
    use crate::domain::models::EditorName;
    use crate::infrastructure::editors::EditorManager;

    let editor_name = EditorName::parse(Config::get(ConfigKey::Editor)).unwrap();
    let editor = EditorManager::get(editor_name.clone())?;

    return Ok(editor);
}

async fn launch_editor(temp_file_path: std::path::PathBuf) -> Result<()> {
    let editor = get_editor()?;
    return editor.edit_prompt(&temp_file_path).await;
}

fn parse_prompt_file(prompt_file: &mut std::fs::File) -> Result<String> {
    prompt_file.rewind()?;

    let reader = BufReader::new(prompt_file);
    let prompt = reader
        .lines()
        .map_while(Result::ok)
        .take_while(|line| return line != PROMPT_DELIMETER)
        .join("\n");

    return Ok(prompt);
}
