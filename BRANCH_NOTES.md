# Feature: Edit prompt

Allow user to edit the prompt in the editor. Inspiration from :Gcommit in fugitive.vim.

```diagon
User -> UI: /edit<CR>
User -> UI: <C-e>
UI -> AppState: waiting_for_editor
UI -> ActionService: ActionEditPrompt(text)
ActionService -> TempFile: Create
ActionService -> TempFile: Copy Message History + Prompt
ActionService -> Worker: Spawn
ActionService -> Neovim: Call lua edit_prompt(tempfile)
Neovim -> Buffer: Create
TempFile -> Buffer: Open
User -> Buffer: Edit prompt
User -> Buffer: <cmd>w
Buffer -> TempFile: Save
TempFile -> Worker: Notify
Worker -> UI: EventReplacePrompt(text)
User -> Buffer: <cmd>q
User -> UI: (anything)
UI -> AppState: !waiting_for_editor
UI -> ActionService: ActionEditPromptAbort
ActionService -> TempFile: Delete
TempFile -> Worker: Notify (exit)
```

 ┌────┐      ┌──┐             ┌────────┐┌─────────────┐                  ┌────────┐     ┌──────┐┌──────┐┌──────┐
 │User│      │UI│             │AppState││ActionService│                  │TempFile│     │Worker││Neovim││Buffer│
 └─┬──┘      └┬─┘             └───┬────┘└──────┬──────┘                  └───┬────┘     └──┬───┘└──┬───┘└──┬───┘
   │          │                   │            │                             │             │       │       │    
   │/edit<CR> │                   │            │                             │             │       │       │    
   │─────────>│                   │            │                             │             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │  <C-e>   │                   │            │                             │             │       │       │    
   │─────────>│                   │            │                             │             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │waiting_for_editor │            │                             │             │       │       │    
   │          │──────────────────>│            │                             │             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │     ActionEditPrompt(text)     │                             │             │       │       │    
   │          │───────────────────────────────>│                             │             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │           Create            │             │       │       │    
   │          │                   │            │────────────────────────────>│             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │Copy Message History + Prompt│             │       │       │    
   │          │                   │            │────────────────────────────>│             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │                   Spawn     │             │       │       │    
   │          │                   │            │──────────────────────────────────────────>│       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │          Call lua edit_prompt(tempfile)   │       │       │    
   │          │                   │            │──────────────────────────────────────────────────>│       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │                             │             │       │Create │    
   │          │                   │            │                             │             │       │──────>│    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │                             │            Open     │       │    
   │          │                   │            │                             │────────────────────────────>│    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │  Edit prompt                │             │       │       │    
   │──────────────────────────────────────────────────────────────────────────────────────────────────────>│    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │    <cmd>w                   │             │       │       │    
   │──────────────────────────────────────────────────────────────────────────────────────────────────────>│    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │                             │            Save     │       │    
   │          │                   │            │                             │<────────────────────────────│    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │                             │   Notify    │       │       │    
   │          │                   │            │                             │────────────>│       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │      EventReplacePrompt(text)            │             │       │       │    
   │          │<───────────────────────────────────────────────────────────────────────────│       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │    <cmd>q                   │             │       │       │    
   │──────────────────────────────────────────────────────────────────────────────────────────────────────>│    
   │          │                   │            │                             │             │       │       │    
   │(anything)│                   │            │                             │             │       │       │    
   │─────────>│                   │            │                             │             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │!waiting_for_editor│            │                             │             │       │       │    
   │          │──────────────────>│            │                             │             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │     ActionEditPromptAbort      │                             │             │       │       │    
   │          │───────────────────────────────>│                             │             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │           Delete            │             │       │       │    
   │          │                   │            │────────────────────────────>│             │       │       │    
   │          │                   │            │                             │             │       │       │    
   │          │                   │            │                             │Notify (exit)│       │       │    
   │          │                   │            │                             │────────────>│       │       │    
 ┌─┴──┐      ┌┴─┐             ┌───┴────┐┌──────┴──────┐                  ┌───┴────┐     ┌──┴───┐┌──┴───┐┌──┴───┐
 │User│      │UI│             │AppState││ActionService│                  │TempFile│     │Worker││Neovim││Buffer│
 └────┘      └──┘             └────────┘└─────────────┘                  └────────┘     └──────┘└──────┘└──────┘

