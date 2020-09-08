use std::io::Read;

use rand::random;

use crate::terminal::Terminal;

const MEMORY: usize = 4_096;
type Instruction = (u8, u8, u8, u8);

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct CPU<R: Read> {
    terminal: Terminal<R>,
    memory: [u8; MEMORY],
    stack: [u16; 16],
    v: [u8; 16], // General purpose registers
    i: u16,
    dt: u8,  // Delay timer
    st: u8,  // Sound timer
    pc: u16, // Program counter aka instruction pointer
    sp: u8,  // Stack pointer
}

impl<R: Read> CPU<R> {
    pub fn new(r: R) -> Self {
        let mut memory = [0; MEMORY];
        memory[..FONT.len()].clone_from_slice(&FONT[..]);

        let mut terminal = Terminal::new(r);

        CPU {
            terminal,
            memory,
            stack: [0; 16],
            v: [0; 16],
            i: 0,
            dt: 0,
            st: 0,
            pc: 0x200,
            sp: 0,
        }
    }

    pub fn tick(&mut self) {
        let instruction = self.read_instruction();
        self.execute_instruction(instruction);
        if self.dt > 0 {
            self.dt -= 1
        }
        if self.st > 0 {
            self.st -= 1
        }
        self.terminal.render();
    }

    pub fn load(&mut self, data: &[u8]) {
        self.memory[0x200..].clone_from_slice(data);
    }

    fn read_instruction(&self) -> Instruction {
        let first_byte = self.memory[self.pc as usize];
        let second_byte = self.memory[self.pc as usize + 1];
        (
            first_byte >> 4,
            first_byte & 0xF,
            second_byte >> 4,
            second_byte & 0xF,
        )
    }

