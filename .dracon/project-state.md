# Project State

## Current Focus
Added D-Bus tool selection documentation for reliable terminal spawning

## Context
The change documents the decision to use `busctl` instead of `qdbus` for terminal spawning due to reliability issues with `qdbus` on Konsole 26.04.0+

## Completed
- [x] Added documentation explaining the D-Bus tool selection rationale
- [x] Documented the specific crash behavior with `qdbus`
- [x] Added warning against reverting to `qdbus` without testing

## In Progress
- [ ] No active work in progress

## Blockers
- None identified

## Next Steps
1. Verify the documentation is sufficient for future maintenance
2. Consider adding runtime checks for Konsole version compatibility
