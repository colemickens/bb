/*
 * bb
 *
 * Copyright 2019 Manos Pitsidianakis
 *
 * This file is part of bb.
 *
 * bb is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * bb is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with bb. If not, see <http://www.gnu.org/licenses/>.
 */

/*!
Components are ways to handle application data. They can draw on the terminal and receive events, but also do other stuff as well. (For example, see the `notifications` module.)

See the `Component` Trait for more details.
*/

use super::*;
mod utilities;
pub use utilities::*;

mod kernel;
pub use kernel::*;
pub mod processes;
pub use processes::*;

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::fmt::{Debug, Display};

use super::{Key, UIEvent};
/// The upper and lower boundary char.
const HORZ_BOUNDARY: char = '─';
/// The left and right boundary char.
const VERT_BOUNDARY: char = '│';

pub type ShortcutMap = HashMap<&'static str, Key>;
pub type ShortcutMaps = HashMap<String, ShortcutMap>;

/// Types implementing this Trait can draw on the terminal and receive events.
/// If a type wants to skip drawing if it has not changed anything, it can hold some flag in its
/// fields (eg self.dirty = false) and act upon that in their `draw` implementation.
pub trait Component: Display + Debug + Send {
    fn draw(
        &mut self,
        grid: &mut CellBuffer,
        area: Area,
        dirty_areas: &mut VecDeque<Area>,
        tick: bool,
    );
    fn process_event(&mut self, event: &mut UIEvent, ui_mode: &mut UIMode);
    fn is_dirty(&self) -> bool {
        true
    }
    fn set_dirty(&mut self);
    fn get_shortcuts(&self) -> ShortcutMaps {
        Default::default()
    }
}

fn bin_to_ch(b: u32) -> char {
    match b {
        0b0001 => '╶',
        0b0010 => '╵',
        0b0011 => '└',
        0b0100 => '╴',
        0b0101 => '─',
        0b0110 => '┘',
        0b0111 => '┴',
        0b1000 => '╷',
        0b1001 => '┌',
        0b1010 => '│',
        0b1011 => '├',
        0b1100 => '┐',
        0b1101 => '┬',
        0b1110 => '┤',
        0b1111 => '┼',
        x => unreachable!(format!("unreachable bin_to_ch(x), x = {:b}", x)),
    }
}

fn ch_to_bin(ch: char) -> Option<u32> {
    match ch {
        '└' => Some(0b0011),
        '─' => Some(0b0101),
        '┘' => Some(0b0110),
        '┴' => Some(0b0111),
        '┌' => Some(0b1001),

        '│' => Some(0b1010),

        '├' => Some(0b1011),
        '┐' => Some(0b1100),
        '┬' => Some(0b1101),

        '┤' => Some(0b1110),

        '┼' => Some(0b1111),
        '╷' => Some(0b1000),

        '╵' => Some(0b0010),
        '╴' => Some(0b0100),
        '╶' => Some(0b0001),
        _ => None,
    }
}

#[allow(clippy::never_loop)]
fn set_and_join_vert(grid: &mut CellBuffer, idx: Pos) -> u32 {
    let (x, y) = idx;
    let mut bin_set = 0b1010;
    /* Check left side
     *
     *        1
     *   -> 2 │ 0
     *        3
     */
    loop {
        if x > 0 {
            if let Some(cell) = grid.get_mut(x - 1, y) {
                if let Some(adj) = ch_to_bin(cell.ch()) {
                    if (adj & 0b0001) > 0 {
                        bin_set |= 0b0100;
                        break;
                    } else if adj == 0b0100 {
                        cell.set_ch(bin_to_ch(0b0101));
                        cell.set_fg(Color::Byte(240));
                        bin_set |= 0b0100;
                        break;
                    }
                }
            }
        }
        bin_set &= 0b1011;
        break;
    }

    /* Check right side
     *
     *        1
     *      2 │ 0 <-
     *        3
     */
    loop {
        if let Some(cell) = grid.get_mut(x + 1, y) {
            if let Some(adj) = ch_to_bin(cell.ch()) {
                if (adj & 0b0100) > 0 {
                    bin_set |= 0b0001;
                    break;
                }
            }
        }
        bin_set &= 0b1110;
        break;
    }

    /* Set upper side
     *
     *        1 <-
     *      2 │ 0
     *        3
     */
    loop {
        if y > 0 {
            if let Some(cell) = grid.get_mut(x, y - 1) {
                if let Some(adj) = ch_to_bin(cell.ch()) {
                    cell.set_ch(bin_to_ch(adj | 0b1000));
                    cell.set_fg(Color::Byte(240));
                } else {
                    bin_set &= 0b1101;
                }
            }
        }
        break;
    }

    /* Set bottom side
     *
     *        1
     *      2 │ 0
     *        3 <-
     */
    loop {
        if let Some(cell) = grid.get_mut(x, y + 1) {
            if let Some(adj) = ch_to_bin(cell.ch()) {
                cell.set_ch(bin_to_ch(adj | 0b0010));
                cell.set_fg(Color::Byte(240));
            } else {
                bin_set &= 0b0111;
            }
        }
        break;
    }

    if bin_set == 0 {
        bin_set = 0b1010;
    }

    bin_set
}