    fn execute_instruction(&mut self, instruction: Instruction) {
        // Increment program counter to point to the next instruction
        self.pc += 2;

        match instruction {
            // CLS
            (0, 0, 0xE, 0) => self.terminal.clear(),
            // RET
            (0, 0, 0xE, 0xE) => self.ret(),
            // JP addr
            (1, a, b, c) => self.pc = addr(a, b, c),
            // CALL addr
            (2, a, b, c) => self.call_addr(a, b, c),
            // SE Vx, byte
            (3, x, k1, k2) => self.se_vx_byte(x, k1, k2),
            // SNE Vx, byte
            (4, x, k1, k2) => self.sne_vx_byte(x, k1, k2),
            // SE Vx, Vy
            (5, x, y, 0) => self.se_vx_vy(x, y),
            // LD Vx, byte
            (6, x, k1, k2) => self.v[x as usize] = to_byte(k1, k2),
            // ADD Vx, byte
            (7, x, k1, k2) => {
                self.v[x as usize] = (self.v[x as usize] as u16 + to_byte(k1, k2) as u16) as u8
            }
            // LD Vx, Vy
            (8, x, y, 0) => self.v[x as usize] = self.v[y as usize],
            // OR Vx, Vy
            (8, x, y, 1) => self.v[x as usize] = self.v[x as usize] | self.v[y as usize],
            // AND Vx, Vy
            (8, x, y, 2) => self.v[x as usize] = self.v[x as usize] & self.v[y as usize],
            // XOR Vx, Vy
            (8, x, y, 3) => self.v[x as usize] = self.v[x as usize] ^ self.v[y as usize],
            // ADD Vx, Vy
            (8, x, y, 4) => self.add_vx_vy(x, y),
            // SUB Vx, Vy
            (8, x, y, 5) => self.sub_vx_vy(x, y),
            // SHR Vx {, Vy}
            (8, x, _, 6) => self.shr_vx(x),
            // SUBN Vx, Vy
            (8, x, y, 7) => self.subn_vx_vy(x, y),
            // SHL Vx {, Vy}
            (8, x, _, 0xE) => self.shl_vx(x),
            // SNE Vx, Vy
            (9, x, y, 0) => self.sne_vx_vy(x, y),
            // SLD I, addr
            (0xA, a, b, c) => self.i = addr(a, b, c),
            // JP V0, addr
            (0xB, a, b, c) => self.pc = self.v[0] as u16 + addr(a, b, c),
            // RND Vx, byte
            (0xC, x, k1, k2) => self.v[x as usize] = random::<u8>() & to_byte(k1, k2),
            // DRW Vx, Vy, nibble
            (0xD, x, y, n) => {
                self.v[0xF] = self.terminal.draw_sprite(
                    self.v[x as usize],
                    self.v[y as usize],
                    &self.memory[self.i as usize..(self.i as usize) + (n as usize)],
                )
            }
            // SKP Vx
            (0xE, x, 9, 0xE) => {
                if self.terminal.check_if_pressed(self.v[x as usize]) {
                    self.pc += 2
                }
            }
            // SKNP Vx
            (0xE, x, 0xA, 1) => {
                if !self.terminal.check_if_pressed(self.v[x as usize]) {
                    self.pc += 2
                }
            }
            // LD Vx, DT
            (0xF, x, 0, 7) => self.v[x as usize] = self.dt,
            // LD Vx, K
            (0xF, x, 0, 0xA) => match self.terminal.wait_for_key_press() {
                Some(key) => self.v[x as usize] = key,
                None => self.pc -= 2,
            },
            // LD DT, Vx
            (0xF, x, 1, 5) => self.dt = self.v[x as usize],
            // LD ST, Vx
            (0xF, x, 1, 8) => self.st = self.v[x as usize],
            // ADD I, Vx
            (0xF, x, 1, 0xE) => self.i = self.i + self.v[x as usize] as u16,
            // LD F, Vx
            (0xF, x, 2, 9) => self.i = (self.v[x as usize] & 0xF) as u16 * 5,
            // LD B, Vx
            (0xF, x, 3, 3) => self.ld_b_vx(x),
            // LD [I], Vx
            (0xF, x, 5, 5) => self.ld_i_vx(x),
            // LD Vx, [I]
            (0xF, x, 6, 5) => self.ld_vx_i(x),
            // SYS addr
            (0, _, _, _) => (), // Ignored by modern interpreters
            x => panic!("Unrecognized instruction: {:?}", x),
        }
    }

    fn sne_vx_vy(&mut self, x: u8, y: u8) {
        if self.v[x as usize] != self.v[y as usize] {
            self.pc += 2
        }
    }

    fn shl_vx(&mut self, x: u8) {
        let vx = self.v[x as usize];
        self.v[0xF] = if vx & 128 == 128 { 1 } else { 0 };
        self.v[x as usize] = vx << 1
    }

    fn subn_vx_vy(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        self.v[0xF] = if vy >= vx { 1 } else { 0 };
        self.v[x as usize] = vy.wrapping_sub(vx)
    }

    fn shr_vx(&mut self, x: u8) {
        let vx = self.v[x as usize];
        self.v[0xF] = if vx & 1 == 1 { 1 } else { 0 };
        self.v[x as usize] = vx >> 1
    }

    fn sub_vx_vy(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        self.v[0xF] = if vx >= vy { 1 } else { 0 };
        self.v[x as usize] = vx.wrapping_sub(vy)
    }

