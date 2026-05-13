#!/usr/bin/env fish
# Tiles shell integration - cd on quit
# Add to your ~/.config/fish/config.fish:
#   source /path/to/tiles-shell.fish
#
# Usage: tcd (or alias tiles='tcd')
# After exiting tiles, your shell will cd to the last visited directory.

function tcd
    set -l tiles_cmd "tiles"
    if command -q "$TILES_CMD"
        set tiles_cmd "$TILES_CMD"
    end

    "$tiles_cmd" $argv

    set -l last_dir_file "$XDG_CONFIG_HOME/tiles/last_dir"
    if test -z "$XDG_CONFIG_HOME"
        set last_dir_file "$HOME/.config/tiles/last_dir"
    end

    if test -f "$last_dir_file"
        set -l last_dir (cat "$last_dir_file" 2>/dev/null)
        if test -d "$last_dir"
            cd "$last_dir"
        end
    end
end

# Optional alias - uncomment if you want `tiles` to always cd on quit
# alias tiles='tcd'
