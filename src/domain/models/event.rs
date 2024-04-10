use tui_textarea::Input;

use super::BackendResponse;
use super::Message;

pub use crate::domain::services::edit_prompt::Event as EditPromptEvent;

pub enum Event {
    BackendMessage(Message),
    BackendPromptResponse(BackendResponse),
    EditPrompt(EditPromptEvent),
    KeyboardCharInput(Input),
    KeyboardCTRLC(),
    KeyboardCTRLO(),
    KeyboardCTRLR(),
    KeyboardEnter(),
    KeyboardPaste(String),
    UITick(),
    UIScrollDown(),
    UIScrollUp(),
    UIScrollPageDown(),
    UIScrollPageUp(),
}
