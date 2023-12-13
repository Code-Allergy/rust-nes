#![feature(file_create_new)]

use std::fs::File;
use std::io::Read;
use std::{fs, io};
//Result<Vec<u8>, io::Error>

// fn get_bin_file(filename: &str) -> () {
//     let mut f = File::open(filename)?;
//     let metadata = fs::metadata(filename)?;
//     dbg!(metadata);
// }

pub mod cpu;
pub mod instructions;
pub mod memory;
pub mod ppu;

#[derive(Debug)]
pub struct NesRom {
    header: [u8; 16], // 16 byte header, 0-3 == "NES" followed by MS-DOS EOL
    trainer: Option<[u8; 512]>,
    pub prg_rom: Vec<[u8; 16384]>, // add x bytes extension based on header.
    pub chr_rom: Vec<[u8; 8192]>,  // add x bytes extension based on header.
    // inst_rom: Option<[u8; 8192]>,
    // prom: Option<[u8; 32]> // unsure
    // todo
    flags6: u8,
    flags7: u8,
    flags8: u8,
    flags9: u8,
    flags10: u8,
}

pub fn combine_bytes_to_u16(high: u8, low: u8) -> u16 {
    // Use bitwise OR to combine the bytes into a u16 value
    let result = ((high as u16) << 8) | low as u16;
    result
}

// HEADER FLAGS
// Byte 6
// 76543210
// ||||||||
// |||||||+- Mirroring: 0: horizontal (vertical arrangement) (CIRAM A10 = PPU A11)
// |||||||              1: vertical (horizontal arrangement) (CIRAM A10 = PPU A10)
// ||||||+-- 1: Cartridge contains battery-backed PRG RAM ($6000-7FFF) or other persistent memory
// |||||+--- 1: 512-byte trainer at $7000-$71FF (stored before PRG data)
// ||||+---- 1: Ignore mirroring control or above mirroring bit; instead provide four-screen VRAM
// ++++----- Lower nybble of mapper number
//
// Byte 7
// Byte 8
// Byte 9
// Byte 10

pub fn parse_bin_file(filename: &str) -> io::Result<NesRom> {
    // let nes_rom = NesRom::new();
    let mut f = File::open(filename).unwrap();
    let metadata = fs::metadata(filename).unwrap();
    let mut header = [0u8; 16];
    if (metadata.len() > 16) {
        f.read_exact(&mut header)?;
        if !header.starts_with(&[78, 69, 83, 26]) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid NES ROM file format",
            ));
        }
        println!("Length of PRG_ROM: {}", header[4]);
    }

    // no trainer handled yet, check if bit is set, if it is, read trainer.
    // let mut trainer = [0u8; 512];
    // f.read_exact(&mut trainer)?;
    // println!("{:?}", trainer);

    /* parse prg_rom pages */
    let prg_rom = (0..header[4])
        .map(|_| {
            let mut prg_rom_page = [0u8; 16384];
            f.read_exact(&mut prg_rom_page)
                .expect("Failed to parse file.");
            prg_rom_page
        })
        .collect();

    /* parse chr_rom pages */
    let chr_rom = (0..header[5])
        .map(|_| {
            let mut chr_rom_page = [0u8; 8192];
            f.read_exact(&mut chr_rom_page)
                .expect("Failed to parse file.");
            chr_rom_page
        })
        .collect();

    Ok(NesRom {
        header,
        prg_rom,
        chr_rom,

        trainer: None,

        flags6: header[6],
        flags7: header[7],
        flags8: header[8],
        flags9: header[9],
        flags10: header[10],
    })
}