    fn add_vx_vy(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize] as u16;
        let vy = self.v[y as usize] as u16;
        self.v[0xF] = if vx + vy > 255 { 1 } else { 0 };
        self.v[x as usize] = (vx + vy % 256) as u8
    }

    fn se_vx_vy(&mut self, x: u8, y: u8) {
        if self.v[x as usize] == self.v[y as usize] {
            self.pc += 2
        }
    }

    fn sne_vx_byte(&mut self, x: u8, k1: u8, k2: u8) {
        if self.v[x as usize] != to_byte(k1, k2) {
            self.pc += 2
        }
    }

    fn se_vx_byte(&mut self, x: u8, k1: u8, k2: u8) {
        if self.v[x as usize] == to_byte(k1, k2) {
            self.pc += 2
        }
    }

    fn call_addr(&mut self, a: u8, b: u8, c: u8) {
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = addr(a, b, c)
    }

    fn ret(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    fn ld_b_vx(&mut self, x: u8) {
        let vx = self.v[x as usize];
        self.memory[self.i as usize] = vx / 100;
        self.memory[self.i as usize + 1] = vx % 100 / 10;
        self.memory[self.i as usize + 2] = vx % 10;
    }

    fn ld_i_vx(&mut self, x: u8) {
        for i in 0..=(x as usize) {
            self.memory[self.i as usize + i] = self.v[i]
        }
    }

    fn ld_vx_i(&mut self, x: u8) {
        for i in 0..=(x as usize) {
            self.v[i] = self.memory[self.i as usize + i]
        }
    }
}

fn to_byte(a: u8, b: u8) -> u8 {
    (a << 4) + b
}

fn addr(a: u8, b: u8, c: u8) -> u16 {
    ((a as u16) << 8) + ((b as u16) << 4) + (c as u16)
}

