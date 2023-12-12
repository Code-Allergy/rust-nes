use crate::memory::{Bus, Memory, ADDR_HI};
use crate::{combine_bytes_to_u16, NesRom};
use std::io;
use std::process::exit;

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
    ZeroPageY,
}

pub struct CurrentInstruction {
    op: Instructions,
    mode: AddressingMode,
}
impl CurrentInstruction {
    fn new() -> Self {
        Self {
            op: Instructions::JAM,
            mode: AddressingMode::Implied,
        }
    }
}

impl AddressingMode {
    fn get_increment(&self) -> u16 {
        match self {
            AddressingMode::Implied | AddressingMode::Accumulator => 1,

            AddressingMode::Immediate
            | AddressingMode::XIndirect
            | AddressingMode::YIndirect
            | AddressingMode::ZeroPage
            | AddressingMode::ZeroPageX
            | AddressingMode::ZeroPageY
            | AddressingMode::Relative => 2,

            AddressingMode::Absolute
            | AddressingMode::AbsoluteX
            | AddressingMode::AbsoluteY
            | AddressingMode::Indirect => 3,
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
    JumpSubroutine,
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
    SHX,
}

// https://www.nesdev.org/wiki/2A03
#[derive(Debug)]
pub struct Registers {
    pub pc: u16,
    sp: u8,
    accumulator: u8,
    idx: u8,
    idy: u8,
    flags: CPUFlags,
}

impl Registers {
    fn new() -> Self {
        // TODO
        Registers {
            pc: 0,
            sp: 0xFF,
            accumulator: 0,
            idx: 0,
            idy: 0,
            flags: CPUFlags::new(),
        }
    }
}

// TODO
// Carry flag
// zero flag
// Interrupt disable
// decimal mode
// break command
// overflow flag
// negative flag
#[derive(Debug)]
struct CPUFlags {
    carry: bool,
    zero: bool,
    interrupt_disable: bool,
    decimal: bool, // nes unused?
    overflow: bool,
    negative: bool,
}

impl CPUFlags {
    fn new() -> Self {
        CPUFlags {
            carry: false,
            zero: false,
            interrupt_disable: true,
            decimal: false,
            overflow: false,
            negative: false,
        }
    }

    fn set_byte(&mut self, byte: u8) {
        self.carry = 0b0000_0001 & byte != 0;
        self.zero = 0b0000_0010 & byte != 0;
        self.interrupt_disable = 0b0000_0100 & byte != 0;
        self.decimal = 0b0000_1000 & byte != 0;
        self.overflow = 0b0100_0000 & byte != 0;
        self.negative = 0b1000_0000 & byte != 0;
    }

    fn as_byte(&self) -> u8 {
        let mut result = 0;

        // Set individual bits based on flag values
        result |= if self.carry { 0b0000_0001 } else { 0 };
        result |= if self.zero { 0b0000_0010 } else { 0 };
        result |= if self.interrupt_disable {
            0b0000_0100
        } else {
            0
        };
        result |= if self.decimal { 0b0000_1000 } else { 0 };
        result |= if self.overflow { 0b0100_0000 } else { 0 };
        result |= if self.negative { 0b1000_0000 } else { 0 };

        result
    }
}

pub struct NesCpu {
    pub memory: Memory,
    pub reg: Registers,
    pub current: CurrentInstruction,
}

impl NesCpu {
    pub fn new() -> Self {
        NesCpu {
            memory: Memory::default(),
            reg: Registers::new(),
            current: CurrentInstruction::new(),
        }
    }
    pub fn next_byte(&self) -> u8 {
        self.memory.read_byte(self.reg.pc + 1)
    }

    pub fn next_word(&self) -> u16 {
        self.memory.read_word(self.reg.pc + 1)
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
            0x10 => (Instructions::BranchOnResultPlus, AddressingMode::Relative),
            0xA0 => (Instructions::LoadY, AddressingMode::Immediate),
            0xBD => (Instructions::LoadAccumulator, AddressingMode::AbsoluteX),
            0xC9 => (Instructions::CompareAccumulator, AddressingMode::Immediate),
            0xB0 => (Instructions::BranchOnCarrySet, AddressingMode::Relative),
            0xCA => (Instructions::DecrementX, AddressingMode::Implied),
            0x88 => (Instructions::DecrementY, AddressingMode::Implied),
            0xD0 => (
                Instructions::BranchOnResultNotZero,
                AddressingMode::Relative,
            ),
            0x20 => (Instructions::JumpSubroutine, AddressingMode::Absolute),
            0xEE => (Instructions::IncrementMem, AddressingMode::Absolute),
            0x09 => (Instructions::ORAccumulator, AddressingMode::Immediate),
            0x4C => (Instructions::Jump, AddressingMode::Absolute),
            0x6C => (Instructions::Jump, AddressingMode::Indirect),
            0x01 => (Instructions::ORAccumulator, AddressingMode::XIndirect),
            0x11 => (Instructions::ORAccumulator, AddressingMode::YIndirect),
            0xC8 => (Instructions::IncrementY, AddressingMode::Implied),
            0xEC => (Instructions::CompareX, AddressingMode::Absolute),
            0x41 => (Instructions::EORAccumulator, AddressingMode::XIndirect),
            0x68 => (
                Instructions::PullAccumulatorFromStack,
                AddressingMode::Implied,
            ),
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
            0xC6 => (Instructions::DecrementMem, AddressingMode::ZeroPage),
            0xCE => (Instructions::DecrementMem, AddressingMode::Absolute),
            0xE6 => (Instructions::IncrementMem, AddressingMode::ZeroPage),
            0xF6 => (Instructions::IncrementMem, AddressingMode::ZeroPageX),
            0x45 => (Instructions::EORAccumulator, AddressingMode::ZeroPage),
            0x18 => (Instructions::ClearCarry, AddressingMode::Implied),
            0x38 => (Instructions::SetCarry, AddressingMode::Implied),
            0x7E => (Instructions::RotateOneRight, AddressingMode::AbsoluteX),
            0xE8 => (Instructions::IncrementX, AddressingMode::Implied),
            0x48 => (
                Instructions::PushAccumulatorOnStack,
                AddressingMode::Implied,
            ),
            0x40 => (Instructions::ReturnFromInterrupt, AddressingMode::Implied),
            0x60 => (Instructions::ReturnFromSubroutine, AddressingMode::Implied),
            0xA8 => (
                Instructions::TransferAccumulatorToY,
                AddressingMode::Implied,
            ),
            0x84 => (Instructions::StoreY, AddressingMode::ZeroPage),
            0x49 => (Instructions::EORAccumulator, AddressingMode::Immediate),
            0xC5 => (Instructions::CompareAccumulator, AddressingMode::ZeroPage),
            0x90 => (Instructions::BranchOnCarryClear, AddressingMode::Relative),
            0x79 => (
                Instructions::AddMemToAccumulatorWithCarry,
                AddressingMode::AbsoluteY,
            ),
            0x65 => (
                Instructions::AddMemToAccumulatorWithCarry,
                AddressingMode::ZeroPage,
            ),
            0xB9 => (Instructions::LoadAccumulator, AddressingMode::AbsoluteY),
            0x69 => (
                Instructions::AddMemToAccumulatorWithCarry,
                AddressingMode::Immediate,
            ),
            0x31 => (Instructions::ANDAccumulator, AddressingMode::YIndirect),
            0x2C => (Instructions::TestBitsAccumulator, AddressingMode::Absolute),
            0x24 => (Instructions::TestBitsAccumulator, AddressingMode::ZeroPage),
            0x99 => (Instructions::StoreAccumulator, AddressingMode::AbsoluteY),
            0x0D => (Instructions::ORAccumulator, AddressingMode::Absolute),
            0xC0 => (Instructions::CompareY, AddressingMode::Immediate),
            0x8A => (
                Instructions::TransferXToAccumulator,
                AddressingMode::Immediate,
            ),
            0x30 => (Instructions::BranchOnResultMinus, AddressingMode::Relative),
            0xA5 => (Instructions::LoadAccumulator, AddressingMode::ZeroPage),
            0x0A => (Instructions::ShiftOneLeft, AddressingMode::Accumulator),
            0x81 => (Instructions::StoreAccumulator, AddressingMode::XIndirect),
            0xC1 => (Instructions::CompareAccumulator, AddressingMode::XIndirect),
            0x05 => (Instructions::ORAccumulator, AddressingMode::ZeroPage),
            0x28 => (
                Instructions::PullProcessorStatusFromStack,
                AddressingMode::Implied,
            ),
            0x86 => (Instructions::StoreX, AddressingMode::ZeroPage),
            0xB4 => (Instructions::LoadY, AddressingMode::ZeroPageX),
            0x98 => (
                Instructions::TransferYToAccumulator,
                AddressingMode::Implied,
            ),
            0xE9 => (
                Instructions::SubtractAccumulatorWithBorrow,
                AddressingMode::Immediate,
            ),
            0xF8 => (Instructions::SetDecimalMode, AddressingMode::Implied),
            0x50 => (
                Instructions::BranchOnOverflowClear,
                AddressingMode::Relative,
            ),
            0xFE => (Instructions::IncrementMem, AddressingMode::AbsoluteX),
            0xAA => (
                Instructions::TransferAccumulatorToX,
                AddressingMode::Implied,
            ),
            0xBC => (Instructions::LoadY, AddressingMode::AbsoluteX),
            0xA6 => (Instructions::LoadX, AddressingMode::ZeroPage),
            0xB5 => (Instructions::LoadAccumulator, AddressingMode::ZeroPageX),
            0x19 => (Instructions::ORAccumulator, AddressingMode::AbsoluteY),
            0x70 => (Instructions::BranchOnOverflowSet, AddressingMode::Relative),
            0x16 => (Instructions::ShiftOneLeft, AddressingMode::ZeroPageX),
            0x91 => (Instructions::StoreAccumulator, AddressingMode::YIndirect),

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
            0x08 => (
                Instructions::PushProcessorStatusOnStack,
                AddressingMode::Implied,
            ),
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
            0xBA => (
                Instructions::TransferStackPointerToX,
                AddressingMode::Implied,
            ),
            0x66 => (Instructions::RotateOneRight, AddressingMode::ZeroPage),
            0x6A => (Instructions::RotateOneRight, AddressingMode::Accumulator),
            0x4D => (Instructions::EORAccumulator, AddressingMode::Absolute),
            0x51 => (Instructions::EORAccumulator, AddressingMode::YIndirect),
            0x6D => (
                Instructions::AddMemToAccumulatorWithCarry,
                AddressingMode::Absolute,
            ),
            0x61 => (
                Instructions::AddMemToAccumulatorWithCarry,
                AddressingMode::XIndirect,
            ),
            0x71 => (
                Instructions::AddMemToAccumulatorWithCarry,
                AddressingMode::YIndirect,
            ),
            0x76 => (Instructions::RotateOneRight, AddressingMode::ZeroPageX),
            0xB6 => (Instructions::LoadX, AddressingMode::ZeroPageY),
            0x5E => (Instructions::ShiftOneRight, AddressingMode::AbsoluteX),
            0xCC => (Instructions::CompareY, AddressingMode::Absolute),
            0x58 => (Instructions::ClearInterruptDisable, AddressingMode::Implied),
            0x1E => (Instructions::ShiftOneLeft, AddressingMode::AbsoluteX),
            0xF9 => (
                Instructions::SubtractAccumulatorWithBorrow,
                AddressingMode::AbsoluteY,
            ),
            0x55 => (Instructions::EORAccumulator, AddressingMode::ZeroPageX),
            0xD1 => (Instructions::CompareAccumulator, AddressingMode::YIndirect),
            0xFD => (
                Instructions::SubtractAccumulatorWithBorrow,
                AddressingMode::AbsoluteX,
            ),
            0x95 => (Instructions::StoreAccumulator, AddressingMode::ZeroPageX),
            0xD9 => (Instructions::CompareAccumulator, AddressingMode::AbsoluteY),
            0x96 => (Instructions::StoreX, AddressingMode::ZeroPageY),
            0x94 => (Instructions::StoreY, AddressingMode::ZeroPageX),
            0xDD => (Instructions::CompareAccumulator, AddressingMode::AbsoluteX),
            0xB8 => (Instructions::ClearOverflow, AddressingMode::Implied),
            0xD6 => (Instructions::DecrementMem, AddressingMode::ZeroPageX),
            0xC4 => (Instructions::CompareY, AddressingMode::ZeroPage),
            0x7D => (
                Instructions::AddMemToAccumulatorWithCarry,
                AddressingMode::AbsoluteX,
            ),
            0x75 => (
                Instructions::AddMemToAccumulatorWithCarry,
                AddressingMode::ZeroPageX,
            ),
            0xE4 => (Instructions::CompareX, AddressingMode::ZeroPage),
            0xD5 => (Instructions::CompareAccumulator, AddressingMode::ZeroPageX),
            0xED => (
                Instructions::SubtractAccumulatorWithBorrow,
                AddressingMode::Absolute,
            ),
            0xE5 => (
                Instructions::SubtractAccumulatorWithBorrow,
                AddressingMode::ZeroPage,
            ),
            0xF5 => (
                Instructions::SubtractAccumulatorWithBorrow,
                AddressingMode::ZeroPageX,
            ),
            0xE1 => (
                Instructions::SubtractAccumulatorWithBorrow,
                AddressingMode::XIndirect,
            ),
            0xF1 => (
                Instructions::SubtractAccumulatorWithBorrow,
                AddressingMode::YIndirect,
            ),
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
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xEA | 0xFA => {
                (Instructions::NoOperation, AddressingMode::Implied)
            }

            0x04 | 0x44 | 0x64 | 0x89 => (Instructions::NoOperation, AddressingMode::ZeroPage),

            0x80 | 0x82 | 0xC2 | 0xE2 => (Instructions::NoOperation, AddressingMode::Immediate),

            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
                (Instructions::NoOperation, AddressingMode::ZeroPageX)
            }

            0x0C => (Instructions::NoOperation, AddressingMode::Absolute),

            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                (Instructions::NoOperation, AddressingMode::AbsoluteX)
            }

            // jam
            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                (Instructions::JAM, AddressingMode::Implied)
            }