#[allow(clippy::never_loop)]
fn set_and_join_horz(grid: &mut CellBuffer, idx: Pos) -> u32 {
    let (x, y) = idx;
    let mut bin_set = 0b0101;
    /* Check upper side
     *
     *        1 <-
     *      2 ─ 0
     *        3
     */
    loop {
        if y > 0 {
            if let Some(cell) = grid.get_mut(x, y - 1) {
                if let Some(adj) = ch_to_bin(cell.ch()) {
                    if (adj & 0b1000) > 0 {
                        bin_set |= 0b0010;
                        break;
                    } else if adj == 0b0010 {
                        bin_set |= 0b0010;
                        cell.set_ch(bin_to_ch(0b1010));
                        cell.set_fg(Color::Byte(240));
                        break;
                    }
                }
            }
        }
        bin_set &= 0b1101;
        break;
    }

    /* Check bottom side
     *
     *        1
     *      2 ─ 0
     *        3 <-
     */
    loop {
        if let Some(cell) = grid.get_mut(x, y + 1) {
            if let Some(adj) = ch_to_bin(cell.ch()) {
                if (adj & 0b0010) > 0 {
                    bin_set |= 0b1000;
                    break;
                } else if adj == 0b1000 {
                    bin_set |= 0b1000;
                    cell.set_ch(bin_to_ch(0b1010));
                    cell.set_fg(Color::Byte(240));
                    break;
                }
            }
        }
        bin_set &= 0b0111;
        break;
    }

    /* Set left side
     *
     *        1
     *   -> 2 ─ 0
     *        3
     */
    loop {
        if x > 0 {
            if let Some(cell) = grid.get_mut(x - 1, y) {
                if let Some(adj) = ch_to_bin(cell.ch()) {
                    cell.set_ch(bin_to_ch(adj | 0b0001));
                    cell.set_fg(Color::Byte(240));
                } else {
                    bin_set &= 0b1011;
                }
            }
        }
        break;
    }

    /* Set right side
     *
     *        1
     *      2 ─ 0 <-
     *        3
     */
    loop {
        if let Some(cell) = grid.get_mut(x + 1, y) {
            if let Some(adj) = ch_to_bin(cell.ch()) {
                cell.set_ch(bin_to_ch(adj | 0b0100));
                cell.set_fg(Color::Byte(240));
            } else {
                bin_set &= 0b1110;
            }
        }
        break;
    }

    if bin_set == 0 {
        bin_set = 0b0101;
    }

    bin_set
}

pub(crate) fn set_and_join_box(grid: &mut CellBuffer, idx: Pos, ch: char) {
    /* Connected sides:
     *
     *        1
     *      2 c 0
     *        3
     *
     *     #3210
     *    0b____
     */

    let bin_set = match ch {
        '│' => set_and_join_vert(grid, idx),
        '─' => set_and_join_horz(grid, idx),
        _ => unreachable!(),
    };

    grid[idx].set_ch(bin_to_ch(bin_set));
    grid[idx].set_fg(Color::Byte(240));
}

pub fn create_box(grid: &mut CellBuffer, area: Area) {
    if !is_valid_area!(area) {
        return;
    }
    let upper_left = upper_left!(area);
    let bottom_right = bottom_right!(area);

    /*for x in get_x(upper_left)..=get_x(bottom_right) {
        grid[(x, get_y(upper_left))].set_ch(HORZ_BOUNDARY);
        grid[(x, get_y(bottom_right))].set_ch(HORZ_BOUNDARY);
        grid[(x, get_y(bottom_right))].set_ch('▒');
        grid[(x, get_y(bottom_right))].set_fg(Color::Byte(240));
    }
    */

    for y in get_y(upper_left)..=get_y(bottom_right) {
        //grid[(get_x(upper_left), y)].set_ch(VERT_BOUNDARY);
        grid[(get_x(bottom_right), y)].set_ch('▒');
        grid[(get_x(bottom_right), y)].set_fg(Color::Byte(240));
    }
    //set_and_join_box(grid, upper_left, HORZ_BOUNDARY);
    //set_and_join_box(grid, set_x(upper_left, get_x(bottom_right)), HORZ_BOUNDARY);
    //set_and_join_box(grid, set_y(upper_left, get_y(bottom_right)), VERT_BOUNDARY);
    //set_and_join_box(grid, bottom_right, VERT_BOUNDARY);
}
