use std::io;
use crate::memory::{ADDR_HI, Bus, Memory};
use crate::NesRom;

pub const CLOCK_RATE: u32 = 21441960;

#[derive(Debug, Eq, PartialEq)]
pub enum AddressingMode {
    Accumulator,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Immediate,
    Implied,
    Indirect,
    XIndirect,
    YIndirect,
    Relative,
    ZeroPage,
    ZeroPageX,
    ZeroPageY
}
impl AddressingMode {
    fn get_increment(&self) -> u16 {
        match self {
            AddressingMode::Implied |
            AddressingMode::Accumulator => 1,

            AddressingMode::Immediate |
            AddressingMode::XIndirect |
            AddressingMode::YIndirect |
            AddressingMode::ZeroPage  |
            AddressingMode::ZeroPageX |
            AddressingMode::ZeroPageY |
            AddressingMode::Relative   => 2,

            AddressingMode::Absolute  |
            AddressingMode::AbsoluteX |
            AddressingMode::AbsoluteY |
            AddressingMode::Indirect   => 3
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Instructions {
    SetInterruptDisable,
    ClearInterruptDisable,
    SetDecimalMode,
    ClearDecimalMode,
    ClearOverflow,
    SetCarry,
    ClearCarry,
    LoadAccumulator,
    StoreAccumulator,
    LoadX,
    LoadY,
    StoreX,
    StoreY,
    MoveXToStackPointer,
    MoveStackPointerToX,
    EORAccumulator,
    ORAccumulator,
    ANDAccumulator,
    CompareAccumulator,
    CompareX,
    CompareY,
    BranchOnCarrySet,
    BranchOnCarryClear,
    BranchOnResultZero,
    BranchOnResultMinus,
    BranchOnResultNotZero,
    BranchOnResultPlus,
    BranchOnOverflowClear,
    BranchOnOverflowSet,
    DecrementX,
    DecrementY,
    DecrementMem,
    IncrementX,
    IncrementY,
    IncrementMem,
    JumpStoreReturnAddress,
    Jump,
    PullAccumulatorFromStack,
    PullProcessorStatusFromStack,
    PushAccumulatorOnStack,
    PushProcessorStatusOnStack,
    ShiftOneRight,
    ShiftOneLeft,
    RotateOneLeft,
    RotateOneRight,
    ReturnFromInterrupt,
    ReturnFromSubroutine,
    TransferAccumulatorToY,
    TransferAccumulatorToX,
    TransferXToAccumulator,
    TransferYToAccumulator,
    TransferStackPointerToX,
    AddMemToAccumulatorWithCarry,
    TestBitsAccumulator,

    SubtractAccumulatorWithBorrow,

    MissingOperation,
    NoOperation,
    JAM,
    ForceBreak,

    // ??
    ISC,

    // illegal?
    SLO,
    SAX,
    DCP,
    ARR,
    TAS,
    ANE,
    LAX,
    RLA,
    ANC,
    SRE,
    RRA,
    ALR,
    USBC,
    LAS,
    LXA,
    SHA,
    SBX,
    SHY,
    SHX

}

pub struct NesCpu {
    pub memory: Memory,
    pub reg: Registers,
}

impl NesCpu {
    pub fn new() -> Self {
        NesCpu {
            memory: Memory::default(),
            reg: Registers::new()
        }
    }
    pub fn decode(opcode: u8) -> (Instructions, AddressingMode) {
        match opcode {
            0x78 => (Instructions::SetInterruptDisable, AddressingMode::Implied),
            0xD8 => (Instructions::ClearDecimalMode, AddressingMode::Implied),
            0xA9 => (Instructions::LoadAccumulator, AddressingMode::Immediate),
            0x8D => (Instructions::StoreAccumulator, AddressingMode::Absolute),
            0xA2 => (Instructions::LoadX, AddressingMode::Immediate),
            0x9A => (Instructions::MoveXToStackPointer, AddressingMode::Implied),
            0xAD => (Instructions::LoadAccumulator, AddressingMode::Absolute),
            0x10 => (Instructions::ORAccumulator, AddressingMode::XIndirect),
            0xA0 => (Instructions::LoadY, AddressingMode::Immediate),
            0xBD => (Instructions::LoadAccumulator, AddressingMode::AbsoluteX),
            0xC9 => (Instructions::CompareAccumulator, AddressingMode::Immediate),
            0xB0 => (Instructions::BranchOnCarrySet, AddressingMode::Relative),
            0xCA => (Instructions::DecrementX, AddressingMode::Implied),
            0x88 => (Instructions::DecrementY, AddressingMode::Implied),
            0xD0 => (Instructions::BranchOnResultNotZero, AddressingMode::Relative),
            0x20 => (Instructions::JumpStoreReturnAddress, AddressingMode::Absolute),
            0xEE => (Instructions::IncrementMem, AddressingMode::Absolute),
            0x09 => (Instructions::ORAccumulator, AddressingMode::Immediate),
            0x4C => (Instructions::Jump, AddressingMode::Absolute),
            0x6C => (Instructions::Jump, AddressingMode::Indirect),
            0x01 => (Instructions::ORAccumulator, AddressingMode::XIndirect),
            0x11 => (Instructions::ORAccumulator, AddressingMode::YIndirect),
            0xC8 => (Instructions::IncrementY, AddressingMode::Implied),
            0xEC => (Instructions::CompareX, AddressingMode::Absolute),
            0x41 => (Instructions::EORAccumulator, AddressingMode::XIndirect),
            0x68 => (Instructions::PullAccumulatorFromStack, AddressingMode::Implied),
            0xDE => (Instructions::DecrementMem, AddressingMode::AbsoluteX),
            0x8E => (Instructions::StoreX, AddressingMode::Absolute),
            0x8C => (Instructions::StoreY, AddressingMode::Absolute),
            0x29 => (Instructions::ANDAccumulator, AddressingMode::Immediate),
            0xAC => (Instructions::LoadY, AddressingMode::Absolute),
            0xAE => (Instructions::LoadX, AddressingMode::Absolute),
            0x85 => (Instructions::StoreAccumulator, AddressingMode::ZeroPage),
            0xE0 => (Instructions::CompareX, AddressingMode::Immediate),
            0xBE => (Instructions::LoadX, AddressingMode::AbsoluteY),
            0x9D => (Instructions::StoreAccumulator, AddressingMode::AbsoluteX),
            0x4A => (Instructions::ShiftOneRight, AddressingMode::Accumulator),
            0xF0 => (Instructions::BranchOnResultZero, AddressingMode::Relative),
            0xCE => (Instructions::DecrementMem, AddressingMode::Absolute),
            0xE6 => (Instructions::IncrementMem, AddressingMode::ZeroPage),
            0xF6 => (Instructions::IncrementMem, AddressingMode::ZeroPageX),
            0x45 => (Instructions::EORAccumulator, AddressingMode::ZeroPage),
            0x18 => (Instructions::ClearCarry, AddressingMode::Implied),
            0x38 => (Instructions::SetCarry, AddressingMode::Implied),
            0x7E => (Instructions::RotateOneRight, AddressingMode::AbsoluteX),
            0xE8 => (Instructions::IncrementX, AddressingMode::Implied),
            0x48 => (Instructions::PushAccumulatorOnStack, AddressingMode::Implied),
            0x40 => (Instructions::ReturnFromInterrupt, AddressingMode::Implied),
            0x60 => (Instructions::ReturnFromSubroutine, AddressingMode::Implied),
            0xA8 => (Instructions::TransferAccumulatorToY, AddressingMode::Implied),
            0x84 => (Instructions::StoreY, AddressingMode::ZeroPage),
            0x49 => (Instructions::EORAccumulator, AddressingMode::Immediate),
            0xC5 => (Instructions::CompareAccumulator, AddressingMode::ZeroPage),
            0x90 => (Instructions::BranchOnCarryClear, AddressingMode::Relative),
            0x79 => (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::AbsoluteY),
            0x65 => (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::ZeroPage),
            0xB9 => (Instructions::LoadAccumulator, AddressingMode::AbsoluteY),
            0x69 => (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::Immediate),
            0x31 => (Instructions::ANDAccumulator, AddressingMode::YIndirect),
            0x2C => (Instructions::TestBitsAccumulator, AddressingMode::Absolute),
            0x24 => (Instructions::TestBitsAccumulator, AddressingMode::ZeroPage),
            0x99 => (Instructions::StoreAccumulator, AddressingMode::AbsoluteY),
            0x0D => (Instructions::ORAccumulator, AddressingMode::Absolute),
            0xC0 => (Instructions::CompareY, AddressingMode::Immediate),
            0x8A => (Instructions::TransferXToAccumulator, AddressingMode::Immediate),
            0x30 => (Instructions::BranchOnResultMinus, AddressingMode::Relative),
            0xA5 => (Instructions::LoadAccumulator, AddressingMode::ZeroPage),
            0x0A => (Instructions::ShiftOneLeft, AddressingMode::Accumulator),
            0x81 => (Instructions::StoreAccumulator, AddressingMode::XIndirect),
            0xC1 => (Instructions::CompareAccumulator, AddressingMode::XIndirect),
            0x05 => (Instructions::ORAccumulator, AddressingMode::ZeroPage),
            0x28 => (Instructions::PullProcessorStatusFromStack, AddressingMode::Implied),
            0x86 => (Instructions::StoreX, AddressingMode::ZeroPage),
            0xB4 => (Instructions::LoadY, AddressingMode::ZeroPageX),
            0x98 => (Instructions::TransferYToAccumulator, AddressingMode::Implied),
            0xE9 => (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::Immediate),
            0xF8 => (Instructions::SetDecimalMode, AddressingMode::Implied),
            0x50 => (Instructions::BranchOnOverflowClear, AddressingMode::Relative),
            0xFE => (Instructions::IncrementMem, AddressingMode::AbsoluteX),
            0xAA => (Instructions::TransferAccumulatorToX, AddressingMode::Implied),
            0xBC => (Instructions::LoadY, AddressingMode::AbsoluteX),
            0xA6 => (Instructions::LoadX, AddressingMode::ZeroPage),
            0xB5 => (Instructions::LoadAccumulator, AddressingMode::ZeroPageX),
            0x19 => (Instructions::ORAccumulator, AddressingMode::AbsoluteY),
            0x70 => (Instructions::BranchOnOverflowSet, AddressingMode::Relative),
            0x16 => (Instructions::ShiftOneLeft, AddressingMode::ZeroPageX),
            0x91 => (Instructions::StoreAccumulator, AddressingMode::YIndirect),
            0xC6 => (Instructions::DecrementMem, AddressingMode::ZeroPage),
            0x15 => (Instructions::ORAccumulator, AddressingMode::ZeroPageX),
            0x1D => (Instructions::ORAccumulator, AddressingMode::AbsoluteX),
            0x0E => (Instructions::ShiftOneLeft, AddressingMode::Absolute),
            0x2E => (Instructions::RotateOneLeft, AddressingMode::Absolute),
            0x21 => (Instructions::ANDAccumulator, AddressingMode::XIndirect),
            0xCD => (Instructions::CompareAccumulator, AddressingMode::Absolute),
            0x25 => (Instructions::ANDAccumulator, AddressingMode::ZeroPage),
            0x26 => (Instructions::RotateOneLeft, AddressingMode::ZeroPage),
            0x36 => (Instructions::RotateOneLeft, AddressingMode::ZeroPageX),
            0x46 => (Instructions::ShiftOneRight, AddressingMode::ZeroPage),
            0x56 => (Instructions::ShiftOneRight, AddressingMode::ZeroPageX),
            0x2D => (Instructions::ANDAccumulator, AddressingMode::Absolute),
            0x3D => (Instructions::ANDAccumulator, AddressingMode::AbsoluteX),
            0x39 => (Instructions::ANDAccumulator, AddressingMode::AbsoluteY),
            0x08 => (Instructions::PushProcessorStatusOnStack, AddressingMode::Implied),
            0x5D => (Instructions::EORAccumulator, AddressingMode::AbsoluteX),
            0x59 => (Instructions::EORAccumulator, AddressingMode::AbsoluteY),
            0x6E => (Instructions::RotateOneRight, AddressingMode::Absolute),
            0x2A => (Instructions::RotateOneLeft, AddressingMode::Accumulator),
            0x06 => (Instructions::ShiftOneLeft, AddressingMode::ZeroPage),
            0xA1 => (Instructions::LoadAccumulator, AddressingMode::XIndirect),
            0xB1 => (Instructions::LoadAccumulator, AddressingMode::YIndirect),
            0xA4 => (Instructions::LoadY, AddressingMode::ZeroPage),
            0x4E => (Instructions::ShiftOneRight, AddressingMode::Accumulator),
            0x35 => (Instructions::ANDAccumulator, AddressingMode::ZeroPageX),
            0xBA => (Instructions::TransferStackPointerToX, AddressingMode::Implied),
            0x66 => (Instructions::RotateOneRight, AddressingMode::ZeroPage),
            0x6A => (Instructions::RotateOneRight, AddressingMode::Accumulator),
            0x4D => (Instructions::EORAccumulator, AddressingMode::Absolute),
            0x51 => (Instructions::EORAccumulator, AddressingMode::YIndirect),
            0x6D => (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::Absolute),
            0x61 => (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::XIndirect),
            0x71 => (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::YIndirect),
            0x76 => (Instructions::RotateOneRight, AddressingMode::ZeroPageX),
            0xB6 => (Instructions::LoadX, AddressingMode::ZeroPageY),
            0x5E => (Instructions::ShiftOneRight, AddressingMode::AbsoluteX),
            0xCC => (Instructions::CompareY, AddressingMode::Absolute),
            0x58 => (Instructions::ClearInterruptDisable, AddressingMode::Implied),
            0x1E => (Instructions::ShiftOneLeft, AddressingMode::AbsoluteX),
            0xF9 => (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::AbsoluteY),
            0x55 => (Instructions::EORAccumulator, AddressingMode::ZeroPageX),
            0xD1 => (Instructions::CompareAccumulator, AddressingMode::YIndirect),
            0xFD => (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::AbsoluteX),
            0x95 => (Instructions::StoreAccumulator, AddressingMode::ZeroPageX),
            0xD9 => (Instructions::CompareAccumulator, AddressingMode::AbsoluteY),
            0x96 => (Instructions::StoreX, AddressingMode::ZeroPageY),
            0x94 => (Instructions::StoreY, AddressingMode::ZeroPageX),
            0xDD => (Instructions::CompareAccumulator, AddressingMode::AbsoluteX),
            0xB8 => (Instructions::ClearOverflow, AddressingMode::Implied),
            0xD6 => (Instructions::DecrementMem, AddressingMode::ZeroPageX),
            0xC4 => (Instructions::CompareY, AddressingMode::ZeroPage),
            0x7D => (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::AbsoluteX),
            0x75 => (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::ZeroPageX),
            0xE4 => (Instructions::CompareX, AddressingMode::ZeroPage),
            0xD5 => (Instructions::CompareAccumulator, AddressingMode::ZeroPageX),
            0xED => (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::Absolute),
            0xE5 => (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::ZeroPage),
            0xF5 => (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::ZeroPageX),
            0xE1 => (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::XIndirect),
            0xF1 => (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::YIndirect),
            0x3E => (Instructions::RotateOneLeft, AddressingMode::AbsoluteX),

            // illegal -- TODO names for these
            0x03 => (Instructions::SLO, AddressingMode::XIndirect),
            0x07 => (Instructions::SLO, AddressingMode::ZeroPage),
            0x13 => (Instructions::SLO, AddressingMode::YIndirect),
            0x17 => (Instructions::SLO, AddressingMode::ZeroPageX),
            0x1F => (Instructions::SLO, AddressingMode::AbsoluteX),
            0x0F => (Instructions::SLO, AddressingMode::Absolute),
            0x1B => (Instructions::SLO, AddressingMode::AbsoluteY),

            0xE7 => (Instructions::ISC, AddressingMode::ZeroPage),
            0xF3 => (Instructions::ISC, AddressingMode::YIndirect),
            0xE3 => (Instructions::ISC, AddressingMode::XIndirect),
            0xEF => (Instructions::ISC, AddressingMode::Absolute),
            0xFB => (Instructions::ISC, AddressingMode::AbsoluteY),
            0xFF => (Instructions::ISC, AddressingMode::AbsoluteX),
            0xF7 => (Instructions::ISC, AddressingMode::ZeroPageX),

            0x27 => (Instructions::RLA, AddressingMode::ZeroPage),
            0x23 => (Instructions::RLA, AddressingMode::XIndirect),
            0x37 => (Instructions::RLA, AddressingMode::ZeroPage),
            0x2F => (Instructions::RLA, AddressingMode::Absolute),
            0x3B => (Instructions::RLA, AddressingMode::AbsoluteY),
            0x33 => (Instructions::RLA, AddressingMode::YIndirect),
            0x3F => (Instructions::RLA, AddressingMode::AbsoluteX),

            0x67 => (Instructions::RRA, AddressingMode::ZeroPage),
            0x63 => (Instructions::RRA, AddressingMode::XIndirect),
            0x7F => (Instructions::RRA, AddressingMode::AbsoluteX),
            0x7B => (Instructions::RRA, AddressingMode::AbsoluteY),
            0x73 => (Instructions::RRA, AddressingMode::YIndirect),
            0x6F => (Instructions::RRA, AddressingMode::Absolute),
            0x77 => (Instructions::RRA, AddressingMode::ZeroPageX),

            0xA3 => (Instructions::LAX, AddressingMode::XIndirect),
            0xA7 => (Instructions::LAX, AddressingMode::ZeroPage),
            0xAF => (Instructions::LAX, AddressingMode::Absolute),
            0xBF => (Instructions::LAX, AddressingMode::AbsoluteY),
            0xB7 => (Instructions::LAX, AddressingMode::ZeroPageY),
            0xB3 => (Instructions::LAX, AddressingMode::YIndirect),

            0x5B => (Instructions::SRE, AddressingMode::AbsoluteY),
            0x43 => (Instructions::SRE, AddressingMode::XIndirect),
            0x53 => (Instructions::SRE, AddressingMode::YIndirect),
            0x5F => (Instructions::SRE, AddressingMode::AbsoluteX),
            0x4F => (Instructions::SRE, AddressingMode::Absolute),
            0x47 => (Instructions::SRE, AddressingMode::ZeroPage),
            0x57 => (Instructions::SRE, AddressingMode::ZeroPageX),

            0xC3 => (Instructions::DCP, AddressingMode::XIndirect),
            0xD3 => (Instructions::DCP, AddressingMode::YIndirect),
            0xDB => (Instructions::DCP, AddressingMode::AbsoluteY),
            0xC7 => (Instructions::DCP, AddressingMode::ZeroPage),
            0xD7 => (Instructions::DCP, AddressingMode::ZeroPageX),
            0xDF => (Instructions::DCP, AddressingMode::AbsoluteX),
            0xCF => (Instructions::DCP, AddressingMode::Absolute),

            0xEB => (Instructions::USBC, AddressingMode::Immediate),

            0x97 => (Instructions::SAX, AddressingMode::ZeroPageY),
            0x8F => (Instructions::SAX, AddressingMode::Absolute),
            0x83 => (Instructions::SAX, AddressingMode::XIndirect),

            0x6B => (Instructions::ARR, AddressingMode::Immediate),
            0x4B => (Instructions::ALR, AddressingMode::Immediate),

            0x9E => (Instructions::SHX, AddressingMode::AbsoluteY),
            0x9C => (Instructions::SHY, AddressingMode::AbsoluteX),
            0x9F => (Instructions::SHA, AddressingMode::AbsoluteY),
            0x93 => (Instructions::SHA, AddressingMode::YIndirect),

            0x2B => (Instructions::ANC, AddressingMode::Immediate), // effectively the same as 0x0B
            0x0B => (Instructions::ANC, AddressingMode::Immediate),

            0x8B => (Instructions::ANE, AddressingMode::Immediate),
            0x87 => (Instructions::SAX, AddressingMode::ZeroPage),

            0x9B => (Instructions::TAS, AddressingMode::AbsoluteY),

            0xBB => (Instructions::LAS, AddressingMode::AbsoluteY),
            0xAB => (Instructions::LXA, AddressingMode::Immediate),


            0xCB => (Instructions::SBX, AddressingMode::Immediate),

            // noop
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xEA | 0xFA =>
                (Instructions::NoOperation, AddressingMode::Implied),

            0x04 | 0x44 | 0x64 | 0x89 =>
                (Instructions::NoOperation, AddressingMode::ZeroPage),

            0x80 | 0x82 | 0xC2 | 0xE2 =>
                (Instructions::NoOperation, AddressingMode::Immediate),

            0x14 | 0x34 | 0x54 | 0x74| 0xD4 | 0xF4 =>
                (Instructions::NoOperation, AddressingMode::ZeroPageX),

            0x0C =>
                (Instructions::NoOperation, AddressingMode::Absolute),

            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC =>
                (Instructions::NoOperation, AddressingMode::AbsoluteX),

            // jam
            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 =>
                (Instructions::JAM, AddressingMode::Implied),

            // software breakpoint
            0x00 => (Instructions::ForceBreak, AddressingMode::Implied),

            _ => (Instructions::MissingOperation, AddressingMode::Implied),
        }
    }

    pub fn execute(&mut self, operation: (&Instructions, &AddressingMode)) {
        match operation {
            (Instructions::ForceBreak, AddressingMode::Implied) => {self.breakpoint()}
            (Instructions::MissingOperation, AddressingMode::Implied) => {panic!("Missing operation??")}

            (_, _) => {println!("Unknown pattern! {:?}, {:?}", operation.0, operation.1) }
        }
    }

    pub fn set_pc(&mut self, addr: u16) {
        self.reg.pc = addr;
    }

    pub fn fetch_decode_next(&mut self) {
        if self.reg.pc >= ADDR_HI {
            eprintln!();
            panic!("PC counter too high! {}", self.reg.pc)
        }
        let next_instruction = self.memory.read_byte(self.reg.pc);
        let (instruction, addressing_mode) = Self::decode(next_instruction);

        // increment pc for each instruction based on instruction type


        // dbg!(&instruction);

        self.execute((&instruction, &addressing_mode));
        self.reg.pc += addressing_mode.get_increment();
    }

    // TODO - works with mapper 0 only
    pub fn load_rom(&mut self, rom: &NesRom) {
        self.memory.write_bytes(0x8000, &rom.prg_rom[0]);
        if rom.prg_rom.len() > 1 {
            self.memory.write_bytes(0xC000, &rom.prg_rom[1]);
        }

        self.set_pc(0x8000);
    }

    fn breakpoint(&self) {
        // Create a new instance of stdin
        let mut stdin = io::stdin();
        // add PC
        println!("BREAKPOINT: {}", "PC");

        // Buffer to hold the input
        let mut input = String::new();

        // Wait for user input
        stdin.read_line(&mut input).expect("Failed to read line");
    }

}


// https://www.nesdev.org/wiki/2A03
pub struct Registers {
    pub pc: u16,
    sp: u8,
    accumulator: u8,
    idx: u8,
    idy: u8,
    flags: u8
}

impl Registers {
    fn new() -> Self {
        // TODO
        Registers {
            pc: 0,
            sp: 0,
            accumulator: 0,
            idx: 0,
            idy: 0,
            flags: 0,
        }
    }
}


// TODO
// Carry flag
// Zero flag
// Interrupt disable
// decimal mode
// break command
// overflow flag
// negative flag
#[repr(u8)]
enum CPUFlags {
    Carry  = 1,
    Zero   = 2,
    InterruptDisabled = 4, // maybe rename?
    DecimalMode = 8,
    Overflow = 0x40,
    Negative = 0x80
}

// set interrupt disable status
// fn sei(cpu: &mut NesCpu) {
//     cpu.reg.flags = CPU.reg.flags | CPUFlags::InterruptDisabled;
// }

