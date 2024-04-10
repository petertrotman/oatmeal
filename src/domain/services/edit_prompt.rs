use anyhow::Result;
use tempfile::NamedTempFile;

use itertools::Itertools as _;
use std::io::{BufRead as _, BufReader, Seek as _, Write as _};

use crate::domain::models::{Event, Message};

type EventTx = tokio::sync::mpsc::UnboundedSender<Event>;

#[derive(Debug, Default)]
enum State {
    #[default]
    Inactive,
    Waiting(EventTx, NamedTempFile),
}

#[derive(Default)]
pub struct Service {
    state: State,
}

impl Service {
    pub fn is_active(&self) -> bool {
        return !matches!(self.state, State::Inactive);
    }

    pub async fn start(&mut self, event_tx: &EventTx, messages: &[Message]) -> Result<()> {
        // Create the temp file for writing and launch the editor.
        // Need to block here because a terminal editor will have to render over the
        // top of us, any event capturing or attempts to render will affect the editor's usability.
        // This also means that this function must be called from the main UI loop.

        if self.is_active() {
            let err_msg = "Edit prompt already in progress";
            return send_error(event_tx, &anyhow::anyhow!(err_msg), err_msg);
        }

        let mut temp_file = match create_temp_file(messages) {
            Ok(temp_file) => temp_file,
            Err(err) => return send_error(event_tx, &err, "Could not create temp file"),
        };

        let editor = match get_editor() {
            Ok(editor) => editor,
            Err(err) => return send_error(event_tx, &err, "Could not get editor"),
        };

        // Blocking here until the editor process returns. The process will return when the user
        // closes a terminal editor, but it will also return after the initial launch of a
        // gui text editor (e.g. vscode). Therefore, we cannot assume that the user has
        // finished editing the prompt.
        if let Err(err) = editor.edit_prompt(temp_file.path()).await {
            return send_error(event_tx, &err, "Could not launch editor");
        }

        let prompt = match parse_prompt_file(temp_file.as_file_mut()) {
            Ok(prompt) => prompt,
            Err(err) => return send_error(event_tx, &err, "Could not parse prompt file"),
        };

        if !prompt.is_empty() {
            // If the prompt file has already been edited then the editor must have been a
            // terminal text editor and the user has finished editing.
            event_tx.send(Event::NewPrompt(prompt))?;
            self.state = State::Inactive;
        } else {
            // Otherwise, until the user interacts with oatmeal again we should assume they
            // are still editing in a graphical editor and wait for them to finish.
            self.state = State::Waiting(event_tx.clone(), temp_file)
        };

        return Ok(());
    }

    pub fn finish(&mut self) -> Result<()> {
        let State::Waiting(event_tx, temp_file) = &mut self.state else {
            return Err(anyhow::anyhow!("Edit prompt service not ready to finish"));
        };

        let prompt = match parse_prompt_file(temp_file.as_file_mut()) {
            Ok(prompt) => prompt,
            Err(err) => return send_error(event_tx, &err, "Could not parse prompt file"),
        };

        if !prompt.is_empty() {
            event_tx.send(Event::NewPrompt(prompt))?;
        }

        self.state = State::Inactive;
        return Ok(());
    }

    pub fn cancel(&mut self) -> Result<()> {
        self.state = State::Inactive;
        return Ok(());
    }

    pub fn widget(&self) -> Result<ratatui::widgets::Paragraph<'static>> {
        use ratatui::prelude::Alignment;
        use ratatui::widgets::Block;
        use ratatui::widgets::BorderType;
        use ratatui::widgets::Borders;
        use ratatui::widgets::Padding;
        use ratatui::widgets::Paragraph;

        if !self.is_active() {
            return Err(anyhow::anyhow!("Edit prompt service is not active"));
        }

        return Ok(
            Paragraph::new("Waiting for editor, press Enter to continue.")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .padding(Padding::new(1, 1, 0, 0)),
                )
                .alignment(Alignment::Center),
        );
    }
}

const PROMPT_DELIMETER: &str =
    "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~";
const HINT_TEXT: &str =
    "Write your prompt above the line and save to have it updated in Oatmeal\n\n";

fn create_temp_file(messages: &[Message]) -> Result<tempfile::NamedTempFile> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("oatmeal-prompt")
        .tempfile()?;
    let initial_content = messages
        .iter()
        .map(|Message { author, text, .. }| format!("{author}:\n{text}\n"))
        .chain([format!("{PROMPT_DELIMETER}\n{HINT_TEXT}\n\n")])
        .rev()
        .collect::<String>();
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
