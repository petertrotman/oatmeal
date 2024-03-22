# Feature: Edit prompt

Allow user to edit the prompt in the editor. Inspiration from :Gcommit in fugitive.vim.

```diagon
User -> UI: /edit<CR>
User -> UI: <C-e>
UI -> ActionService: ActionEditPrompt(text)
ActionService -> TempFile: Create
ActionService -> TempFile: Copy Message History + Prompt
ActionService -> Neovim: Call lua edit_prompt(tempfile)
Neovim -> Buffer: Create
TempFile -> Buffer: Open
User -> Buffer: Edit prompt
User -> Buffer: <cmd>w
Buffer -> TempFile: Save
User -> Buffer: <cmd>q
Buffer -> Neovim: BufLeave
Neovim -> ActionService: Return(status)
TempFile -> ActionService: Read prompt
ActionService -> TempFile: Delete
ActionService -> UI: EventReplacePrompt
```


 ┌────┐     ┌──┐             ┌─────────────┐                  ┌────────┐┌──────┐ ┌──────┐
 │User│     │UI│             │ActionService│                  │TempFile││Neovim│ │Buffer│
 └─┬──┘     └┬─┘             └──────┬──────┘                  └───┬────┘└──┬───┘ └──┬───┘
   │         │                      │                             │        │        │    
   │/edit<CR>│                      │                             │        │        │    
   │────────>│                      │                             │        │        │    
   │         │                      │                             │        │        │    
   │  <C-e>  │                      │                             │        │        │    
   │────────>│                      │                             │        │        │    
   │         │                      │                             │        │        │    
   │         │ActionEditPrompt(text)│                             │        │        │    
   │         │─────────────────────>│                             │        │        │    
   │         │                      │                             │        │        │    
   │         │                      │           Create            │        │        │    
   │         │                      │────────────────────────────>│        │        │    
   │         │                      │                             │        │        │    
   │         │                      │Copy Message History + Prompt│        │        │    
   │         │                      │────────────────────────────>│        │        │    
   │         │                      │                             │        │        │    
   │         │                      │    Call lua edit_prompt(tempfile)    │        │    
   │         │                      │─────────────────────────────────────>│        │    
   │         │                      │                             │        │        │    
   │         │                      │                             │        │ Create │    
   │         │                      │                             │        │───────>│    
   │         │                      │                             │        │        │    
   │         │                      │                             │      Open       │    
   │         │                      │                             │────────────────>│    
   │         │                      │                             │        │        │    
   │         │                      │ Edit prompt                 │        │        │    
   │───────────────────────────────────────────────────────────────────────────────>│    
   │         │                      │                             │        │        │    
   │         │                      │    <cmd>w                   │        │        │    
   │───────────────────────────────────────────────────────────────────────────────>│    
   │         │                      │                             │        │        │    
   │         │                      │                             │      Save       │    
   │         │                      │                             │<────────────────│    
   │         │                      │                             │        │        │    
   │         │                      │    <cmd>q                   │        │        │    
   │───────────────────────────────────────────────────────────────────────────────>│    
   │         │                      │                             │        │        │    
   │         │                      │                             │        │BufLeave│    
   │         │                      │                             │        │<───────│    
   │         │                      │                             │        │        │    
   │         │                      │            Return(status)   │        │        │    
   │         │                      │<─────────────────────────────────────│        │    
   │         │                      │                             │        │        │    
   │         │                      │         Read prompt         │        │        │    
   │         │                      │<────────────────────────────│        │        │    
   │         │                      │                             │        │        │    
   │         │                      │           Delete            │        │        │    
   │         │                      │────────────────────────────>│        │        │    
   │         │                      │                             │        │        │    
   │         │  EventReplacePrompt  │                             │        │        │    
   │         │<─────────────────────│                             │        │        │    
 ┌─┴──┐     ┌┴─┐             ┌──────┴──────┐                  ┌───┴────┐┌──┴───┐ ┌──┴───┐
 │User│     │UI│             │ActionService│                  │TempFile││Neovim│ │Buffer│
 └────┘     └──┘             └─────────────┘                  └────────┘└──────┘ └──────┘

