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

use crate::csi;


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

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    accessors    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    pub(crate) fn locate(&self) -> (u16, u16) { (self.x, self.y) }

    pub(crate) fn stack_depth(&self) -> usize { self.stack.len() }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    movement    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯
    
    /// Moves cursor via CUP
    pub(crate) fn move_to(&mut self, x: u16, y: u16, max_x: u16, max_y: u16) -> Vec<u8> {
        self.x = x.min(max_x);
        self.y = y.min(max_y);

        if x == 0 && y == 0 { return csi!('H'); }
        csi!(y + 1, x + 1, 'H')
    }

    /// Moves cursor via CHA
    pub(crate) fn move_to_column(&mut self, x: u16, max: u16) -> Vec<u8> {
        self.x = x.min(max);
        csi!(x + 1, 'G')
    }

    /// Moves cursor via VPA
    pub(crate) fn move_to_row(&mut self, y: u16, max: u16) -> Vec<u8> {
        self.y = y.min(max);
        csi!(y + 1, 'd')
    }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    memory    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    /// Adds current cursor position to internal stack.
    pub(crate) fn save(&mut self) { self.stack.push((self.x, self.y)); }

    /// Pops most recent saved position off internal stack.
    pub(crate) fn restore(&mut self) -> Option<Vec<u8>> {
        if let Some((x, y)) = self.stack.pop() {
            self.x = x;
            self.y = y;

            if x == 0 && y == 0 { return Some(csi!('H')); }
            Some(csi!(y + 1, x + 1, 'H'))
        } else {
            None
        }
    }
}
