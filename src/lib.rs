// ╭────────────────────────────────────────────────────────────────────lib.rs─╮
// │                                                                           │
// │                                ┏━┓    ┏━━┓              ┏━┓               │
// │                                ┃ ┃    ┗┓ ┃              ┃ ┃               │
// │           ┏━━━┓┏┓┏━━━━━┓┏━┓┏━━━┛ ┃     ┃ ┃┏━━━━━┓┏━━━━━┓┃ ┃┏━━┓           │
// │           ┃ ┏━┓ ┃┃ ┏━━━┛┃ ┃┃ ┏━┓ ┃     ┃ ┃┃ ┏━┓ ┃┃ ┏━━━┛┃ ┗┛┏━┛           │
// │           ┃ ┗━┛ ┃┃ ┃    ┃ ┃┃ ┗━┛ ┃ ┏━┓ ┃ ┃┃ ┗━┛ ┃┃ ┗━━━┓┃ ┏┓┗━┓           │
// │           ┗━━━┓ ┃┗━┛    ┗━┛┗━━━┛┗┛ ┗━┛ ┗━━┛┗━━━━┛┗━━━━━┛┗━┛┗━━┛           │
// │           ┏━━━┛ ┃ ////////////////////////////////////////////            │
// │           ┗━━━━━┛                                                         │
// │                                                                           │
// │                copyright (c) 2026 Malakai Smith (@tenault)                │
// │                                                                           │
// │    This Source Code Form is subject to the terms of the Mozilla Public    │
// │    License, v. 2.0. If a copy of the MPL was not distributed with this    │
// │         file, You can obtain one at https://mozilla.org/MPL/2.0.          │
// │                                                                           │
// ╰───────────────────────────────────────────────────────────────────────────╯

// ╭───────────────────╮
// │    ENVIRONMENT    │
// ╰───────────────────╯

pub mod io;

pub use io::{Color, Terminal, TerminalError, TerminalStyle};
