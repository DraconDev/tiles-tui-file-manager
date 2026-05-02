# Sidebar Folder Tree Fixes

## TL;DR

Fix three issues with the new sidebar folder tree: (1) invisible FAVORITES header, (2) wrong sidebar title, (3) single-click navigation needs to be split into arrow-click=expand/collapse, name-click=navigate.

**Deliverables**:
- FOLDERS section renders with visible FAVORITES header below
- Sidebar title shows current directory path (not "FAVORITES")
- Click on ▸/▾ arrow = expand/collapse only
- Click on folder name = navigate to folder

**Estimated Effort**: Medium
**Parallel Execution**: NO — sequential, touches same file

---

## Context

### Original Request
User added a FOLDERS tree section to the sidebar. Three issues surfaced:
1. Favorites section appears to have "gone away" — actually the header is invisible
2. Sidebar title still says "FAVORITES" instead of current path
3. Single click on a folder both expands AND navigates — wants Dolphin-like behavior where arrow click = expand/collapse, name click = navigate

### Current Broken State
**File**: `src/ui/panes/sidebar.rs`

The FOLDERS section was inserted at the top of `draw_sidebar()` (Files view). The code flow is:
1. FOLDERS header + tree items → `sidebar_items` and `sidebar_bounds`
2. `app.sidebar_bounds.push(SidebarBounds { y: area.y, ... })` for FAVORITES header ← **BUG**: only pushed to bounds, not items, and y=area.y is wrong
3. Favorites items pushed to both

**Title**: `Block::default().title_top(Line::from(" FAVORITES "))` at line ~515 — hardcoded.

**Click handler**: `src/events/mod.rs:504-558` — `SidebarTarget::Project` click toggles expand AND navigates unconditionally.

### Research Findings
- `SidebarBounds` has `y`, `index`, `target` — no click-zone info
- Tree items render as: `[indent][marker][icon][name]` where marker is "▸ " or "▾ "
- Need to know where arrow ends to distinguish arrow-click from name-click

---

## Work Objectives

### Core Objective
Fix the sidebar folder tree rendering and interaction to match Dolphin-style behavior.

### Concrete Deliverables
1. `SidebarBounds` struct extended with `arrow_end_x: u16`
2. FOLDERS tree items populate `arrow_end_x` during render
3. FAVORITES header renders visibly at correct Y position
4. Sidebar block title shows current directory path
5. Click handler split: arrow zone = expand/collapse, name zone = navigate

### Definition of Done
- [ ] Run `tiles`, sidebar shows: `┌─ ~/projects ───────┐` with FOLDERS tree and visible FAVORITES section
- [ ] Clicking `▸`/`▾` on a folder toggles expand/collapse
- [ ] Clicking folder name navigates main pane to that folder
- [ ] No compiler warnings

### Must Have
- FOLDERS section with tree at top of sidebar
- Visible section headers (FOLDERS, FAVORITES, RECENT, STORAGES, REMOTES)
- Correct sidebar title
- Split click behavior

### Must NOT Have
- Remove existing functionality (favorites, storage, remotes)
- Change keyboard shortcuts

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (cargo)
- **Automated tests**: NO — manual verification only
- **Agent-Executed QA**: Playwright/tmux verification of UI behavior

### QA Policy
Every task includes agent-executed QA scenarios.

---

## Execution Strategy

### Wave 1 (Sequential — single file, tightly coupled changes)

