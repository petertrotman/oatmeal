# Feature: Edit prompt

Allow user to edit the prompt in the editor. Inspiration from :Gcommit in fugitive.vim.

```diagon
User -> UI: /edit<CR>
User -> UI: <C-e>
UI -> ActionService: ActionEditPrompt(text)
UI -> AppState: waiting_for_editor - stop rendering (loop)
ActionService -> TempFile: Create
ActionService -> TempFile: Copy Message History + Prompt
ActionService -> Neovim: Call lua edit_prompt(tempfile)
Neovim -> Buffer: Create
TempFile -> Buffer: Open
User -> Buffer: Edit prompt
User -> Buffer: Quit
User -> UI: <Enter>
UI -> AppState: !waiting_for_editor - restart rendering
UI -> ActionService: ActionEditPromptEnd
ActionService -> TempFile: Read prompt
ActionService -> TempFile: Delete
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

