#!/usr/bin/env python3
"""Migrate field references from flat App to sub-structs."""

import re, sys
from pathlib import Path

# Mapping: old_field -> (sub_struct, field_in_substruct)
FIELDS = {
    # AppCore
    'app.running':        ('core', 'running'),
    'app.current_view':   ('core', 'current_view'),
    'app.mode':           ('core', 'mode'),
    'app.previous_mode':  ('core', 'previous_mode'),
    'app.input':         ('core', 'input'),
    'app.icon_mode':     ('core', 'icon_mode'),
    'app.is_split_mode': ('core', 'is_split_mode'),
    'app.terminal_size': ('core', 'terminal_size'),
    'app.mouse_pos':     ('core', 'mouse_pos'),
    # SidebarState
    'app.show_sidebar':             ('sidebar', 'show_sidebar'),
    'app.sidebar_focus':             ('sidebar', 'sidebar_focus'),
    'app.sidebar_index':            ('sidebar', 'sidebar_index'),
    'app.sidebar_folders':          ('sidebar', 'sidebar_folders'),
    'app.sidebar_favorites':        ('sidebar', 'sidebar_favorites'),
    'app.sidebar_recent':           ('sidebar', 'sidebar_recent'),
    'app.sidebar_storage':          ('sidebar', 'sidebar_storage'),
    'app.sidebar_remotes':          ('sidebar', 'sidebar_remotes'),
    'app.show_side_panel':          ('sidebar', 'show_side_panel'),
    'app.sidebar_width_percent':    ('sidebar', 'sidebar_width_percent'),
    'app.sidebar_bounds':           ('sidebar', 'sidebar_bounds'),
    'app.sidebar_scroll_offset':   ('sidebar', 'sidebar_scroll_offset'),
    'app.tree_expanded_folders':   ('sidebar', 'tree_expanded_folders'),
    'app.sidebar_tree_cache':       ('sidebar', 'sidebar_tree_cache'),
    'app.sidebar_tree_cache_key':  ('sidebar', 'sidebar_tree_cache_key'),
    'app.editor_sidebar_cache':     ('sidebar', 'editor_sidebar_cache'),
    'app.editor_sidebar_cache_key': ('sidebar', 'editor_sidebar_cache_key'),
    # MonitorState
    'app.monitor_subview':          ('monitor', 'monitor_subview'),
    'app.monitor_subview_bounds':   ('monitor', 'monitor_subview_bounds'),
    'app.overview_scroll_offset':   ('monitor', 'overview_scroll_offset'),
    'app.process_sort_col':         ('monitor', 'process_sort_col'),
    'app.process_sort_asc':          ('monitor', 'process_sort_asc'),
    'app.process_column_bounds':    ('monitor', 'process_column_bounds'),
    'app.process_selected_idx':     ('monitor', 'process_selected_idx'),
    'app.process_table_state':      ('monitor', 'process_table_state'),
    'app.process_search_filter':    ('monitor', 'process_search_filter'),
    'app.process_tree_view':        ('monitor', 'process_tree_view'),
    # EditorGlobalState
    'app.editor_state':     ('editor_global', 'editor_state'),
    'app.scroll_positions': ('editor_global', 'scroll_positions'),
    'app.replace_buffer':   ('editor_global', 'replace_buffer'),
    'app.editor_clipboard': ('editor_global', 'editor_clipboard'),
    # UndoState
    'app.undo_stack': ('undo_state', 'undo_stack'),
    'app.redo_stack': ('undo_state', 'redo_stack'),
    # SettingsState
    'app.settings_index':       ('settings', 'settings_index'),
    'app.settings_section':     ('settings', 'settings_section'),
    'app.settings_target':     ('settings', 'settings_target'),
    'app.settings_scroll':     ('settings', 'settings_scroll'),
    'app.open_with_index':     ('settings', 'open_with_index'),
    'app.confirm_delete':      ('settings', 'confirm_delete'),
    'app.smart_date':          ('settings', 'smart_date'),
    'app.semantic_coloring':   ('settings', 'semantic_coloring'),
    'app.auto_save':           ('settings', 'auto_save'),
    'app.default_show_hidden':('settings', 'default_show_hidden'),
    # LayoutState
    'app.single_columns':      ('layout', 'single_columns'),
    'app.split_columns':       ('layout', 'split_columns'),
    'app.header_icon_bounds':  ('layout', 'header_icon_bounds'),
    'app.tab_bounds':          ('layout', 'tab_bounds'),
    'app.hovered_header_icon': ('layout', 'hovered_header_icon'),
    'app.expanded_folders':    ('layout', 'expanded_folders'),
    # OutputState
    'app.background_tasks':             ('output', 'background_tasks'),
    'app.last_action_msg':              ('output', 'last_action_msg'),
    'app.input_shield_until':           ('output', 'input_shield_until'),
    'app.input_shield_active_until':    ('output', 'input_shield_active_until'),
    # DragState
    'app.drag_start_pos':       ('drag', 'drag_start_pos'),
    'app.drag_source':          ('drag', 'drag_source'),
    'app.is_dragging':          ('drag', 'is_dragging'),
    'app.hovered_drop_target':  ('drag', 'hovered_drop_target'),
    # NavState
    'app.starred':             ('nav', 'starred'),
    'app.recent_folders':      ('nav', 'recent_folders'),
    'app.command_index':       ('nav', 'command_index'),
    'app.filtered_commands':   ('nav', 'filtered_commands'),
    'app.view_prefs':          ('nav', 'view_prefs'),
    # RemoteState
    'app.remote_bookmarks': ('remote', 'remote_bookmarks'),
    'app.pending_remote':   ('remote', 'pending_remote'),
    'app.external_tools':   ('remote', 'external_tools'),
    # MouseState
    'app.mouse_last_click':    ('mouse', 'mouse_last_click'),
    'app.mouse_click_pos':     ('mouse', 'mouse_click_pos'),
    'app.mouse_click_count':   ('mouse', 'mouse_click_count'),
    'app.is_resizing_sidebar': ('mouse', 'is_resizing_sidebar'),
    # SelectionState2
    'app.selection_mode':                     ('selection', 'selection_mode'),
    'app.prevent_mouse_up_selection_cleanup': ('selection', 'prevent_mouse_up_selection_cleanup'),
    'app.rename_selected':    ('selection', 'rename_selected'),
    'app.clipboard':          ('selection', 'clipboard'),
    'app.path_colors':        ('selection', 'path_colors'),
    'app.folder_selections': ('selection', 'folder_selections'),
}