            // software breakpoint
            0x00 => (Instructions::ForceBreak, AddressingMode::Implied),

            _ => (Instructions::MissingOperation, AddressingMode::Implied),
        }
    }

    fn set_interrupts_disabled(&mut self, status: bool) {
        self.reg.flags.interrupt_disable = status;
        println!("Interrupts Disabled: {}", status);
        self.reg.pc += 1;
    }

    fn set_decimal(&mut self, status: bool) {
        self.reg.flags.interrupt_disable = status;
        println!("Decimal bit: {}", status);
        self.reg.pc += 1;
    }

    fn set_carry(&mut self, status: bool) {
        self.reg.flags.carry = status;
        println!("Carry bit: {}", status);
        self.reg.pc += 1;
    }

    fn set_overflow(&mut self, status: bool) {
        self.reg.flags.overflow = status;
        println!("Overflow bit: {}", status);
        self.reg.pc += 1;
    }

    fn push_stack(&mut self, data: u8) {
        let address: u16 = 0x100 + self.reg.sp as u16;
        println!(
            "Initial SP: 0x{:x} w/ offset 0x{:x} PC: 0x{:x}",
            self.reg.sp, &address, self.reg.pc
        );
        self.memory.write_byte(address, data);
        self.reg.sp -= 1;
        println!(
            "Stack push (pointer: 0x{1:x})! {} (0x{0:X})",
            data, self.reg.sp
        );
    }

    fn pop_stack(&mut self) -> u8 {
        let address: u16 = 0x100 + self.reg.sp as u16;
        self.reg.sp += 1;
        let res = self.memory.read_byte(address + 1);
        println!(
            "Stack pop (pointer: 0x{1:x})! {} (0x{0:X})",
            res, self.reg.sp
        );
        res
    }

    // togo
    fn add_mem_to_accumulator_with_carry_absolute_x(&mut self) {
        let address = self.memory.read_word(self.reg.pc + 1);
        let offset = self.reg.idx;
        // Read the value from memory at the specified address + X offset
        let operand = self.memory.read_byte(address + u16::from(offset));

        // Perform addition
        let (result, carry_out) = self.reg.accumulator.overflowing_add(operand);

        // Update the carry flag
        self.reg.flags.carry = carry_out;

        // Update the overflow flag
        self.reg.flags.overflow =
            (self.reg.accumulator ^ operand) & !(self.reg.accumulator ^ result) & 0x80 != 0;

        // Update the zero and negative flags
        self.reg.flags.zero = result == 0;
        self.reg.flags.negative = result & 0x80 != 0;

        // Update the accumulator with the result
        self.reg.accumulator = result;
        println!("ADDED MEM TO A, WITH CARRY {}", self.reg.accumulator);
        self.reg.pc += AddressingMode::AbsoluteX.get_increment();
    }

    fn test_bit(&mut self, addressing_mode: &AddressingMode) {
        let operand = match addressing_mode {
            AddressingMode::ZeroPage => self.memory.read_byte(self.reg.pc + 1),
            _ => {
                panic!("test_bit not implemented for mode {:?}", addressing_mode)
            }
        };
        // Extract bits 6 and 7 from the operand
        let bit_6 = (operand >> 6) & 0b1;
        let bit_7 = (operand >> 7) & 0b1;
        println!("OPERAND: {:b}", operand);
        // Transfer bits 6 and 7 to bits 6 and 7 of the status register
        self.reg.flags.overflow = bit_6 == 1;
        self.reg.flags.negative = bit_7 == 1;

        // Perform bitwise AND between the accumulator and the operand
        let result = self.reg.accumulator & operand;

        // Update zero flag based on the result
        self.reg.flags.zero = result == 0;

        self.reg.pc += addressing_mode.get_increment();
    }

    fn load_register(&mut self, addressing_mode: &AddressingMode, reg_name: &str) {
        let address = match addressing_mode {
            AddressingMode::Absolute | AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
                self.memory.read_word(self.reg.pc + 1)
            }
            AddressingMode::ZeroPage => self.memory.read_byte(self.reg.pc + 1) as u16,
            _ => 0,
        };

        let value = match addressing_mode {
            AddressingMode::Immediate => self.memory.read_byte(self.reg.pc + 1),
            AddressingMode::Absolute => self.memory.read_byte(address),
            AddressingMode::AbsoluteX => self.memory.read_byte(address + self.reg.idx as u16),
            AddressingMode::AbsoluteY => self.memory.read_byte(address + self.reg.idy as u16),
            AddressingMode::ZeroPage => self.memory.read_byte(address),
            _ => panic!("Load Not implemented! {:?}", addressing_mode),
        };

        match reg_name {
            "A" => self.reg.accumulator = value,
            "X" => self.reg.idx = value,
            "Y" => self.reg.idy = value,
            "SP" => self.reg.sp = value,
            _ => panic!("Unknown register: {}", reg_name),
        }
        println!(
            "Load {}: {1} (0x{1:X}) {2:?}",
            reg_name, value, &addressing_mode
        );
        self.reg.pc += addressing_mode.get_increment();
    }

    fn store_register(&mut self, addressing_mode: &AddressingMode, register_name: &str) {
        let address = match addressing_mode {
            AddressingMode::Absolute => self.memory.read_word(self.reg.pc + 1),
            AddressingMode::ZeroPage => self.memory.read_byte(self.reg.pc + 1) as u16,
            AddressingMode::ZeroPageX => {
                let zero_pg = self.memory.read_byte(self.reg.pc + 1);
                (zero_pg + self.reg.idx) as u16
            }
            AddressingMode::ZeroPageY => {
                let zero_pg = self.memory.read_byte(self.reg.pc + 1);
                (zero_pg + self.reg.idy) as u16
            }

            _ => panic!("Store Not implemented! {:?}", addressing_mode),
        };

        let register = match register_name {
            "A" => self.reg.accumulator,
            "X" => self.reg.idx,
            "Y" => self.reg.idy,
            "SP" => self.reg.sp,
            _ => panic!("Unknown register: {}", register_name),
        };

        self.memory.write_byte(address, register);
        println!(
            "Stored {0}: Val: {2} (0x{2:X}) 0x{1:X} PC: 0x{3:x}",
            register_name, address, register, self.reg.pc
        );
        self.reg.pc += addressing_mode.get_increment();
    }

    fn shift_one_left(&mut self, addressing_mode: &AddressingMode) {
        let address = match addressing_mode {
            AddressingMode::Absolute => self.memory.read_word(self.reg.pc + 1),
            _ => 0,
        };

        let byte = self.memory.read_byte(address);
        self.memory.write_byte(address, byte << 1);
        println!(
            "Shifting one bit left at addr: {}, old: {} new: {}",
            address,
            byte,
            byte << 1
        );
        self.reg.pc += addressing_mode.get_increment();
    }

    fn increase_register(&mut self, addressing_mode: &AddressingMode, register_name: &str) {
        let register = match register_name {
            "A" => &mut self.reg.accumulator,
            "X" => &mut self.reg.idx,
            "Y" => &mut self.reg.idy,
            "SP" => &mut self.reg.sp,
            _ => panic!("Unknown register: {}", register_name),
        };
        *register = register.wrapping_add(1);
        if *register == 0 {
            self.reg.flags.zero = true;
            self.reg.flags.overflow = true;
        } else {
            self.reg.flags.zero = false;
        };
        println!("Increased {0}: Val: {1} (0x{1:x})", register_name, register);
        self.reg.pc += addressing_mode.get_increment();
    }

    // todo set zero bit if == 0, negative bit if negative.
    fn decrease_register(&mut self, addressing_mode: &AddressingMode, register_name: &str) {
        let register = match register_name {
            "A" => &mut self.reg.accumulator,
            "X" => &mut self.reg.idx,
            "Y" => &mut self.reg.idy,
            "SP" => &mut self.reg.sp,
            _ => panic!("Unknown register: {}", register_name),
        };
        *register = register.wrapping_sub(1);
        if *register == 0xFF {
            self.reg.flags.zero = false;
            self.reg.flags.negative = true;
        } else if *register == 0 {
            self.reg.flags.zero = true;
        } else {
            self.reg.flags.zero = false;
        }
        println!("Decreased {0}: Val: {1} (0x{1:x})", register_name, register);
        self.reg.pc += addressing_mode.get_increment();
    }

    pub fn execute(&mut self, operation: (&Instructions, &AddressingMode)) {
        match operation {
            (Instructions::Jump, AddressingMode::Absolute) => {
                self.jump(self.memory.read_word(self.reg.pc + 1));
            }
            (Instructions::Jump, AddressingMode::Indirect) => {
                self.reg.pc += operation.1.get_increment();
                println!("Indirect jump not implemented!");
            }

            (Instructions::JumpSubroutine, AddressingMode::Absolute) => {
                let ra_bytes = (self.reg.pc + 3).to_le_bytes();
                self.push_stack(ra_bytes[0]);
                self.push_stack(ra_bytes[1]);
                self.jump(self.memory.read_word(self.reg.pc + 1));
                println!("Jump SUBROUTINE")
            }

            (Instructions::ReturnFromSubroutine, AddressingMode::Implied) => {
                let low = self.pop_stack();
                let hi = self.pop_stack();
                self.jump(u16::from_le_bytes([low, hi]));
                println!("Return JUMP SUBROUTINE")
            }

            (Instructions::CompareAccumulator, AddressingMode::Immediate)
            | (Instructions::CompareAccumulator, AddressingMode::AbsoluteY) => {
                self.compare_register(operation.1, "A");
            }
            (Instructions::CompareX, AddressingMode::Immediate) => {
                self.compare_register(operation.1, "X");
            }
            (Instructions::CompareY, AddressingMode::Immediate) => {
                self.compare_register(operation.1, "Y");
            }

            (Instructions::SetInterruptDisable, AddressingMode::Implied) => {
                self.set_interrupts_disabled(true);
            }

            (Instructions::ClearDecimalMode, AddressingMode::Implied) => {
                self.set_decimal(false);
            }

            /* load registers */
            (Instructions::LoadAccumulator, AddressingMode::Immediate)
            | (Instructions::LoadAccumulator, AddressingMode::ZeroPage)
            | (Instructions::LoadAccumulator, AddressingMode::Absolute)
            | (Instructions::LoadAccumulator, AddressingMode::AbsoluteX) => {
                self.load_register(operation.1, "A");
            }

            (Instructions::PushAccumulatorOnStack, AddressingMode::Implied) => {
                self.push_stack(self.reg.accumulator);
                self.reg.pc += operation.1.get_increment()
            }

            (Instructions::PullAccumulatorFromStack, AddressingMode::Implied) => {
                self.reg.accumulator = self.pop_stack();
                self.reg.pc += operation.1.get_increment()
            }

            (Instructions::LoadX, AddressingMode::Immediate)
            | (Instructions::LoadX, AddressingMode::ZeroPage)
            | (Instructions::LoadX, AddressingMode::Absolute) => {
                self.load_register(operation.1, "X")
            }

            (Instructions::LoadY, AddressingMode::Immediate)
            | (Instructions::LoadY, AddressingMode::ZeroPage)
            | (Instructions::LoadY, AddressingMode::Absolute) => {
                self.load_register(operation.1, "Y")
            }

            /* storing registers */
            (Instructions::StoreAccumulator, AddressingMode::Absolute)
            | (Instructions::StoreAccumulator, AddressingMode::ZeroPage)
            | (Instructions::StoreAccumulator, AddressingMode::ZeroPageX)
            | (Instructions::StoreAccumulator, AddressingMode::ZeroPageY) => {
                self.store_register(operation.1, "A")
            }

            (Instructions::StoreX, AddressingMode::Absolute)
            | (Instructions::StoreX, AddressingMode::ZeroPage)
            | (Instructions::StoreX, AddressingMode::ZeroPageY) => {
                self.store_register(operation.1, "X")
            }

            (Instructions::StoreY, AddressingMode::Absolute)
            | (Instructions::StoreY, AddressingMode::ZeroPage)
            | (Instructions::StoreY, AddressingMode::ZeroPageX) => {
                self.store_register(operation.1, "Y")
            }

            (Instructions::ShiftOneLeft, AddressingMode::Absolute) => {
                self.shift_one_left(operation.1)
            }

            // increment/decrement registers
            (Instructions::IncrementX, AddressingMode::Implied) => {
                self.increase_register(operation.1, "X")
            }
            (Instructions::IncrementY, AddressingMode::Implied) => {
                self.increase_register(operation.1, "Y")
            }

            (Instructions::DecrementX, AddressingMode::Implied) => {
                self.decrease_register(operation.1, "X")
            }
            (Instructions::DecrementY, AddressingMode::Implied) => {
                self.decrease_register(operation.1, "Y")
            }
            (Instructions::SetDecimalMode, AddressingMode::Implied) => self.set_decimal(true),
            (Instructions::ClearCarry, AddressingMode::Implied) => self.set_carry(false),
            (Instructions::SetCarry, AddressingMode::Implied) => self.set_carry(true),
            (Instructions::ClearOverflow, AddressingMode::Implied) => self.set_overflow(false),

            (Instructions::TestBitsAccumulator, AddressingMode::ZeroPage) => {
                self.test_bit(operation.1);
            }

            (Instructions::BranchOnCarrySet, AddressingMode::Relative) => {
                self.branch(operation.1, self.reg.flags.carry)
            }
            (Instructions::BranchOnCarryClear, AddressingMode::Relative) => {
                self.branch(operation.1, !self.reg.flags.carry)
            }

            (Instructions::BranchOnResultZero, AddressingMode::Relative) => {
                self.branch(operation.1, self.reg.flags.zero)
            }
            (Instructions::BranchOnResultNotZero, AddressingMode::Relative) => {
                self.branch(operation.1, !self.reg.flags.zero)
            }

            (Instructions::BranchOnResultPlus, AddressingMode::Relative) => {
                self.branch(operation.1, !self.reg.flags.negative)
            }

            (Instructions::BranchOnResultMinus, AddressingMode::Relative) => {
                self.branch(operation.1, self.reg.flags.negative)
            }

            (Instructions::BranchOnOverflowSet, AddressingMode::Relative) => {
                self.branch(operation.1, self.reg.flags.overflow)
            }

            (Instructions::BranchOnOverflowClear, AddressingMode::Relative) => {
                self.branch(operation.1, !self.reg.flags.overflow)
            }

            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::AbsoluteX) => {
                self.add_mem_to_accumulator_with_carry_absolute_x()
            }

            /* bitwise */
            (Instructions::ORAccumulator, AddressingMode::XIndirect) => {
                let base_address =
                    self.memory.read_byte(self.reg.pc + 1) as u16 + self.reg.idx as u16;
                let operand = self.memory.read_byte(base_address);
                self.reg.accumulator |= operand;
                self.reg.pc += operation.1.get_increment();
                println!("ORAccumulator XIndirect: 0x{:x}", self.reg.sp); //tmp
            }

            (Instructions::MoveXToStackPointer, AddressingMode::Implied) => {
                self.reg.sp = self.reg.idx;
                println!("Stored X in SP: 0x{:x}", self.reg.sp);
                self.reg.pc += operation.1.get_increment();
            }

            (Instructions::RotateOneLeft, AddressingMode::Absolute) => {
                let address = self.memory.read_word(self.reg.pc + 1);
                let value = self.memory.read_byte(address);
                self.memory.write_byte(address, value.rotate_left(1));
                self.reg.pc += operation.1.get_increment();
            }

            (Instructions::ISC, AddressingMode::Absolute) => self.isc_abs(),

            (Instructions::PushProcessorStatusOnStack, AddressingMode::Implied) => {
                self.push_stack(self.reg.flags.as_byte());
                self.reg.pc += operation.1.get_increment();
                println!(
                    "ProcessorStatus: PUSH SP {1} 0x{:x}",
                    self.reg.sp,
                    self.reg.flags.as_byte()
                );
            }
            (Instructions::PullProcessorStatusFromStack, AddressingMode::Implied) => {
                let status = self.pop_stack();
                self.reg.flags.set_byte(status);
                println!(
                    "ProcessorStatus: POP SP {1} 0x{:x}",
                    self.reg.sp,
                    self.reg.flags.as_byte()
                );
            }

            // todo
            (Instructions::TransferAccumulatorToY, AddressingMode::Implied) => {
                self.reg.idy = self.reg.accumulator;
                println!("Transfered A -> Y {}", self.reg.idy);
                self.reg.pc += operation.1.get_increment();
            }

            // todo
            (Instructions::TransferAccumulatorToX, AddressingMode::Implied) => {
                self.reg.idx = self.reg.accumulator;
                println!("Transfered A -> X {}", self.reg.idx);
                self.reg.pc += operation.1.get_increment();
            }

            (Instructions::MissingOperation, AddressingMode::Implied) => {
                panic!("Missing operation??")
            }

            (Instructions::EORAccumulator, AddressingMode::XIndirect) => {
                self.eor_accumulator_xindirect();
            }

            (Instructions::NoOperation, _) => {
                println!("NOOP!");
                self.reg.pc += operation.1.get_increment()
            }

            (Instructions::ForceBreak, AddressingMode::Implied) => self.breakpoint(),
            (Instructions::JAM, AddressingMode::Implied) => {
                // self.breakpoint()
                println!("JAM... Writing memory dump.");
                self.memory
                    .dump_to_file("JAMMED.bin")
                    .expect("Error while writing to dump file");
                exit(1);
            }

            (_, _) => {
                println!(
                    "Unknown pattern! {:?}, {:?} PC: {:x}",
                    operation.0, operation.1, self.reg.pc
                );
                self.memory
                    .dump_to_file("UNKNOWN.bin")
                    .expect("Error while writing to dump file");
                exit(1);
                self.reg.pc += operation.1.get_increment();
            }
        }
    }

    fn eor_accumulator_xindirect(&mut self) {
        let address = self.memory.read_byte(self.reg.pc + 1);
        // Calculate the effective address using XIndirect addressing mode
        let effective_address = address.wrapping_add(self.reg.idx);

        // Fetch the value from memory at the effective address
        let value = self.memory.read_byte(effective_address as u16);

        // Perform EOR operation
        self.reg.accumulator ^= value;

        // Update flags (N and Z)
        self.reg.flags.zero = self.reg.accumulator == 0;
        self.reg.flags.negative = (self.reg.accumulator & 0x80) != 0;

        self.reg.pc += 2;
    }

    pub fn set_pc(&mut self, addr: u16) {
        self.reg.pc = addr;
    }

    fn isc_abs(&mut self) {
        let address = self.memory.read_word(self.reg.pc + 1);
        // Step 1: Increment memory value
        let operand = self.memory.read_byte(address);
        let incremented_value = operand.wrapping_add(1);
        self.memory.write_byte(address, incremented_value);

        // Step 2: Subtract with carry
        let borrow = if self.reg.flags.carry { 0 } else { 1 };
        let result = self
            .reg
            .accumulator
            .wrapping_sub(incremented_value)
            .wrapping_sub(borrow);

        // Update flags
        self.reg.flags.zero = result == 0;
        self.reg.flags.negative = (result & 0x80) != 0;
        self.reg.flags.overflow = ((self.reg.accumulator ^ incremented_value) & 0x80 != 0)
            && ((self.reg.accumulator ^ result) & 0x80 != 0);
        self.reg.flags.carry = result <= self.reg.accumulator; // Check if there is a borrow
        self.reg.accumulator = result;

        self.reg.pc += 3;
    }

    pub fn fetch_decode_next(&mut self) {
        if self.reg.pc >= ADDR_HI {
            eprintln!();
            panic!("PC counter too high! {}", self.reg.pc)
        }
        let next_instruction = self.memory.read_byte(self.reg.pc);
        let (instruction, addressing_mode) = Self::decode(next_instruction);
        dbg!(&instruction, &addressing_mode);
        // increment pc for each instruction based on instruction type

        // dbg!(&instruction);

        self.execute((&instruction, &addressing_mode));
    }

    // TODO - works with mapper 0 only
    pub fn load_rom(&mut self, rom: &NesRom) {
        self.memory.write_bytes(0x8000, &rom.prg_rom[0]);
        if rom.prg_rom.len() > 1 {
            self.memory.write_bytes(0xC000, &rom.prg_rom[1]);
        } else {
            self.memory.write_bytes(0xC000, &rom.prg_rom[0]);
        }

        // self.set_pc(0x8000);
        self.set_pc(0xC000);
    }

    pub fn load_bytes(&mut self, data: &[u8]) {
        self.memory.write_bytes(0x8000, data);

        self.set_pc(0x8000);
        // self.set_pc(0xC000);
    }

    // 0x00
    fn breakpoint(&mut self) {
        // Create a new instance of stdin
        let stdin = io::stdin();
        // add PC
        println!("BREAKPOINT: 0x{:X}", self.reg.pc);

        // Buffer to hold the input
        let mut input = String::new();

        // Wait for user input
        stdin.read_line(&mut input).expect("Failed to read line");
        self.reg.pc += 1;
    }

    fn compare_register(&mut self, addressing_mode: &AddressingMode, register_name: &str) {
        let value = match addressing_mode {
            AddressingMode::Immediate => self.memory.read_byte(self.reg.pc + 1),
            AddressingMode::AbsoluteY => {
                let address = self.memory.read_word(self.reg.pc + 1);
                self.memory.read_byte(address + self.reg.idy as u16)
            }
            _ => {
                panic!(
                    "Unimplemented! Compare register {:?} {:?}",
                    register_name, addressing_mode
                )
            }
        };

        let result = match register_name {
            "A" => self.reg.accumulator.wrapping_sub(value),
            "X" => self.reg.idx.wrapping_sub(value),
            "Y" => self.reg.idy.wrapping_sub(value),
            _ => {
                panic!(
                    "Unimplemented! Compare register {:?} {:?}",
                    register_name, addressing_mode
                )
            }
        };

        // Update status flags based on the result
        self.reg.flags.zero = result == 0;
        // self.reg.flags.negative = (result & 0x80) != 0;
        self.reg.flags.carry = self.reg.accumulator >= value;

        self.reg.pc += addressing_mode.get_increment();
    }

    fn branch(&mut self, addressing_mode: &AddressingMode, condition: bool) {
        if condition {
            self.reg.pc = match addressing_mode {
                AddressingMode::Relative => {
                    let value = self.memory.read_byte(self.reg.pc + 1);
                    self.reg.pc + value as u16
                }
                _ => panic!("Unimplemented! Branch: {:?}", addressing_mode),
            };
            println!("Branching to addr: 0x{:x}", self.reg.pc);
        } else {
            self.reg.pc += addressing_mode.get_increment();
        }
        dbg!(&self.reg.flags);
    }

    // jump to address
    fn jump(&mut self, address: u16) {
        self.set_pc(address);
        println!("Jumped! {:x}", self.reg.pc);
    }
}

