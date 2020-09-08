use std::fs::File;
use std::io::Read;
use std::time::Duration;
use std::{env, thread};

use termion::async_stdin;

mod cpu;
mod terminal;

fn main() {
    let mut cpu = cpu::CPU::new(async_stdin());

    let args: Vec<String> = env::args().collect();
    let file = &args[1];
    let mut buf = [0; 3584];
    let mut rom = File::open(file).unwrap();
    let _ = rom.read(&mut buf).unwrap();
    cpu.load(&buf);

    loop {
        cpu.tick();

        thread::sleep(Duration::from_millis(7));
    }
}
