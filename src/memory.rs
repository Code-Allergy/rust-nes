use std::fs::File;
use std::io;
use std::io::Write;

// https://www.nesdev.org/wiki/CPU_memory_map
pub const ADDR_LO: u16 = 0x0000;
pub const ADDR_HI: u16 = 0xFFFF;
const STACK_ADDR_LO: u16 = 0x0100;
const STACK_ADDR_HI: u16 = 0x01FF;
const MEMORY_SIZE: usize = (ADDR_HI - ADDR_LO) as usize + 1usize;


pub trait Bus {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, byte: u8);
    fn write_bytes(&mut self, address: u16, bytes: &[u8]) {
        bytes.iter().enumerate().for_each(|(offset, &byte)| {
            self.write_byte((address + offset as u16), byte);
        });
    }
}


// first 256bytes: Zero Page (0000-00FF)
// second 256bytes: System Stack (0100-01FF)
// last 6 bytes (FFFA-FFFF):
//    addresses of the non-maskable interrupt handler ($FFFA/B)
//    the power on reset location ($FFFC/D)
//    BRK/interrupt request handler ($FFFE/F)

#[derive(Copy, Clone)]
pub struct Memory {
    bytes: [u8; MEMORY_SIZE],
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
impl Bus for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        // handle IO devices
        match address {
            _ => self.bytes[address as usize]
        }
    }

    // handle io devices
    fn write_byte(&mut self, address: u16, byte: u8) {
        self.bytes[address as usize] = byte;
    }
}

impl Memory {
    pub fn new() -> Memory { Memory { bytes: [0u8; MEMORY_SIZE] } }
    pub fn dump(&self) -> [u8; MEMORY_SIZE] { self.bytes }
    pub fn dump_to_file(&self, filename: &str) -> Result<(), io::Error> {
        File::create_new(filename)?.write_all(&self.bytes)
    }
}