// set interrupt disable status
// fn sei(cpu: &mut NesCpu) {
//     cpu.reg.flags = CPU.reg.flags | CPUFlags::InterruptDisabled;
// }

impl NesCpu {
    // encode back to binary
    pub fn encode(operation: (Instructions, AddressingMode)) -> u8 {
        match operation {
            (Instructions::SetInterruptDisable, AddressingMode::Implied) => 0x78,
            (Instructions::ClearDecimalMode, AddressingMode::Implied) => 0xD8,
            (Instructions::LoadAccumulator, AddressingMode::Immediate) => 0xA9,
            (Instructions::StoreAccumulator, AddressingMode::Absolute) => 0x8D,
            (Instructions::LoadX, AddressingMode::Immediate) => 0xA2,
            (Instructions::MoveXToStackPointer, AddressingMode::Implied) => 0x9A,
            (Instructions::LoadAccumulator, AddressingMode::Absolute) => 0xAD,
            (Instructions::BranchOnResultPlus, AddressingMode::Relative) => 0x10,
            (Instructions::LoadY, AddressingMode::Immediate) => 0xA0,
            (Instructions::LoadAccumulator, AddressingMode::AbsoluteX) => 0xBD,
            (Instructions::CompareAccumulator, AddressingMode::Immediate) => 0xC9,
            (Instructions::BranchOnCarrySet, AddressingMode::Relative) => 0xB0,
            (Instructions::DecrementX, AddressingMode::Implied) => 0xCA,
            (Instructions::DecrementY, AddressingMode::Implied) => 0x88,
            (Instructions::BranchOnResultNotZero, AddressingMode::Relative) => 0xD0,
            (Instructions::JumpSubroutine, AddressingMode::Absolute) => 0x20,
            (Instructions::IncrementMem, AddressingMode::Absolute) => 0xEE,
            (Instructions::ORAccumulator, AddressingMode::Immediate) => 0x09,
            (Instructions::Jump, AddressingMode::Absolute) => 0x4C,
            (Instructions::Jump, AddressingMode::Indirect) => 0x6C,
            (Instructions::ORAccumulator, AddressingMode::XIndirect) => 0x01,
            (Instructions::ORAccumulator, AddressingMode::YIndirect) => 0x11,
            (Instructions::IncrementY, AddressingMode::Implied) => 0xC8,
            (Instructions::CompareX, AddressingMode::Absolute) => 0xEC,
            (Instructions::EORAccumulator, AddressingMode::XIndirect) => 0x41,
            (Instructions::PullAccumulatorFromStack, AddressingMode::Implied) => 0x68,

            (Instructions::DecrementMem, AddressingMode::AbsoluteX) => 0xDE,
            (Instructions::StoreX, AddressingMode::Absolute) => 0x8E,
            (Instructions::StoreY, AddressingMode::Absolute) => 0x8C,
            (Instructions::ANDAccumulator, AddressingMode::Immediate) => 0x29,
            (Instructions::LoadY, AddressingMode::Absolute) => 0xAC,
            (Instructions::LoadX, AddressingMode::Absolute) => 0xAE,
            (Instructions::StoreAccumulator, AddressingMode::ZeroPage) => 0x85,
            (Instructions::CompareX, AddressingMode::Immediate) => 0xE0,
            (Instructions::LoadX, AddressingMode::AbsoluteY) => 0xBE,
            (Instructions::StoreAccumulator, AddressingMode::AbsoluteX) => 0x9D,
            (Instructions::ShiftOneRight, AddressingMode::Accumulator) => 0x4A,
            (Instructions::BranchOnResultZero, AddressingMode::Relative) => 0xF0,
            (Instructions::DecrementMem, AddressingMode::ZeroPage) => 0xC6,
            (Instructions::DecrementMem, AddressingMode::Absolute) => 0xCE,
            (Instructions::IncrementMem, AddressingMode::ZeroPage) => 0xE6,
            (Instructions::IncrementMem, AddressingMode::ZeroPageX) => 0xF6,
            (Instructions::EORAccumulator, AddressingMode::ZeroPage) => 0x45,
            (Instructions::ClearCarry, AddressingMode::Implied) => 0x18,
            (Instructions::SetCarry, AddressingMode::Implied) => 0x38,
            (Instructions::RotateOneRight, AddressingMode::AbsoluteX) => 0x7E,
            (Instructions::IncrementX, AddressingMode::Implied) => 0xE8,
            (Instructions::PushAccumulatorOnStack, AddressingMode::Implied) => 0x48,
            (Instructions::ReturnFromInterrupt, AddressingMode::Implied) => 0x40,
            (Instructions::ReturnFromSubroutine, AddressingMode::Implied) => 0x60,
            (Instructions::TransferAccumulatorToY, AddressingMode::Implied) => 0xA8,
            (Instructions::StoreY, AddressingMode::ZeroPage) => 0x84,
            (Instructions::EORAccumulator, AddressingMode::Immediate) => 0x49,
            (Instructions::CompareAccumulator, AddressingMode::ZeroPage) => 0xC5,
            (Instructions::BranchOnCarryClear, AddressingMode::Relative) => 0x90,
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::AbsoluteY) => 0x79,
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::ZeroPage) => 0x65,
            (Instructions::LoadAccumulator, AddressingMode::AbsoluteY) => 0xB9,
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::Immediate) => 0x69,
            (Instructions::ANDAccumulator, AddressingMode::YIndirect) => 0x31,
            (Instructions::TestBitsAccumulator, AddressingMode::Absolute) => 0x2C,
            (Instructions::TestBitsAccumulator, AddressingMode::ZeroPage) => 0x24,
            (Instructions::StoreAccumulator, AddressingMode::AbsoluteY) => 0x99,
            (Instructions::ORAccumulator, AddressingMode::Absolute) => 0x0D,
            (Instructions::CompareY, AddressingMode::Immediate) => 0xC0,
            (Instructions::TransferXToAccumulator, AddressingMode::Immediate) => 0x8A,
            (Instructions::BranchOnResultMinus, AddressingMode::Relative) => 0x30,
            (Instructions::LoadAccumulator, AddressingMode::ZeroPage) => 0xA5,
            (Instructions::ShiftOneLeft, AddressingMode::Accumulator) => 0x0A,
            (Instructions::StoreAccumulator, AddressingMode::XIndirect) => 0x81,
            (Instructions::CompareAccumulator, AddressingMode::XIndirect) => 0xC1,
            (Instructions::ORAccumulator, AddressingMode::ZeroPage) => 0x05,
            (Instructions::PullProcessorStatusFromStack, AddressingMode::Implied) => 0x28,
            (Instructions::StoreX, AddressingMode::ZeroPage) => 0x86,
            (Instructions::LoadY, AddressingMode::ZeroPageX) => 0xB4,
            (Instructions::TransferYToAccumulator, AddressingMode::Implied) => 0x98,
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::Immediate) => 0xE9,
            (Instructions::SetDecimalMode, AddressingMode::Implied) => 0xF8,
            (Instructions::BranchOnOverflowClear, AddressingMode::Relative) => 0x50,
            (Instructions::IncrementMem, AddressingMode::AbsoluteX) => 0xFE,
            (Instructions::TransferAccumulatorToX, AddressingMode::Implied) => 0xAA,
            (Instructions::LoadY, AddressingMode::AbsoluteX) => 0xBC,
            (Instructions::LoadX, AddressingMode::ZeroPage) => 0xA6,
            (Instructions::LoadAccumulator, AddressingMode::ZeroPageX) => 0xB5,
            (Instructions::ORAccumulator, AddressingMode::AbsoluteY) => 0x19,
            (Instructions::BranchOnOverflowSet, AddressingMode::Relative) => 0x70,
            (Instructions::ShiftOneLeft, AddressingMode::ZeroPageX) => 0x16,
            (Instructions::StoreAccumulator, AddressingMode::YIndirect) => 0x91,
            (Instructions::ORAccumulator, AddressingMode::ZeroPageX) => 0x15,
            (Instructions::ORAccumulator, AddressingMode::AbsoluteX) => 0x1D,
            (Instructions::ShiftOneLeft, AddressingMode::Absolute) => 0x0E,
            (Instructions::RotateOneLeft, AddressingMode::Absolute) => 0x2E,
            (Instructions::ANDAccumulator, AddressingMode::XIndirect) => 0x21,
            (Instructions::CompareAccumulator, AddressingMode::Absolute) => 0xCD,
            (Instructions::ANDAccumulator, AddressingMode::ZeroPage) => 0x25,
            (Instructions::RotateOneLeft, AddressingMode::ZeroPage) => 0x26,
            (Instructions::RotateOneLeft, AddressingMode::ZeroPageX) => 0x36,
            (Instructions::ShiftOneRight, AddressingMode::ZeroPage) => 0x46,
            (Instructions::ShiftOneRight, AddressingMode::ZeroPageX) => 0x56,
            (Instructions::ANDAccumulator, AddressingMode::Absolute) => 0x2D,
            (Instructions::ANDAccumulator, AddressingMode::AbsoluteX) => 0x3D,
            (Instructions::ANDAccumulator, AddressingMode::AbsoluteY) => 0x39,
            (Instructions::PushProcessorStatusOnStack, AddressingMode::Implied) => 0x08,
            (Instructions::EORAccumulator, AddressingMode::AbsoluteX) => 0x5D,
            (Instructions::EORAccumulator, AddressingMode::AbsoluteY) => 0x59,
            (Instructions::RotateOneRight, AddressingMode::Absolute) => 0x6E,
            (Instructions::RotateOneLeft, AddressingMode::Accumulator) => 0x2A,
            (Instructions::ShiftOneLeft, AddressingMode::ZeroPage) => 0x06,
            (Instructions::LoadAccumulator, AddressingMode::XIndirect) => 0xA1,
            (Instructions::LoadAccumulator, AddressingMode::YIndirect) => 0xB1,
            (Instructions::LoadY, AddressingMode::ZeroPage) => 0xA4,
            (Instructions::ShiftOneRight, AddressingMode::Absolute) => 0x4E,
            (Instructions::ANDAccumulator, AddressingMode::ZeroPageX) => 0x35,
            (Instructions::TransferStackPointerToX, AddressingMode::Implied) => 0xBA,
            (Instructions::RotateOneRight, AddressingMode::ZeroPage) => 0x66,
            (Instructions::RotateOneRight, AddressingMode::Accumulator) => 0x6A,
            (Instructions::EORAccumulator, AddressingMode::Absolute) => 0x4D,
            (Instructions::EORAccumulator, AddressingMode::YIndirect) => 0x51,
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::Absolute) => 0x6D,
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::XIndirect) => 0x61,
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::YIndirect) => 0x71,
            (Instructions::RotateOneRight, AddressingMode::ZeroPageX) => 0x76,
            (Instructions::LoadX, AddressingMode::ZeroPageY) => 0xB6,
            (Instructions::ShiftOneRight, AddressingMode::AbsoluteX) => 0x5E,
            (Instructions::CompareY, AddressingMode::Absolute) => 0xCC,
            (Instructions::ClearInterruptDisable, AddressingMode::Implied) => 0x58,
            (Instructions::ShiftOneLeft, AddressingMode::AbsoluteX) => 0x1E,
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::AbsoluteY) => 0xF9,
            (Instructions::EORAccumulator, AddressingMode::ZeroPageX) => 0x55,
            (Instructions::CompareAccumulator, AddressingMode::YIndirect) => 0xD1,
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::AbsoluteX) => 0xFD,
            (Instructions::StoreAccumulator, AddressingMode::ZeroPageX) => 0x95,
            (Instructions::CompareAccumulator, AddressingMode::AbsoluteY) => 0xD9,
            (Instructions::StoreX, AddressingMode::ZeroPageY) => 0x96,
            (Instructions::StoreY, AddressingMode::ZeroPageX) => 0x94,
            (Instructions::CompareAccumulator, AddressingMode::AbsoluteX) => 0xDD,
            (Instructions::ClearOverflow, AddressingMode::Implied) => 0xB8,
            (Instructions::DecrementMem, AddressingMode::ZeroPageX) => 0xD6,
            (Instructions::CompareY, AddressingMode::ZeroPage) => 0xC4,
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::AbsoluteX) => 0x7D,
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::ZeroPageX) => 0x75,
            (Instructions::CompareX, AddressingMode::ZeroPage) => 0xE4,
            (Instructions::CompareAccumulator, AddressingMode::ZeroPageX) => 0xD5,
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::Absolute) => 0xED,
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::ZeroPage) => 0xE5,
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::ZeroPageX) => 0xF5,
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::XIndirect) => 0xE1,
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::YIndirect) => 0xF1,
            (Instructions::RotateOneLeft, AddressingMode::AbsoluteX) => 0x3E,

            // illegal -- TODO names for these
            (Instructions::SLO, AddressingMode::XIndirect) => 0x03,
            (Instructions::SLO, AddressingMode::ZeroPage) => 0x07,
            (Instructions::SLO, AddressingMode::YIndirect) => 0x13,
            (Instructions::SLO, AddressingMode::ZeroPageX) => 0x17,
            (Instructions::SLO, AddressingMode::AbsoluteX) => 0x1F,
            (Instructions::SLO, AddressingMode::Absolute) => 0x0F,
            (Instructions::SLO, AddressingMode::AbsoluteY) => 0x1B,
            (Instructions::ISC, AddressingMode::ZeroPage) => 0xE7,
            (Instructions::ISC, AddressingMode::YIndirect) => 0xF3,
            (Instructions::ISC, AddressingMode::XIndirect) => 0xE3,
            (Instructions::ISC, AddressingMode::Absolute) => 0xEF,
            (Instructions::ISC, AddressingMode::AbsoluteY) => 0xFB,
            (Instructions::ISC, AddressingMode::AbsoluteX) => 0xFF,
            (Instructions::ISC, AddressingMode::ZeroPageX) => 0xF7,
            (Instructions::RLA, AddressingMode::ZeroPage) => 0x27,
            (Instructions::RLA, AddressingMode::XIndirect) => 0x23,
            (Instructions::RLA, AddressingMode::ZeroPage) => 0x37,
            (Instructions::RLA, AddressingMode::Absolute) => 0x2F,
            (Instructions::RLA, AddressingMode::AbsoluteY) => 0x3B,
            (Instructions::RLA, AddressingMode::YIndirect) => 0x33,
            (Instructions::RLA, AddressingMode::AbsoluteX) => 0x3F,
            (Instructions::RRA, AddressingMode::ZeroPage) => 0x67,
            (Instructions::RRA, AddressingMode::XIndirect) => 0x63,
            (Instructions::RRA, AddressingMode::AbsoluteX) => 0x7F,
            (Instructions::RRA, AddressingMode::AbsoluteY) => 0x7B,
            (Instructions::RRA, AddressingMode::YIndirect) => 0x73,
            (Instructions::RRA, AddressingMode::Absolute) => 0x6F,
            (Instructions::RRA, AddressingMode::ZeroPageX) => 0x77,
            (Instructions::LAX, AddressingMode::XIndirect) => 0xA3,
            (Instructions::LAX, AddressingMode::ZeroPage) => 0xA7,
            (Instructions::LAX, AddressingMode::Absolute) => 0xAF,
            (Instructions::LAX, AddressingMode::AbsoluteY) => 0xBF,
            (Instructions::LAX, AddressingMode::ZeroPageY) => 0xB7,
            (Instructions::LAX, AddressingMode::YIndirect) => 0xB3,
            (Instructions::SRE, AddressingMode::AbsoluteY) => 0x5B,
            (Instructions::SRE, AddressingMode::XIndirect) => 0x43,
            (Instructions::SRE, AddressingMode::YIndirect) => 0x53,
            (Instructions::SRE, AddressingMode::AbsoluteX) => 0x5F,
            (Instructions::SRE, AddressingMode::Absolute) => 0x4F,
            (Instructions::SRE, AddressingMode::ZeroPage) => 0x47,
            (Instructions::SRE, AddressingMode::ZeroPageX) => 0x57,
            (Instructions::DCP, AddressingMode::XIndirect) => 0xC3,
            (Instructions::DCP, AddressingMode::YIndirect) => 0xD3,
            (Instructions::DCP, AddressingMode::AbsoluteY) => 0xDB,
            (Instructions::DCP, AddressingMode::ZeroPage) => 0xC7,
            (Instructions::DCP, AddressingMode::ZeroPageX) => 0xD7,
            (Instructions::DCP, AddressingMode::AbsoluteX) => 0xDF,
            (Instructions::DCP, AddressingMode::Absolute) => 0xCF,
            (Instructions::USBC, AddressingMode::Immediate) => 0xEB,
            (Instructions::SAX, AddressingMode::ZeroPageY) => 0x97,
            (Instructions::SAX, AddressingMode::Absolute) => 0x8F,
            (Instructions::SAX, AddressingMode::XIndirect) => 0x83,
            (Instructions::ARR, AddressingMode::Immediate) => 0x6B,
            (Instructions::ALR, AddressingMode::Immediate) => 0x4B,
            (Instructions::SHX, AddressingMode::AbsoluteY) => 0x9E,
            (Instructions::SHY, AddressingMode::AbsoluteX) => 0x9C,
            (Instructions::SHA, AddressingMode::AbsoluteY) => 0x9F,
            (Instructions::SHA, AddressingMode::YIndirect) => 0x93,
            (Instructions::ANC, AddressingMode::Immediate) => 0x2B, // effectively the same as 0x0B
            (Instructions::ANC, AddressingMode::Immediate) => 0x0B,
            (Instructions::ANE, AddressingMode::Immediate) => 0x8B,
            (Instructions::SAX, AddressingMode::ZeroPage) => 0x87,
            (Instructions::TAS, AddressingMode::AbsoluteY) => 0x9B,
            (Instructions::LAS, AddressingMode::AbsoluteY) => 0xBB,
            (Instructions::LXA, AddressingMode::Immediate) => 0xAB,
            (Instructions::SBX, AddressingMode::Immediate) => 0xCB,

            // noop
            (Instructions::NoOperation, AddressingMode::Implied) => 0x1A,

            (Instructions::NoOperation, AddressingMode::ZeroPage) => 0x04,

            (Instructions::NoOperation, AddressingMode::Immediate) => 0x80,

            (Instructions::NoOperation, AddressingMode::ZeroPageX) => 0x14,

            (Instructions::NoOperation, AddressingMode::Absolute) => 0x0C,
            (Instructions::NoOperation, AddressingMode::AbsoluteX) => 0x1C,

            // jam
            (Instructions::JAM, AddressingMode::Implied) => 0x02,

            // software breakpoint
            (Instructions::ForceBreak, AddressingMode::Implied) => 0x00,
            _ => 0x02,
        }
    }
}
