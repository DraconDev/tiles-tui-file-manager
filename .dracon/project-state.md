# Project State

## Current Focus
Added safety documentation for raw file descriptor handling in terminal input polling

## Context
The change addresses potential safety concerns around raw file descriptor usage in the terminal input polling mechanism. This is part of ongoing work to improve code safety and documentation in the terminal handling subsystem.

## Completed
- [x] Added safety documentation for raw file descriptor handling in terminal input polling
- [x] Documented the safety considerations for using BorrowedFd in the input polling loop

## In Progress
- [ ] No active work in progress beyond this change

## Blockers
- None identified for this specific change

## Next Steps
1. Review the safety documentation for completeness
2. Continue with terminal subsystem improvements and safety audits
