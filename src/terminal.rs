use std::io::{stdout, Read, Stdout, Write};
use std::process::exit;

use termion::cursor;
use termion::event::Key;
use termion::input::{Keys, TermRead};
use termion::raw::{IntoRawMode, RawTerminal};

pub struct Terminal<R: TermRead> {
    stdout: RawTerminal<Stdout>,
    stdin: Keys<R>,
    pixels: [u64; 32],
    unprocessed: Vec<u8>,
    pub exit: bool,
}

struct BitIterator {
    n: u64,
    index: u32,
}
impl BitIterator {
    pub fn new(n: u64) -> Self {
        Self { n, index: 64 }
    }
}
impl Iterator for BitIterator {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            return None;
        }
        let res = self.n & (1 << (self.index - 1));
        self.index -= 1;
        Some(res > 0)
    }
}

impl<R: Read> Terminal<R> {
    pub fn new(r: R) -> Self {
        let mut term = Terminal {
            stdout: stdout().into_raw_mode().unwrap(),
            stdin: r.keys(),
            pixels: [0; 32],
            unprocessed: Vec::new(),
            exit: false,
        };
        term.clear();
        write!(term.stdout, "{}", cursor::Hide).unwrap();
        term
    }

    pub fn render(&mut self) {
        for (y, &line) in self.pixels.iter().enumerate() {
            for (x, bit) in BitIterator::new(line).enumerate() {
                write!(
                    self.stdout,
                    "{}{}",
                    cursor::Goto(x as u16 + 1, y as u16 + 1),
                    if bit { '█' } else { ' ' }
                )
                .unwrap();
            }
        }
        self.stdout.flush().unwrap();
    }

    pub fn clear(&mut self) {
        write!(self.stdout, "{}", termion::clear::All).unwrap();
        self.pixels = [0; 32];
        self.stdout.flush().unwrap();
    }

    pub fn draw_sprite(&mut self, x: u8, y: u8, sprite: &[u8]) -> u8 {
        let mut row = y as usize;
        let mut overwritten = false;

        for &byte in sprite {
            if row >= 32 {
                row %= 32;
            }
            let new_line = self.pixels[row] ^ (u64::from_be(byte as u64).rotate_right(x as u32));
            overwritten = overwritten || self.pixels[row] & new_line != self.pixels[row];
            self.pixels[row] = new_line;
            row += 1;
        }
        if overwritten {
            1
        } else {
            0
        }
    }

    pub fn check_if_pressed(&mut self, expected: u8) -> bool {
        for (i, &key) in self.unprocessed.iter().enumerate() {
            if key == expected {
                let _: Vec<_> = self.unprocessed.drain(0..=i).collect();
                return true;
            }
        }

        while let Some(Ok(k)) = self.stdin.next() {
            if k == Key::Ctrl('c') {
                self.exit = true;
            }
            match Self::map_key(k) {
                Some(key) if key == expected => {
                    self.unprocessed.clear();
                    return true;
                }
                Some(key) => self.unprocessed.push(key),
                _ => (),
            }
        }

        false
    }

    pub fn wait_for_key_press(&mut self) -> Option<u8> {
        if let Some(Ok(k)) = self.stdin.next() {
            if k == Key::Ctrl('c') {
                self.exit = true;
            }
            match Self::map_key(k) {
                Some(key) => Some(key),
                _ => None,
            }
        } else {
            None
        }
    }

    fn map_key(key: Key) -> Option<u8> {
        match key {
            Key::Char('0') => Some(0),
            Key::Char('1') => Some(1),
            Key::Char('2') => Some(2),
            Key::Char('3') => Some(3),
            Key::Char('4') => Some(4),
            Key::Char('5') => Some(5),
            Key::Char('6') => Some(6),
            Key::Char('7') => Some(7),
            Key::Char('8') => Some(8),
            Key::Char('9') => Some(9),
            Key::Char('a') => Some(10),
            Key::Char('b') => Some(11),
            Key::Char('c') => Some(12),
            Key::Char('d') => Some(13),
            Key::Char('e') => Some(14),
            Key::Char('f') => Some(15),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::terminal::BitIterator;

    #[test]
    fn draw_sprite() {
        let r: &[u8] = b"\x1Bayo\x7F\x1B[D";
        let mut term = super::Terminal::new(r);
        let mut overwritten = term.draw_sprite(1, 1, &[0b1100_1100]);
        assert_eq!(overwritten, 0);
        assert_eq!(
            term.pixels[1],
            0b0110_0110_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000
        );

        overwritten = term.draw_sprite(1, 1, &[0b0011_0000, 0b0011_0011]);
        assert_eq!(overwritten, 0);
        assert_eq!(
            term.pixels[1],
            0b0111_1110_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000
        );
        assert_eq!(
            term.pixels[2],
            0b0001_1001_1000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000
        );

        overwritten = term.draw_sprite(1, 2, &[0b1100_0011]);
        assert_eq!(overwritten, 1);
        assert_eq!(
            term.pixels[1],
            0b0111_1110_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000
        );
        assert_eq!(
            term.pixels[2],
            0b0111_1000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000
        );

        overwritten = term.draw_sprite(60, 31, &[0b1100_0011, 0b0011_1100]);
        assert_eq!(overwritten, 0);
        assert_eq!(
            term.pixels[0],
            0b1100_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0011
        );
        assert_eq!(
            term.pixels[1],
            0b0111_1110_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000
        );
        assert_eq!(
            term.pixels[2],
            0b0111_1000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000
        );
        assert_eq!(
            term.pixels[31],
            0b0011_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_1100
        );
    }

    #[test]
    fn bit_iterator() {
        let val = 0b1111_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_1100;
        let res = BitIterator::new(val).collect::<Vec<bool>>();
        assert_eq!(res[0..7], [true, true, true, true, false, false, false]);
        assert_eq!(res[57..], [false, false, false, true, true, false, false]);
        assert_eq!(res.len(), 64);
    }
}
