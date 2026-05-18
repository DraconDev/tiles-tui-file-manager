## Goal
Implement two P4 mouse UX features:
1. **FN-043**: Marquee drag selection — rubber-band rect on file list
2. **FN-044**: Undo close tab — Ctrl+Shift+T reopens last closed tab

## Workflow
After each step: `cargo build && cargo test && cargo clippy -- -D warnings`

## Step 1: Marquee drag selection (FN-043)

1. Find `FileViewState` — understand current mouse_state fields
2. Add `drag_start: Option<(u16, u16)>` and `drag_current: Option<(u16, u16)>` to mouse state
3. Find `handle_mouse_event` or similar in events — where mouseDown/mouseDrag/mouseUp are handled
4. On mouseDown in file list area → set drag_start = (col, row)
5. On mouseDrag → update drag_current; call a new render function that draws a dashed rect
6. On mouseUp → compute rows whose rect intersects the drag rect → add to selection
   - Normal drag = add to selection (replace current)
   - Ctrl+drag = toggle individual items
   - Shift+drag = range select from anchor to each item in rect
7. Clear drag state after mouseUp
8. Add unit test: row-rect intersection logic

## Step 2: Undo close tab (FN-044)

1. Find App or TabState struct — where tabs are managed
2. Add `closed_tabs: VecDeque<TabInfo>` field (cap 10)
3. Find tab close handler — push tab info to closed_tabs before removing
4. Add Ctrl+Shift+T event handler
5. Handler: if closed_tabs not empty, pop and reopen tab
6. If empty, do nothing
7. Add unit test: closed_tabs push/pop behavior

## Completion
Mark both tasks done in Fusion