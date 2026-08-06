// ╭───────────────────────────────────────────────────────────────────────io/mod.rs─╮
// │                                                                                 │
// │    ┏━━━━━━━┓ ┏━━━━━━━┓ ┏━┓ ┏━━━━━━━┓ ┏━┓       ┏━━━━━━━┓ ┏━━━━━━━┓ ┏━┓ ┏━━━┓    │
// │    ┃ ┏━━━━━┛ ┃ ┏━━━┓ ┃ ┃ ┃ ┗━┓ ┏━┓ ┃ ┃ ┃       ┃ ┏━━━┓ ┃ ┃ ┏━━━━━┛ ┃ ┃ ┃ ┏━┛    │
// │    ┃ ┃ ┏━━━┓ ┃ ┗━━━┛ ┃ ┃ ┃   ┃ ┃ ┃ ┃ ┃ ┃       ┃ ┃   ┃ ┃ ┃ ┃       ┃ ┗━┛ ┗━┓    │
// │    ┃ ┃ ┗━┓ ┃ ┃ ┏━┓ ┏━┛ ┃ ┃   ┃ ┃ ┃ ┃ ┃ ┃       ┃ ┃   ┃ ┃ ┃ ┃       ┃ ┏━━━┓ ┃    │
// │    ┃ ┗━━━┛ ┃ ┃ ┃ ┃ ┗━┓ ┃ ┃ ┏━┛ ┗━┛ ┃ ┃ ┗━━━━━┓ ┃ ┗━━━┛ ┃ ┃ ┗━━━━━┓ ┃ ┃   ┃ ┃    │
// │    ┗━━━━━━━┛ ┗━┛ ┗━━━┛ ┗━┛ ┗━━━━━━━┛ ┗━━━━━━━┛ ┗━━━━━━━┛ ┗━━━━━━━┛ ┗━┛   ┗━┛    │
// │                                                                                 │
// │                   copyright (c) 2026 Malakai Smith (@tenault)                   │
// │                                                                                 │
// │       This Source Code Form is subject to the terms of the Mozilla Public       │
// │       License, v. 2.0. If a copy of the MPL was not distributed with this       │
// │            file, You can obtain one at https://mozilla.org/MPL/2.0.             │
// │                                                                                 │
// ╰─────────────────────────────────────────────────────────────────────────────────╯

mod guard;
mod signal;
mod termios;
mod tty;

pub use tty::{TerminalError, TerminalSnapshot};
pub use guard::TerminalGuard;