# Fields that stay on App directly
APP_DIRECT = {
    'app.show_main_stage': ('app', 'show_main_stage'),
}


def migrate(content):
    # Normalize multi-line field accesses:
    # Patterns like "  app\n    .field" or "app\n        .field" → "app.field"
    # This handles both styles: dot-on-next-line and indented dot
    content = re.sub(
        r'app\s*\n\s*\.(\w+)',
        lambda m: f'app.{m.group(1)}',
        content
    )
    # Also handle app_guard. variant
    content = re.sub(
        r'app_guard\s*\n\s*\.(\w+)',
        lambda m: f'app_guard.{m.group(1)}',
        content
    )

    # Process sub-struct fields
    for old, (sub, field) in sorted(FIELDS.items(), key=lambda x: -len(x[0])):
        new = f'app.{sub}.{field}'
        if old in content:
            count = content.count(old)
            content = content.replace(old, new)
            print(f'  {old} -> {new} ({count}x)', file=sys.stderr)
        # Also handle app_guard. variant
        guard_old = old.replace('app.', 'app_guard.', 1)
        guard_new = f'app_guard.{sub}.{field}'
        if guard_old in content:
            count = content.count(guard_old)
            content = content.replace(guard_old, guard_new)
            print(f'  {guard_old} -> {guard_new} ({count}x)', file=sys.stderr)
    # Process App-direct fields
    for old, (_, field) in sorted(APP_DIRECT.items(), key=lambda x: -len(x[0])):
        new = f'app.{field}'
        if old in content:
            count = content.count(old)
            content = content.replace(old, new)
            print(f'  {old} -> {new} ({count}x)', file=sys.stderr)
        guard_old = old.replace('app.', 'app_guard.', 1)
        guard_new = f'app_guard.{field}'
        if guard_old in content:
            count = content.count(guard_old)
            content = content.replace(guard_old, guard_new)
            print(f'  {guard_old} -> {guard_new} ({count}x)', file=sys.stderr)
    return content


def process_file(path):
    text = path.read_text(encoding='utf-8')
    original = text
    text = migrate(text)
    if text != original:
        path.write_text(text, encoding='utf-8')
        return True
    return False


if __name__ == '__main__':
    src = Path('/home/dracon/Dev/tiles/src')
    for f in src.rglob('*.rs'):
        if process_file(f):
            print(f'Migrated: {f.relative_to(src)}')