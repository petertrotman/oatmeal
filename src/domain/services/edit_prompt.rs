use itertools::Itertools as _;
use std::io::{BufRead as _, BufReader, Seek as _, Write as _};

use anyhow::Result;
use notify::Watcher as _;

use crate::domain::models::{Event as AppEvent, Message};

type FileWatcher = tokio::task::JoinHandle<Result<()>>;
type EventTx = tokio::sync::mpsc::UnboundedSender<AppEvent>;

#[derive(Debug)]
pub enum Event {
    Begin(EventTx, Vec<Message>),
    AnimationFrame,
    NewPrompt(String),
    Cancel,
    Error(Message),
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Inactive,
    AwaitingAnimationFrame(EventTx, Vec<Message>),
    WatchingFile(FileWatcher),
}

#[derive(Default)]
pub struct Service {
    state: State,
}

impl Service {
    pub fn is_active(&self) -> bool {
        return !matches!(self.state, State::Inactive);
    }

    pub async fn handle_event(&mut self, event: Event) -> Result<()> {
        self.state = match (&self.state, event) {
            (State::Inactive, Event::Begin(event_tx, messages)) => {
                // Send event to allow the render cycle to run at least once to display
                // the waiting for editor notification over the textarea.
                event_tx.send(AppEvent::EditPrompt(Event::AnimationFrame))?;
                State::AwaitingAnimationFrame(event_tx, messages)
            }

            (State::AwaitingAnimationFrame(event_tx, messages), Event::AnimationFrame) => {
                // Create the temp file for writing and launch the editor.
                // Need to block here because a terminal editor will have to render over the
                // top of us, any event capturing or atttempts to render will affect the editor's usability.
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

                let state = if !prompt.is_empty() {
                    // If the prompt file has already been edited then the editor must have been a
                    // terminal text editor and the user has finished editing.
                    event_tx.send(AppEvent::EditPrompt(Event::NewPrompt(prompt)))?;
                    State::Inactive
                } else {
                    // Otherwise, until the user interacts with oatmeal again we should assume they
                    // are still editing in a graphical editor and watch the file for changes.
                    let temp_file_path = temp_file.path().to_owned();
                    let handler = notify_event_handler(temp_file, event_tx.clone());
                    let mut watcher = match notify::recommended_watcher(handler) {
                        Ok(watcher) => watcher,
                        Err(err) => {
                            return send_error(
                                event_tx,
                                &err.into(),
                                "Could not create file watcher",
                            )
                        }
                    };
                    let worker = tokio::spawn(async move {
                        watcher.watch(&temp_file_path, notify::RecursiveMode::NonRecursive)?;
                        return Ok(());
                    });

                    State::WatchingFile(worker)
                };

                state
            }

            (State::WatchingFile(watcher_worker), Event::Cancel) => {
                watcher_worker.abort();
                State::Inactive
            }
            (State::WatchingFile(watcher_worker), Event::Error(_)) => {
                watcher_worker.abort();
                State::Inactive
            }
            (_, Event::Error(_)) => State::Inactive,

            (state, event) => {
                tracing::error!("Invalid event for state: State: {state:?}, Event: {event:?}");
                return Err(anyhow::anyhow!("Invalid event for state"));
            }
        };

        Ok(())
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

fn error_event(message: &str) -> AppEvent {
    return AppEvent::EditPrompt(Event::Error(Message::new_with_type(
        crate::domain::models::Author::Oatmeal,
        crate::domain::models::MessageType::Error,
        message,
    )));
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

fn notify_event_handler(
    mut temp_file: tempfile::NamedTempFile,
    event_tx: EventTx,
) -> impl notify::EventHandler {
    let handler = move |res: notify::Result<notify::Event>| {
        let event = match res {
            Ok(event) => event,
            Err(err) => {
                let _ = send_error(&event_tx, &err.into(), "File watch error");
                return;
            }
        };

        let prompt_result = match event.kind {
            notify::EventKind::Access(_) if !event.need_rescan() => return,
            _ => parse_prompt_file(temp_file.as_file_mut()),
        };

        if let Ok(prompt) = prompt_result {
            let _ = event_tx.send(AppEvent::EditPrompt(Event::NewPrompt(prompt)));
        } else if let Err(err) = prompt_result {
            let _ = send_error(&event_tx, &err, "Prompt parsing error");
        }
    };
    return handler;
}