#### Task 1: Extend SidebarBounds with arrow_end_x
**What to do**:
- Edit `src/state/mod.rs` line ~217: add `pub arrow_end_x: u16` to `SidebarBounds` struct
- Default value: 0 (non-tree items don't use it)

**Must NOT do**:
- Remove existing fields

**Recommended Agent Profile**:
- **Category**: `quick`
- **Skills**: None needed

**Parallelization**:
- **Can Run In Parallel**: NO
- **Blocks**: Task 2

**Acceptance Criteria**:
- [ ] `SidebarBounds` compiles with new field
- [ ] All existing `SidebarBounds` initializers compile (use `..Default::default()` or add `arrow_end_x: 0`)

**QA Scenario**:
```
Scenario: SidebarBounds compiles
  Tool: Bash
  Preconditions: None
  Steps:
    1. cargo check 2>&1
  Expected Result: No errors
  Evidence: terminal output
```

**Commit**: YES
- Message: `fix(sidebar): add arrow_end_x to SidebarBounds`
- Files: `src/state/mod.rs`

---

#### Task 2: Fix sidebar rendering (title, headers, arrow_end_x)
**What to do**:
In `src/ui/panes/sidebar.rs`:

1. **Sidebar title** (line ~511-525): Change from `title_top(" FAVORITES ")` to:
   ```rust
   let title_text = app.current_file_state()
       .map(|fs| fs.current_path.to_string_lossy().to_string())
       .unwrap_or_else(|| "Files".to_string());
   ```
   Then use `title(format!(" {} ", title_text))` instead of `title_top`.

2. **FAVORITES header** (line ~175): Push a visible `ListItem` to `sidebar_items`:
   ```rust
   if show_favorites {
       sidebar_items.push(ListItem::new(""));
       current_y += 1;
       // ... header line ...
       sidebar_items.push(ListItem::new(section_header_line(...)));
       app.sidebar_bounds.push(SidebarBounds { y: current_y, ... });
       current_y += 1;
   }
   ```
   Remove the old invisible bounds-only push.

3. **FOLDERS tree arrow_end_x** (line ~166): When pushing `SidebarBounds` for tree items:
   ```rust
   let arrow_end_x = inner.x + 1 + (depth as u16 * 2) + 2; // indent + marker width
   app.sidebar_bounds.push(SidebarBounds {
       y: current_y,
       index: current_idx,
       target: SidebarTarget::Project(path.clone()),
       arrow_end_x,
   });
   ```
   The marker is 2 chars ("▸ "), and we add 1 for the leading space.

**Must NOT do**:
- Change the tree walking logic
- Remove existing sections

**Recommended Agent Profile**:
- **Category**: `unspecified-high`
- **Skills**: None needed

**Parallelization**:
- **Can Run In Parallel**: NO (depends on Task 1)
- **Blocks**: Task 3

**References**:
- `src/ui/panes/sidebar.rs:175` — broken FAVORITES header
- `src/ui/panes/sidebar.rs:511-525` — block title
- `src/ui/panes/sidebar.rs:149-170` — tree item rendering

**Acceptance Criteria**:
- [ ] Sidebar title shows current path (e.g., "~/projects")
- [ ] FOLDERS section header visible
- [ ] FAVORITES section header visible below FOLDERS
- [ ] Tree items have `arrow_end_x > 0`

**QA Scenario**:
```
Scenario: Sidebar renders correctly
  Tool: interactive_bash
  Preconditions: tiles running in tmux
  Steps:
    1. tmux new-session -d -s tiles-test "tiles"
    2. tmux capture-pane -t tiles-test -p | head -20
  Expected Result: Title shows path, FOLDERS and FAVORITES headers visible
  Evidence: screenshot
```

**Commit**: YES
- Message: `fix(sidebar): fix title, headers, and add arrow_end_x`
- Files: `src/ui/panes/sidebar.rs`

---

#### Task 3: Split click handler (arrow vs name)
**What to do**:
In `src/events/mod.rs` line ~504-558:

Change `SidebarTarget::Project(path)` handling:
```rust
SidebarTarget::Project(path) => {
    if path.is_dir() {
        let clicked_arrow = column < b.arrow_end_x;
        let path_ref = path.clone();
        let was_expanded = app.tree_expanded_folders.contains(&path_ref);
        
        if clicked_arrow {
            // Toggle expand/collapse
            if was_expanded {
                app.tree_expanded_folders.remove(&path_ref);
            } else {
                app.tree_expanded_folders.insert(path.clone());
            }
        } else {
            // Navigate to folder (always, even if already expanded)
            if let Some(fs) = app.current_file_state_mut() {
                fs.current_path = path.clone();
                fs.selection.selected = Some(0);
                fs.selection.anchor = Some(0);
                fs.selection.clear_multi();
                crate::event_helpers::push_history(fs, path.clone());
                let _ = event_tx.try_send(AppEvent::RefreshFiles(
                    app.focused_pane_index,
                ));
            }
            // Also expand if not already expanded
            if !was_expanded {
                app.tree_expanded_folders.insert(path.clone());
            }
        }
        app.sidebar_focus = false;
    } else {
        // ... existing file handling ...
    }
}
```

**Must NOT do**:
- Change behavior for non-Project targets (Favorites, Remotes, etc.)
- Break existing single-click on favorites

**Recommended Agent Profile**:
- **Category**: `unspecified-high`
- **Skills**: None needed

**Parallelization**:
- **Can Run In Parallel**: NO (depends on Task 2)
- **Blocks**: Final verification

**References**:
- `src/events/mod.rs:504` — click handler entry
- `src/events/mod.rs:506` — Project path handling

**Acceptance Criteria**:
- [ ] Clicking arrow toggles expand/collapse without navigating
- [ ] Clicking folder name navigates to folder
- [ ] Clicking already-expanded folder name navigates (doesn't collapse)

**QA Scenario**:
```
Scenario: Arrow click expands without navigating
  Tool: interactive_bash
  Preconditions: tiles running, folder tree visible with collapsed folder
  Steps:
    1. Click on "▸" arrow of a folder
    2. Check: folder expands (▸ → ▾), children appear
    3. Check: main pane path does NOT change
  Expected Result: Folder expanded, path unchanged
  Evidence: screenshot before/after

Scenario: Name click navigates and expands
  Tool: interactive_bash
  Preconditions: tiles running, folder tree visible
  Steps:
    1. Click on folder name (not arrow)
    2. Check: main pane navigates to that folder
    3. Check: folder expands if it wasn't already
  Expected Result: Main pane shows folder contents, folder expanded
  Evidence: screenshot
```

**Commit**: YES
- Message: `feat(sidebar): split click into arrow=expand, name=navigate`
- Files: `src/events/mod.rs`

---

## Final Verification Wave

- [ ] F1. **Render check** — `oracle` reads sidebar.rs draw_sidebar and confirms:
  - Title uses current path
  - FOLDERS header pushed to sidebar_items
  - FAVORITES header pushed to sidebar_items at correct y
  - Tree items have arrow_end_x populated

- [ ] F2. **Interaction check** — `unspecified-high`:
  - Build passes: `cargo build --release`
  - Run tiles, verify sidebar shows tree
  - Click arrow → expand only
  - Click name → navigate

- [ ] F3. **Regression check** — `unspecified-high`:
  - Favorites still clickable and navigate
  - Storage/remotes sections still render
  - Settings toggles still work

---

## Commit Strategy

1. `fix(sidebar): add arrow_end_x to SidebarBounds` — state/mod.rs
2. `fix(sidebar): fix title, headers, and add arrow_end_x` — sidebar.rs
3. `feat(sidebar): split click into arrow=expand, name=navigate` — events/mod.rs

---

## Success Criteria

### Verification Commands
```bash
# Build
cargo build --release

# Run and test manually
tiles
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] Build passes with zero errors
- [ ] Sidebar title shows current path
- [ ] FOLDERS and FAVORITES headers visible
- [ ] Arrow click = expand/collapse only
- [ ] Name click = navigate to folder
