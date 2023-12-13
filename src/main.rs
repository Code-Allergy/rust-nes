extern crate sdl2;

use lazy_static::lazy_static;
use nesemu::cpu::{NesCpu, Processor, CLOCK_RATE};
use nesemu::instructions::{AddressingMode, Instructions};
use nesemu::parse_bin_file;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::fs::File;
use std::io::Read;
use std::sync::{Mutex, MutexGuard};
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io, panic};

const SIM_CLOCK_RATE: i32 = 1000;

lazy_static! {
    static ref PROCESSOR: Mutex<NesCpu> = Mutex::new(NesCpu::new());
}

pub fn main() {
    // let rom = parse_bin_file("test-bin/branch_timing_tests/Branch_Basics.nes")
    //     .expect("TODO: panic message");
    // let rom = parse_bin_file("test-bin/genuine/SMB1.nes").expect("Fart brains");
    let rom = parse_bin_file("test-bin/nestest.nes").expect("Fart brains");

    let mut processor: MutexGuard<NesCpu> = PROCESSOR.lock().unwrap();
    processor.load_rom(&rom);
    // let mut file = File::open("test-bin/non-nes/6502_functional_test.bin").unwrap();
    // let mut program = [0u8; 32768];
    // file.read_exact(&mut program).unwrap();
    // processor.load_bytes(&program);
    // cpu.memory.dump_to_file("Memout.bin").expect("FUCK");
    loop {
        processor.fetch_decode_next();
        sleep(Duration::from_secs_f64(1.0 / SIM_CLOCK_RATE as f64));
    }

    // cpu.memory.dump_to_file("some.bin").expect("Failed to write some");
    // dbg!(rom);
    // sdl_display();
}

pub fn sdl_display() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rust-sdl2 demo", 256, 240)
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
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

fn test_branch() -> [u8; 32] {
    let mut operations = [0u8; 32];
    operations[0] =
        NesCpu::encode_instructions(Instructions::LoadAccumulator, AddressingMode::Immediate);
    operations[1] = 128;

    operations[2] = NesCpu::encode_instructions(
        Instructions::PushAccumulatorOnStack,
        AddressingMode::Implied,
    );
    operations[3] =
        NesCpu::encode_instructions(Instructions::LoadAccumulator, AddressingMode::Immediate);
    operations[4] = 4;
    operations[5] = NesCpu::encode_instructions(
        Instructions::PushAccumulatorOnStack,
        AddressingMode::Implied,
    );
    operations[6] =
        NesCpu::encode_instructions(Instructions::LoadAccumulator, AddressingMode::Immediate);
    operations[7] = 3;
    operations[8] = NesCpu::encode_instructions(
        Instructions::PushAccumulatorOnStack,
        AddressingMode::Implied,
    );
    operations[9] =
        NesCpu::encode_instructions(Instructions::LoadAccumulator, AddressingMode::Immediate);
    operations[10] = 2;
    operations[11] = NesCpu::encode_instructions(
        Instructions::PushAccumulatorOnStack,
        AddressingMode::Implied,
    );
    operations[12] =
        NesCpu::encode_instructions(Instructions::LoadAccumulator, AddressingMode::Immediate);
    operations[13] = 1;
    operations[14] = NesCpu::encode_instructions(
        Instructions::PushAccumulatorOnStack,
        AddressingMode::Implied,
    );
    operations[15] = NesCpu::encode_instructions(
        Instructions::PullAccumulatorFromStack,
        AddressingMode::Implied,
    );
    operations[16] = NesCpu::encode_instructions(
        Instructions::PullAccumulatorFromStack,
        AddressingMode::Implied,
    );
    operations[17] = NesCpu::encode_instructions(
        Instructions::PullAccumulatorFromStack,
        AddressingMode::Implied,
    );
    operations[18] = NesCpu::encode_instructions(
        Instructions::PullAccumulatorFromStack,
        AddressingMode::Implied,
    );
    operations[19] = NesCpu::encode_instructions(
        Instructions::PullAccumulatorFromStack,
        AddressingMode::Implied,
    );
    return operations;
}
