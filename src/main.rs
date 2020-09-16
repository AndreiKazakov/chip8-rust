use std::fs::File;
use std::io::Read;
use std::time::{Duration, SystemTime};
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
    let mut time = SystemTime::now();
    let mut update_timers = false;

    while cpu.tick(update_timers) {
        update_timers = false;
        thread::sleep(Duration::from_micros(200));
        let new_time = SystemTime::now();
        if new_time.duration_since(time).unwrap().as_micros() > 16667 {
            time = new_time;
            update_timers = true;
        }
    }
}
