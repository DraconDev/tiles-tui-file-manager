#!/usr/bin/env python3
"""Migrate field references from flat FileState to sub-structs."""

import re, sys
from pathlib import Path

# Mapping: old_field -> (sub_struct, field_in_substruct)
FIELDS = {
    # FileNavState
    'fs.current_path':   ('nav', 'current_path'),
    'fs.remote_session': ('nav', 'remote_session'),
    'fs.show_hidden':    ('nav', 'show_hidden'),
    'fs.search_filter': ('nav', 'search_filter'),
    'fs.search_generation': ('nav', 'search_generation'),
    'fs.history':        ('nav', 'history'),
    'fs.history_index':  ('nav', 'history_index'),
    'fs.sort_column':   ('nav', 'sort_column'),
    'fs.sort_ascending':('nav', 'sort_ascending'),
    'fs.search_debounce_until': ('nav', 'search_debounce_until'),
    # FileListState
    'fs.files':          ('list', 'files'),
    'fs.selection':      ('list', 'selection'),
    'fs.columns':         ('list', 'columns'),
    'fs.local_count':     ('list', 'local_count'),
    'fs.tree_file_depths':('list', 'tree_file_depths'),
    'fs.metadata':        ('list', 'metadata'),
    'fs.path_colors':     ('list', 'path_colors'),
    # FileViewState
    'fs.preview':           ('view', 'preview'),
    'fs.view_height':        ('view', 'view_height'),
    'fs.table_state':        ('view', 'table_state'),
    'fs.column_bounds':       ('view', 'column_bounds'),
    'fs.breadcrumb_bounds':  ('view', 'breadcrumb_bounds'),
    'fs.breadcrumb_header_bounds': ('view', 'breadcrumb_header_bounds'),
    'fs.pending_select_path':('view', 'pending_select_path'),
    'fs.file_row_bounds':    ('view', 'file_row_bounds'),
    # FileGitState
    'fs.git_history':        ('git', 'git_history'),
    'fs.git_history_state':  ('git', 'git_history_state'),
    'fs.git_pending_state':  ('git', 'git_pending_state'),
    'fs.git_branch':        ('git', 'git_branch'),
    'fs.git_ahead':         ('git', 'git_ahead'),
    'fs.git_behind':         ('git', 'git_behind'),
    'fs.git_pending':        ('git', 'git_pending'),
    'fs.git_summary':        ('git', 'git_summary'),
    'fs.git_remotes':        ('git', 'git_remotes'),
    'fs.git_stashes':        ('git', 'git_stashes'),
    'fs.git_cache_until':    ('git', 'git_cache_until'),
}


def migrate(content):
    # Normalize multi-line field accesses
    content = re.sub(r'fs\s*\n\s*\.(\w+)', lambda m: f'fs.{m.group(1)}', content)
    # Process sub-struct fields
    for old, (sub, field) in sorted(FIELDS.items(), key=lambda x: -len(x[0])):
        new = f'fs.{sub}.{field}'
        if old in content:
            count = content.count(old)
            content = content.replace(old, new)
            print(f'  {old} -> {new} ({count}x)', file=sys.stderr)
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