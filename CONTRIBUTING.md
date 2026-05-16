# Contributing to Tiles

Thank you for your interest in contributing!

## License

All contributions are subject to the terms of the [AGPLv3 license](./LICENSE) and the [Contributor License Agreement (CLA)](./CLA.md).

**By submitting a Contribution (including via pull request, issue, comment, or any other method), you agree to be bound by both the AGPLv3 license and the CLA.**

## Before You Submit a Pull Request

1. **Read the CLA** — Make sure you understand and agree to the [Contributor License Agreement](./CLA.md) before submitting any Contribution.
2. **Fork and branch** — Create a feature branch from `main` for your changes.
3. **Write clean, idiomatic code** — Follow the existing style and conventions of the project.
4. **Test your changes** — Ensure all existing and new tests pass before opening a PR.
5. **Describe your changes** — Include a clear PR description explaining *what* changed and *why*.
6. **Keep scope small** — One PR per logical change. Don't bundle unrelated fixes.

## Prerequisites

- Rust stable (1.75+)
- Git
- A terminal with TrueColor support

## Setup

```bash
git clone https://github.com/DraconDev/tiles
cd tiles
cargo build
```

## Project Structure

```
src/
├── main.rs              # Entry point, event loop, tokio runtime, file watchers
├── app.rs               # App state, debug logging, widget definitions
├── event.rs             # Event type conversion helpers
├── event_helpers.rs     # Navigation, clipboard, path resolution, history
├── config.rs            # Settings persistence (TOML)
├── icons.rs             # File type icon mapping
├── state/
│   └── mod.rs           # Data structures: FileState, AppMode, CurrentView, etc.
├── modules/
│   ├── files.rs         # Local filesystem: read_dir, metadata, search, git data
│   ├── system.rs        # System info: CPU, memory, disk usage
│   ├── docker.rs        # Docker container management
│   └── introspection.rs # Process and system introspection
├── servers.rs           # Remote server management (SSH/SFTP)
└── ui/
    ├── mod.rs           # Rendering helpers and layout utilities
    ├── panes/           # Sidebar, editor, preview panes
    └── theme.rs         # Color scheme definitions
```

## Code of Conduct

All contributors are expected to behave professionally and respectfully. We do not tolerate harassment, discrimination, or hostile behavior in any form.

## Getting Help

If you have questions or need guidance, open an issue or reach out to the maintainers directly.

---

*For details on commercial licensing, see [COMMERCIAL-LICENSE.md](./COMMERCIAL-LICENSE.md).*