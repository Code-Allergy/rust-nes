use crate::cpu::{NesCpu, Processor};
use crate::memory::Bus;
use std::fmt::{Display, Formatter};
use std::process::exit;

#[derive(Debug, Eq, PartialEq, Clone)]
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

// todo reduce length of some entries
#[derive(Debug, Eq, PartialEq, Clone)]
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

    // illegal -- need to add nicer names
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

#[derive(Debug, Clone)]
pub struct CurrentInstruction {
    pub(crate) op: Instructions,
    pub(crate) mode: AddressingMode,
}
impl CurrentInstruction {
    pub(crate) fn new() -> Self {
        Self {
            op: Instructions::JAM,
            mode: AddressingMode::Implied,
        }
    }
}

impl Display for CurrentInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.op, self.mode)
    }
}

impl AddressingMode {
    pub(crate) fn get_increment(&self) -> u16 {
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

impl Processor for NesCpu {
    fn decode_instruction(opcode: u8) -> (Instructions, AddressingMode) {
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
                AddressingMode::Implied,
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

    fn encode_instructions(instruction: Instructions, addressing_mode: AddressingMode) -> u8 {
        match (instruction, addressing_mode) {
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

    // fn execute_instruction(&mut self) {
    //     // temporary -- TODO find a solution to this, shouldn't need to clone shit each instruction
    //     let operation = (&self.current.op.clone(), &self.current.mode.clone());
    //
    //     match operation {
    //         (Instructions::Jump, AddressingMode::Absolute) => self.jump(self.next_word()),
    //         (Instructions::Jump, AddressingMode::Indirect) => {
    //             let address = self.next_word();
    //             self.jump(self.memory.read_word(address));
    //         }
    //
    //         // JSR
    //         (Instructions::JumpSubroutine, AddressingMode::Absolute) => {
    //             self.push_stack_u16(self.memory.read_word(self.reg.pc + 3));
    //             self.jump(self.next_word());
    //         }
    //         (Instructions::ReturnFromSubroutine, AddressingMode::Implied) => {
    //             let addr = self.pop_stack_u16();
    //             self.jump(addr);
    //         }
    //
    //         // conditional branching
    //         (Instructions::BranchOnResultPlus, AddressingMode::Relative)
    //         | (Instructions::BranchOnResultMinus, AddressingMode::Relative)
    //         | (Instructions::BranchOnResultZero, AddressingMode::Relative)
    //         | (Instructions::BranchOnResultNotZero, AddressingMode::Relative)
    //         | (Instructions::BranchOnOverflowSet, AddressingMode::Relative)
    //         | (Instructions::BranchOnOverflowClear, AddressingMode::Relative)
    //         | (Instructions::BranchOnCarrySet, AddressingMode::Relative)
    //         | (Instructions::BranchOnCarryClear, AddressingMode::Relative) => self.branch(),
    //
    //         (Instructions::CompareAccumulator, AddressingMode::Immediate)
    //         | (Instructions::CompareX, AddressingMode::Immediate)
    //         | (Instructions::CompareY, AddressingMode::Immediate)
    //         | (Instructions::CompareAccumulator, AddressingMode::AbsoluteY) => {
    //             self.compare_register();
    //         }
    //
    //         /* storing registers */
    //         (Instructions::StoreAccumulator, AddressingMode::Absolute)
    //         | (Instructions::StoreAccumulator, AddressingMode::AbsoluteX)
    //         | (Instructions::StoreAccumulator, AddressingMode::AbsoluteY)
    //         | (Instructions::StoreAccumulator, AddressingMode::ZeroPage)
    //         | (Instructions::StoreAccumulator, AddressingMode::ZeroPageX)
    //         | (Instructions::StoreAccumulator, AddressingMode::ZeroPageY)
    //         | (Instructions::StoreAccumulator, AddressingMode::YIndirect)
    //         | (Instructions::StoreX, AddressingMode::Absolute)
    //         | (Instructions::StoreX, AddressingMode::ZeroPage)
    //         | (Instructions::StoreX, AddressingMode::ZeroPageY)
    //         | (Instructions::StoreY, AddressingMode::Absolute)
    //         | (Instructions::StoreY, AddressingMode::ZeroPage)
    //         | (Instructions::StoreY, AddressingMode::ZeroPageX) => self.store_register(),
    //
    //         /* load registers */
    //         (Instructions::LoadAccumulator, AddressingMode::Immediate)
    //         | (Instructions::LoadAccumulator, AddressingMode::Absolute)
    //         | (Instructions::LoadAccumulator, AddressingMode::AbsoluteX)
    //         | (Instructions::LoadAccumulator, AddressingMode::ZeroPage)
    //         | (Instructions::LoadAccumulator, AddressingMode::ZeroPageX)
    //         | (Instructions::LoadAccumulator, AddressingMode::ZeroPageY)
    //         | (Instructions::LoadX, AddressingMode::Immediate)
    //         | (Instructions::LoadX, AddressingMode::ZeroPage)
    //         | (Instructions::LoadX, AddressingMode::Absolute)
    //         | (Instructions::LoadY, AddressingMode::Immediate)
    //         | (Instructions::LoadY, AddressingMode::ZeroPage)
    //         | (Instructions::LoadY, AddressingMode::Absolute) => {
    //             self.load_register();
    //         }
    //
    //         (Instructions::PushAccumulatorOnStack, AddressingMode::Implied) => {
    //             self.push_stack(self.reg.accumulator);
    //             self.next();
    //         }
    //
    //         (Instructions::PullAccumulatorFromStack, AddressingMode::Implied) => {
    //             self.reg.accumulator = self.pop_stack();
    //             self.reg.pc += operation.1.get_increment()
    //         }
    //
    //         (Instructions::ShiftOneLeft, AddressingMode::Absolute) => {
    //             self.shift_one_left(operation.1)
    //         }
    //
    //         // increment/decrement registers
    //         (Instructions::IncrementX, AddressingMode::Implied)
    //         | (Instructions::IncrementY, AddressingMode::Implied) => self.increase_register(),
    //         (Instructions::DecrementX, AddressingMode::Implied)
    //         | (Instructions::DecrementY, AddressingMode::Implied) => self.decrease_register(),
    //
    //         // increase/decrement memory
    //         (Instructions::IncrementMem, AddressingMode::Absolute) => self.increment_mem(),
    //         (Instructions::DecrementMem, AddressingMode::Absolute) => self.decrement_mem(),
    //
    //         // TODO
    //         (Instructions::SetDecimalMode, AddressingMode::Implied) => self.set_decimal(true),
    //         (Instructions::ClearCarry, AddressingMode::Implied) => self.set_carry(false),
    //         (Instructions::SetCarry, AddressingMode::Implied) => self.set_carry(true),
    //         (Instructions::ClearOverflow, AddressingMode::Implied) => self.set_overflow(false),
    //         (Instructions::SetInterruptDisable, AddressingMode::Implied) => {
    //             self.set_interrupts_disabled(true);
    //         }
    //         (Instructions::ClearDecimalMode, AddressingMode::Implied) => {
    //             self.set_decimal(false);
    //         }
    //
    //         (Instructions::TestBitsAccumulator, AddressingMode::ZeroPage) => {
    //             self.test_bit(operation.1);
    //         }
    //
    //         /* bitwise */
    //         (Instructions::ORAccumulator, AddressingMode::XIndirect)
    //         | (Instructions::ORAccumulator, AddressingMode::YIndirect) => {
    //             self.or_accumulator();
    //         }
    //
    //         (Instructions::MoveXToStackPointer, AddressingMode::Implied) => {
    //             self.reg.sp = self.reg.idx;
    //             println!("Stored X in SP: 0x{:x}", self.reg.sp);
    //             self.next();
    //         }
    //
    //         (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::AbsoluteX) => {
    //             self.add_mem_to_accumulator_with_carry_absolute_x()
    //         }
    //
    //         (Instructions::RotateOneLeft, AddressingMode::Absolute) => {
    //             let address = self.next_word();
    //             let value = self.memory.read_byte(address);
    //             self.memory.write_byte(address, value.rotate_left(1));
    //             self.next();
    //         }
    //
    //         (Instructions::ISC, AddressingMode::Absolute) => self.isc_abs(),
    //
    //         (Instructions::PushProcessorStatusOnStack, AddressingMode::Implied) => {
    //             self.push_stack(self.reg.flags.as_byte());
    //             self.reg.pc += operation.1.get_increment();
    //             println!(
    //                 "ProcessorStatus: PUSH SP {1} 0x{:x}",
    //                 self.reg.sp,
    //                 self.reg.flags.as_byte()
    //             );
    //         }
    //         (Instructions::PullProcessorStatusFromStack, AddressingMode::Implied) => {
    //             let status = self.pop_stack();
    //             self.reg.flags.set_byte(status);
    //             println!(
    //                 "ProcessorStatus: POP SP {1} 0x{:x}",
    //                 self.reg.sp,
    //                 self.reg.flags.as_byte()
    //             );
    //         }
    //
    //         // todo
    //         (Instructions::TransferAccumulatorToY, AddressingMode::Implied) => {
    //             self.reg.idy = self.reg.accumulator;
    //             println!("Transfered A -> Y {}", self.reg.idy);
    //             self.next();
    //         }
    //
    //         // todo
    //         (Instructions::TransferAccumulatorToX, AddressingMode::Implied) => {
    //             self.reg.idx = self.reg.accumulator;
    //             println!("Transfered A -> X {}", self.reg.idx);
    //             self.next();
    //         }
    //
    //         // todo
    //         (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::Immediate) => {
    //             let operand = self.next_byte();
    //             // Perform Subtract Immediate with Carry
    //             let borrow = if self.reg.flags.carry { 1 } else { 0 };
    //             let result = self
    //                 .reg
    //                 .accumulator
    //                 .wrapping_sub(operand)
    //                 .wrapping_sub(borrow);
    //
    //             // Update CPU state
    //             self.reg.accumulator = result;
    //             self.reg.flags.carry = result <= self.reg.accumulator; // Set carry flag if no borrow
    //                                                                    // ... other flag updates and state modifications
    //         }
    //         // todo
    //         (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::YIndirect) => {
    //             // Subtract Immediate with Carry, Y-Indirect
    //             let y_register = self.reg.idy; // Assuming you have a Y register in your CPU state
    //             let indirect_address = self.next_byte().wrapping_add(y_register) as u16;
    //             let operand_value = self.memory.read_byte(indirect_address);
    //
    //             let borrow = if self.reg.flags.carry { 1 } else { 0 };
    //             let result = self
    //                 .reg
    //                 .accumulator
    //                 .wrapping_sub(operand_value)
    //                 .wrapping_sub(borrow);
    //
    //             // Update CPU state
    //             self.reg.accumulator = result;
    //             self.reg.flags.carry = result <= self.reg.accumulator; // Set carry flag if no borrow
    //                                                                    // ... other flag updates and state modifications
    //             self.next();
    //         }
    //
    //         (Instructions::EORAccumulator, AddressingMode::XIndirect) => {
    //             self.eor_accumulator_xindirect();
    //         }
    //
    //         (Instructions::MissingOperation, AddressingMode::Implied) => {
    //             panic!("Missing operation??")
    //         }
    //         (Instructions::NoOperation, _) => {
    //             println!("NO OP");
    //             self.next();
    //         }
    //
    //         (Instructions::ForceBreak, AddressingMode::Implied) => self.breakpoint(),
    //         (Instructions::JAM, AddressingMode::Implied) => {
    //             // self.breakpoint()
    //             println!("JAM... Writing memory dump.");
    //             self.memory
    //                 .dump_to_file("JAMMED.bin")
    //                 .expect("Error while writing to dump file");
    //             exit(1);
    //         }
    //
    //         (_, _) => {
    //             println!(
    //                 "Unknown pattern! {:?}, {:?} PC: {:x}",
    //                 operation.0, operation.1, self.reg.pc
    //             );
    //             self.memory
    //                 .dump_to_file("UNKNOWN.bin")
    //                 .expect("Error while writing to dump file");
    //             exit(1);
    //             self.reg.pc += operation.1.get_increment();
    //         }
    //     }
}