#[cfg(test)]
mod tests {
    #[test]
    fn ret() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.sp = 1;
        cpu.stack[0] = 0xDDD;
        cpu.execute_instruction((0, 0, 0xE, 0xE));
        assert_eq!(cpu.pc, 0xDDD);
    }

    #[test]
    fn jp() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.execute_instruction((2, 0xA, 0xE, 0xF));
        assert_eq!(cpu.pc, 0xAEF);
        assert_eq!(cpu.sp, 1);
        assert_eq!(cpu.stack[0], 0x202);
    }

    #[test]
    fn call() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.execute_instruction((1, 0xA, 0xE, 0xF));
        assert_eq!(cpu.pc, 0xAEF);
    }

    #[test]
    fn se_vx_byte() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        assert_eq!(cpu.pc, 0x200);
        cpu.v[1] = 0xEF;
        cpu.v[2] = 0xAA;
        cpu.execute_instruction((3, 1, 0xE, 0xF));
        assert_eq!(cpu.pc, 0x204);
        cpu.execute_instruction((3, 2, 0xD, 0xD));
        assert_eq!(cpu.pc, 0x206);
    }

    #[test]
    fn sne_vx_byte() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        assert_eq!(cpu.pc, 0x200);
        cpu.v[1] = 0xEF;
        cpu.v[2] = 0xAA;
        cpu.execute_instruction((4, 1, 0xE, 0xF));
        assert_eq!(cpu.pc, 0x202);
        cpu.execute_instruction((4, 2, 0xD, 0xD));
        assert_eq!(cpu.pc, 0x206);
    }

    #[test]
    fn se_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        assert_eq!(cpu.pc, 0x200);
        cpu.v[1] = 0xEF;
        cpu.v[2] = 0xAA;
        cpu.v[10] = 0xAA;
        cpu.execute_instruction((5, 2, 10, 0));
        assert_eq!(cpu.pc, 0x204);
        cpu.execute_instruction((5, 1, 2, 0));
        assert_eq!(cpu.pc, 0x206);
    }

    #[test]
    fn ld_vx_byte() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.execute_instruction((6, 2, 0xE, 0xA));
        assert_eq!(cpu.v[2], 0xEA);
    }

    #[test]
    fn add_vx_byte() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[2] = 0x22;
        cpu.execute_instruction((7, 2, 0x4, 0x5));
        assert_eq!(cpu.v[2], 0x67);
    }

    #[test]
    fn ld_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[3] = 0xEE;
        cpu.execute_instruction((8, 2, 3, 0));
        assert_eq!(cpu.v[2], 0xEE);
    }

    #[test]
    fn or_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[2] = 0b1100_1001;
        cpu.v[9] = 0b1000_0101;
        cpu.execute_instruction((8, 2, 9, 1));
        assert_eq!(cpu.v[2], 0b1100_1101);
    }

    #[test]
    fn and_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[2] = 0b1100_1001;
        cpu.v[9] = 0b1000_0101;
        cpu.execute_instruction((8, 2, 9, 2));
        assert_eq!(cpu.v[2], 0b1000_0001);
    }

    #[test]
    fn xor_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[2] = 0b1100_1001;
        cpu.v[9] = 0b1000_0101;
        cpu.execute_instruction((8, 2, 9, 3));
        assert_eq!(cpu.v[2], 0b0100_1100);
    }

    #[test]
    fn add_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[2] = 0xAA;
        cpu.v[9] = 0x12;
        cpu.execute_instruction((8, 2, 9, 4));
        assert_eq!(cpu.v[2], 0xBC);
        assert_eq!(cpu.v[0xf], 0);

        cpu.v[2] = 0xFF;
        cpu.v[9] = 0xFF;
        cpu.execute_instruction((8, 2, 9, 4));
        assert_eq!(cpu.v[2], 0xFE);
        assert_eq!(cpu.v[0xf], 1);
    }

    #[test]
    fn sub_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[2] = 0x0F;
        cpu.v[9] = 0xFF;
        cpu.execute_instruction((8, 2, 9, 5));
        assert_eq!(cpu.v[2], 0x10);
        assert_eq!(cpu.v[0xf], 0);

        cpu.v[2] = 0xFF;
        cpu.v[9] = 0x0F;
        cpu.execute_instruction((8, 2, 9, 5));
        assert_eq!(cpu.v[2], 0xF0);
        assert_eq!(cpu.v[0xf], 1);
    }

    #[test]
    fn shr_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[2] = 0b0001_0001;
        cpu.execute_instruction((8, 2, 9, 6));
        assert_eq!(cpu.v[2], 0b0000_1000);
        assert_eq!(cpu.v[0xf], 1);

        cpu.v[2] = 0b0001_0000;
        cpu.execute_instruction((8, 2, 9, 6));
        assert_eq!(cpu.v[2], 0b0000_1000);
        assert_eq!(cpu.v[0xf], 0);
    }

    #[test]
    fn subn_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[9] = 0x0F;
        cpu.v[2] = 0xFF;
        cpu.execute_instruction((8, 2, 9, 7));
        assert_eq!(cpu.v[2], 0x10);
        assert_eq!(cpu.v[0xf], 0);

        cpu.v[9] = 0xFF;
        cpu.v[2] = 0x0F;
        cpu.execute_instruction((8, 2, 9, 7));
        assert_eq!(cpu.v[2], 0xF0);
        assert_eq!(cpu.v[0xf], 1);
    }

    #[test]
    fn shl_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[2] = 0b0001_0001;
        cpu.execute_instruction((8, 2, 9, 0xE));
        assert_eq!(cpu.v[2], 0b0010_0010);
        assert_eq!(cpu.v[0xf], 0);

        cpu.v[2] = 0b1001_0001;
        cpu.execute_instruction((8, 2, 9, 0xE));
        assert_eq!(cpu.v[2], 0b0010_0010);
        assert_eq!(cpu.v[0xf], 1);
    }

    #[test]
    fn sne_vx_vy() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        assert_eq!(cpu.pc, 0x200);
        cpu.v[1] = 0xEF;
        cpu.v[2] = 0xAA;
        cpu.v[10] = 0xAA;
        cpu.execute_instruction((9, 2, 10, 0));
        assert_eq!(cpu.pc, 0x202);
        cpu.execute_instruction((9, 1, 2, 0));
        assert_eq!(cpu.pc, 0x206);
    }

    #[test]
    fn ld_i_addr() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.execute_instruction((0xA, 0xA, 0xB, 0xC));
        assert_eq!(cpu.i, 0xABC);
    }

    #[test]
    fn jp_v0_addr() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[0] = 0x11;
        cpu.execute_instruction((0xB, 0xA, 0xB, 0xC));
        assert_eq!(cpu.pc, 0xACD);
    }

    #[test]
    fn ld_vx_dt() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.dt = 0x11;
        cpu.execute_instruction((0xF, 4, 0, 7));
        assert_eq!(cpu.v[4], 0x11);
    }

    #[test]
    fn ld_dt_vx() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[4] = 0x11;
        cpu.execute_instruction((0xF, 4, 1, 5));
        assert_eq!(cpu.dt, 0x11);
    }

    #[test]
    fn ld_st_vx() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[4] = 0x11;
        cpu.execute_instruction((0xF, 4, 1, 8));
        assert_eq!(cpu.st, 0x11);
    }

    #[test]
    fn add_i_vx() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[4] = 0x11;
        cpu.i = 0xAA;
        cpu.execute_instruction((0xF, 4, 1, 0xE));
        assert_eq!(cpu.i, 0xBB);
    }

    #[test]
    fn ld_f_vx() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[4] = 0xA;
        cpu.execute_instruction((0xF, 4, 2, 9));
        assert_eq!(cpu.memory[cpu.i as usize], 0xF0);
        assert_eq!(cpu.memory[cpu.i as usize + 1], 0x90);
        assert_eq!(cpu.memory[cpu.i as usize + 2], 0xF0);
        assert_eq!(cpu.memory[cpu.i as usize + 3], 0x90);
        assert_eq!(cpu.memory[cpu.i as usize + 4], 0x90);

        cpu.v[4] = 0xBA;
        cpu.execute_instruction((0xF, 4, 2, 9));
        assert_eq!(cpu.memory[cpu.i as usize], 0xF0);
        assert_eq!(cpu.memory[cpu.i as usize + 1], 0x90);
        assert_eq!(cpu.memory[cpu.i as usize + 2], 0xF0);
        assert_eq!(cpu.memory[cpu.i as usize + 3], 0x90);
        assert_eq!(cpu.memory[cpu.i as usize + 4], 0x90);
    }

    #[test]
    fn ld_b_vx() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[4] = 0xFE;
        cpu.i = 0x100;
        cpu.execute_instruction((0xF, 4, 3, 3));
        assert_eq!(cpu.memory[0x100], 2);
        assert_eq!(cpu.memory[0x101], 5);
        assert_eq!(cpu.memory[0x102], 4);
    }

    #[test]
    fn ld_i_vx() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.v[0] = 0x12;
        cpu.v[1] = 0x34;
        cpu.v[2] = 0x56;
        cpu.v[3] = 0x78;
        cpu.i = 0x100;
        cpu.execute_instruction((0xF, 3, 5, 5));
        assert_eq!(cpu.memory[0x100], 0x12);
        assert_eq!(cpu.memory[0x101], 0x34);
        assert_eq!(cpu.memory[0x102], 0x56);
        assert_eq!(cpu.memory[0x103], 0x78);
    }

    #[test]
    fn ld_vx_i() {
        let r: &[u8] = b"";
        let mut cpu = super::CPU::new(r);
        cpu.memory[0x100] = 0x12;
        cpu.memory[0x101] = 0x34;
        cpu.memory[0x102] = 0x56;
        cpu.memory[0x103] = 0x78;
        cpu.i = 0x100;
        cpu.execute_instruction((0xF, 3, 6, 5));
        assert_eq!(cpu.v[0], 0x12);
        assert_eq!(cpu.v[1], 0x34);
        assert_eq!(cpu.v[2], 0x56);
        assert_eq!(cpu.v[3], 0x78);
    }

    #[test]
    fn addr() {
        assert_eq!(super::addr(0, 0, 0), 0);
        assert_eq!(super::addr(1, 1, 1), 0b1_0001_0001);
        assert_eq!(super::addr(0b1000, 0b1000, 0b1000), 0b1000_1000_1000);
    }

    #[test]
    fn to_byte() {
        assert_eq!(super::to_byte(0, 0), 0);
        assert_eq!(super::to_byte(0xA, 0xD), 0xAD);
    }
}
