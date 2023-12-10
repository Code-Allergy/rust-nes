extern crate sdl2;

use std::{fs, io};
use std::fs::File;
use std::io::Read;
use std::thread::sleep;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use nesemu::{parse_bin_file};
use nesemu::cpu::{CLOCK_RATE, NesCpu};


pub fn main() {
    let rom = parse_bin_file("test-bin/branch_timing_tests/Branch_Basics.nes").expect("TODO: panic message");
    // let rom = parse_bin_file("test-bin/genuine/SMB1.nes").expect("Fart brains");
    let mut cpu = NesCpu::new();
    cpu.load_rom(&rom);

    loop {
        cpu.fetch_decode_next();
        sleep(Duration::from_secs_f64(1.0 / CLOCK_RATE as f64));
    }
    // cpu.memory.dump_to_file("some.bin").expect("Failed to write some");
    // dbg!(rom);
    // sdl_display();
}

pub fn sdl_display() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo", 256, 240)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    'running: loop {
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

