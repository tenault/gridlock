// ╭──────────────────────────────────────────────────────────────io/cursor.rs─╮
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

use super::escape::{Escapable, CursorContext};


// ╭──────────────────────╮
// │    VIRTUAL CURSOR    │
// ╰──────────────────────╯

pub(crate) struct VirtualCursor {
    x: u16,
    y: u16,
    stack: Vec<(u16, u16)>,
}

impl VirtualCursor {

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    constructor    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    pub(crate) fn new() -> Self {
        Self { x: 0, y: 0, stack: Vec::new() }
    }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    movement    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    /// Move cursor via CUP
    pub(crate) fn move_to(&mut self, x: u16, y: u16, max_x: u16, max_y: u16) -> Option<Vec<u8>> {
        let nx = x.min(max_x);
        let ny = y.min(max_y);

        let ctx = CursorContext {
            x: if self.x != nx { Some(nx) } else { None },
            y: if self.y != ny { Some(ny) } else { None },
        };

        self.x = nx;
        self.y = ny;

        ctx.get_escape()
    }

    /// Move cursor via CHA
    pub(crate) fn move_to_column(&mut self, x: u16, max: u16) -> Option<Vec<u8>> {
        let nx = x.min(max);
        
        let ctx = CursorContext {
            x: if self.x != nx { Some(nx) } else { None },
            y: None,
        };

        self.x = nx;

        ctx.get_escape()
    }

    /// Move cursor via VPA
    pub(crate) fn move_to_row(&mut self, y: u16, max: u16) -> Option<Vec<u8>> {
        let ny = y.min(max);

        let ctx = CursorContext {
            x: None,
            y: if self.y != ny { Some(ny) } else { None },
        };

        self.y = ny;

        ctx.get_escape()
    }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    memory    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    /// Adds current cursor position to internal stack.
    pub(crate) fn save(&mut self) { self.stack.push((self.x, self.y)); } // todo: save w/ coords

    /// Pops most recent saved position off internal stack.
    pub(crate) fn restore(&mut self) -> Option<Vec<u8>> {
        if let Some((nx, ny)) = self.stack.pop() {
            let ctx = CursorContext {
                x: if self.x != nx { Some(nx) } else { None },
                y: if self.y != ny { Some(ny) } else { None },
            };

            self.x = nx;
            self.y = ny;

            ctx.get_escape()
        } else {
            None
        }
    }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    accessors    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    pub(crate) fn locate(&self) -> (u16, u16) { (self.x, self.y) }

    pub(crate) fn stack_depth(&self) -> usize { self.stack.len() }
}
