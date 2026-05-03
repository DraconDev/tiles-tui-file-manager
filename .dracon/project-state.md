# Project State

## Current Focus
Refactored system data retrieval to use blocking task spawning for better resource management

## Context
The change was prompted by a need to improve resource handling in the TTY polling loop. The original synchronous `get_data()` call could potentially block the async runtime, leading to degraded performance.

## Completed
- [x] Replaced synchronous `get_data()` call with async `spawn_blocking`
- [x] Added proper error handling for task spawning
- [x] Maintained same functionality while improving concurrency

## In Progress
- [ ] None

## Blockers
- None identified

## Next Steps
1. Verify no performance regressions in the TTY polling loop
2. Consider adding metrics to track blocking task execution times
