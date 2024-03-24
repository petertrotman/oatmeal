use tui_textarea::Input;

use super::BackendResponse;
use super::Message;

pub enum Event {
    BackendMessage(Message),
    BackendPromptResponse(BackendResponse),
    EditorMessage(Message),
    EditPromptReplace(String),
    EditPromptEnd(),
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
