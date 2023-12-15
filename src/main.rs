extern crate sdl2;

use nesemu::cpu::{NesCpu, CLOCK_RATE};
use nesemu::parse_bin_file;
use nesemu::sdl::sdl_display;
use std::env;
use std::time::Duration;

const SIM_CLOCK_RATE: u32 = 1000;

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let default = "test-bin/nestest.nes".to_string();
    let rom_file = args.get(1).unwrap_or(&default);
    let rom = parse_bin_file(rom_file).expect("Rom not found.");

    let mut processor = NesCpu::new();
    processor.load_rom(&rom);
    std::thread::spawn(sdl_display);

    loop {
        processor.fetch_decode_next();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / SIM_CLOCK_RATE));
    }
}
