#!/usr/bin/env zsh
# Tiles shell integration - cd on quit
# Add to your ~/.zshrc:
#   source /path/to/tiles-shell.zsh
#
# Usage: tcd (or alias tiles='tcd')
# After exiting tiles, your shell will cd to the last visited directory.

tcd() {
    local tiles_cmd="tiles"
    if (( $+commands[${TILES_CMD}] )); then
        tiles_cmd="${TILES_CMD}"
    fi

    "$tiles_cmd" "$@"

    local last_dir_file="${XDG_CONFIG_HOME:-$HOME/.config}/tiles/last_dir"
    if [[ -f "$last_dir_file" ]]; then
        local last_dir
        last_dir="$(cat "$last_dir_file" 2>/dev/null)"
        if [[ -d "$last_dir" ]]; then
            cd "$last_dir" || return
        fi
    fi
}

# Optional alias - uncomment if you want `tiles` to always cd on quit
# alias tiles='tcd'
