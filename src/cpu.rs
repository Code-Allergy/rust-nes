use crate::instructions::{AddressingMode, CurrentInstruction, Instructions};
use crate::memory::{Bus, Memory, ADDR_HI};
use crate::NesRom;
use std::fmt::{write, Display, Formatter};
use std::io;
use std::process::exit;

pub const CLOCK_RATE: u32 = 21441960;

// https://www.nesdev.org/wiki/2A03
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
struct CPUFlags {
    carry: bool,
    zero: bool,
    interrupt_disable: bool,
    decimal: bool, // nes unused?
    overflow: bool,
    negative: bool,
}

pub trait Processor {
    fn decode_instruction(opcode: u8) -> (Instructions, AddressingMode);
    fn encode_instructions(instruction: Instructions, addressing_mode: AddressingMode) -> u8;
    // fn execute_instruction(&mut self);
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
        result |= 0b0010_0000;
        result |= 0b0001_0000; // B flag
        result |= if self.decimal { 0b0000_1000 } else { 0 };
        result |= if self.overflow { 0b0100_0000 } else { 0 };
        result |= if self.negative { 0b1000_0000 } else { 0 };

        result
    }
}

#[derive(Clone)]
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
    /// Gets the next byte after the current instruction
    pub fn next_byte(&self) -> u8 {
        self.memory.read_byte(self.reg.pc + 1)
    }

    /// Gets the next word after the current instruction
    pub fn next_word(&self) -> u16 {
        self.memory.read_word(self.reg.pc + 1)
    }

    fn set_interrupts_disabled(&mut self, status: bool) {
        self.reg.flags.interrupt_disable = status;
        println!("Interrupts Disabled: {}", status);
        self.reg.pc += 1;
    }

    fn set_decimal(&mut self, status: bool) {
        self.reg.flags.decimal = status;
        println!("Decimal bit: {}", status);
        self.next();
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

    fn push_stack_u16(&mut self, data: u16) {
        let ra_bytes = (data).to_le_bytes();
        self.push_stack(ra_bytes[1]);
        self.push_stack(ra_bytes[0]);
    }

    fn pop_stack(&mut self) -> u8 {
        if self.reg.sp == 0xFF {
            panic!("Stack pointer overflow!");
        }
        let address: u16 = 0x100 + self.reg.sp as u16;
        self.reg.sp += 1;
        let res = self.memory.read_byte(address + 1);
        println!(
            "Stack pop (pointer: 0x{1:x})! {} (0x{0:X})",
            res, self.reg.sp
        );
        res
    }

    fn pop_stack_u16(&mut self) -> u16 {
        let low = self.pop_stack();
        let hi = self.pop_stack();
        println!("HIGH: {} LOW: {}", hi, low);
        u16::from_le_bytes([low, hi])
    }

    // TODO implement this in.
    fn transfer_reg_to_a(&mut self) {
        let source_register = match self.current.op {
            Instructions::TransferXToAccumulator => self.reg.idx,
            Instructions::TransferYToAccumulator => self.reg.idy,
            _ => panic!("Invalid op for transfer_reg_to_a: {:?}", self.current.op),
        };

        self.reg.accumulator = source_register;
    }

    // togo
    fn add_mem_to_accumulator_with_carry(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Immediate => 0, // unused for immediate
            AddressingMode::AbsoluteX => {
                let address = self.memory.read_word(self.reg.pc + 1);
                address.wrapping_add(self.reg.idx as u16)
            }
            AddressingMode::AbsoluteY => {
                let address = self.memory.read_word(self.reg.pc + 1);
                address.wrapping_add(self.reg.idx as u16)
            }
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            AddressingMode::XIndirect => self.get_indirect_x(),
            AddressingMode::YIndirect => self.get_indirect_y(),
            _ => panic!(
                "add_mem_to_accumulator_with_carry unknown mode: {:?}",
                self.current.mode
            ),
        };
        // Read the value from memory at the specified address + X offset
        let operand = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            _ => self.memory.read_byte(address),
        };
        let carry_add: u8 = if self.reg.flags.carry { 1 } else { 0 };
        // Perform addition
        let (result, carry_out) = self.reg.accumulator.overflowing_add(operand + carry_add);

        // Update the carry flag
        self.reg.flags.carry = carry_out;
        dbg!(carry_out);

        // Update the overflow flag
        self.reg.flags.overflow = ((self.reg.accumulator ^ operand) & 0x80 != 0)
            && ((self.reg.accumulator ^ result) & 0x80 != 0);

        // Update the zero and negative flags
        self.reg.flags.zero = result == 0;
        self.reg.flags.negative = result & 0x80 != 0;

        // Update the accumulator with the result
        self.reg.accumulator = result;
        println!("ADDED MEM TO A, WITH CARRY {}", self.reg.accumulator);
        self.next();
    }

    fn test_bit(&mut self) {
        let address = match self.current.mode {
            AddressingMode::ZeroPage => self.next_byte() as u16,
            _ => {
                panic!("test_bit not implemented for mode {:?}", self.current.mode)
            }
        };
        let operand = self.memory.read_byte(address);
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

        self.next();
    }

    /// Move the processor program counter to the next instruction in memory.
    fn next(&mut self) {
        self.reg.pc += self.current.mode.get_increment();
    }

    /// Load a value into a register
    fn load_register(&mut self) {
        // TODO errors
        // TODO x/yIndirect
        let address = match self.current.mode {
            AddressingMode::Absolute | AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
                self.next_word()
            }
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::XIndirect => {
                let address = self.next_byte();
                let low = self
                    .memory
                    .read_byte(address.wrapping_add(self.reg.idx) as u16);
                let high = self
                    .memory
                    .read_byte(address.wrapping_add(self.reg.idx.wrapping_add(1)) as u16);
                (u16::from(high) << 8) | u16::from(low)
            }
            AddressingMode::YIndirect => {
                let address = self.memory.read_byte(self.next_byte() as u16);
                self.memory
                    .read_word(address.wrapping_add(self.reg.idy) as u16)
            }
            _ => 0,
        };

        let value = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            AddressingMode::Absolute => self.memory.read_byte(address),
            AddressingMode::AbsoluteX => self.memory.read_byte(address + self.reg.idx as u16),
            AddressingMode::AbsoluteY => self.memory.read_byte(address + self.reg.idy as u16),
            AddressingMode::ZeroPage => self.memory.read_byte(address),
            AddressingMode::ZeroPageX => self.memory.read_byte(address + self.reg.idx as u16),
            AddressingMode::ZeroPageY => self.memory.read_byte(address + self.reg.idy as u16),
            AddressingMode::XIndirect => self.memory.read_byte(address),
            AddressingMode::YIndirect => panic!("YIndirect"), //self.memory.read_byte(address), // TODO
            _ => panic!("Load Not implemented! {:?}", self.current.mode),
        };

        match self.current.op {
            Instructions::LoadAccumulator => self.reg.accumulator = value,
            Instructions::LoadX => self.reg.idx = value,
            Instructions::LoadY => self.reg.idy = value,
            _ => panic!(
                "Unknown instruction for load_register: {:?}",
                self.current.op
            ),
        }
        self.reg.flags.zero = value == 0;
        self.reg.flags.negative = value & 0x80 == 0x80;

        // TODO
        println!(
            "{:?}: {1} (0x{1:X}) {2:?}",
            self.current.op, value, &self.current.mode
        );
        self.next();
    }

    /// Store a register in memory
    fn store_register(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Absolute => self.next_word(),
            AddressingMode::AbsoluteX => self.next_word().wrapping_add(self.reg.idx as u16),
            AddressingMode::AbsoluteY => self.next_word().wrapping_add(self.reg.idy as u16),
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            AddressingMode::XIndirect => self.get_indirect_x(),
            AddressingMode::YIndirect => self.get_indirect_y(),
            _ => panic!("Store Not implemented! {:?}", self.current.mode),
        };

        let register_value = match self.current.op {
            Instructions::StoreAccumulator => self.reg.accumulator,
            Instructions::StoreX => self.reg.idx,
            Instructions::StoreY => self.reg.idy,
            _ => panic!(
                "Unknown instruction for store_register: {:?}",
                self.current.op
            ),
        };

        self.memory.write_byte(address, register_value);
        println!(
            "{:?}: Val: {2} (0x{2:X}) 0x{1:X} PC: 0x{3:x}",
            self.current.op, address, register_value, self.reg.pc
        );

        self.next();
    }

    /// Increase a register by one
    fn increase_register(&mut self) {
        let register = match self.current.op {
            Instructions::IncrementX => &mut self.reg.idx,
            Instructions::IncrementY => &mut self.reg.idy,
            _ => panic!(
                "Unknown instruction for increase_register: {:?}",
                self.current.op
            ),
        };
        *register = register.wrapping_add(1);
        if *register == 0 {
            self.reg.flags.zero = true;
            self.reg.flags.overflow = true;
        } else {
            self.reg.flags.zero = false;
        };

        println!("{:?}: Val: {1} (0x{1:x})", self.current.op, register);
        self.next();
    }

    // todo set zero bit if == 0, negative bit if negative.
    /// Decrease a register by one
    fn decrease_register(&mut self) {
        let register = match self.current.op {
            Instructions::DecrementX => &mut self.reg.idx,
            Instructions::DecrementY => &mut self.reg.idy,
            _ => panic!(
                "Unknown instruction for decrease_register: {:?}",
                self.current.op
            ),
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

        println!("{:?}: Val: {1} (0x{1:x})", self.current.op, register);
        self.next();
    }

    // todo logging message
    /// decrement mem
    fn decrement_mem(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Absolute => self.next_word(),
            _ => panic!("Invalid mode for decrement_mem {:?}", self.current.mode),
        };

        let result = self.memory.read_byte(address).wrapping_sub(1);
        self.memory.write_byte(address, result);
        self.next();
    }

    // todo logging message
    /// decrement mem
    fn increment_mem(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Absolute => self.next_word(),
            _ => panic!("Invalid mode for decrement_mem {:?}", self.current.mode),
        };

        let result = self.memory.read_byte(address).wrapping_add(1);
        self.memory.write_byte(address, result);
        self.next();
    }

    // TODO cleanup
    fn shift_one_left(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Accumulator => 0, // unused for accumulator
            AddressingMode::Absolute => self.next_word(),
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            _ => 0,
        };

        let result = match self.current.mode {
            AddressingMode::Accumulator => {
                self.reg.flags.carry = self.reg.accumulator & 0x80 == 0x80;
                self.reg.accumulator = self.reg.accumulator << 1;
                self.reg.accumulator
            }
            // TODO carry bit
            _ => {
                let value = self.memory.read_byte(address);
                self.reg.flags.carry = value & 0x80 == 0x80;
                let byte = value << 1;
                self.memory.write_byte(address, byte);
                byte
            }
        };

        self.reg.flags.zero = result == 0;
        self.reg.flags.negative = result & 0x80 == 0x80;

        self.next();
    }

    fn shift_one_right(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Accumulator => 0, // unused for accumulator
            AddressingMode::Absolute => self.next_word(),
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            _ => 0,
        };

        let result = match self.current.mode {
            AddressingMode::Accumulator => {
                self.reg.flags.carry = 0x1 & self.reg.accumulator == 0x1;
                let val = self.reg.accumulator >> 1;
                self.reg.accumulator = val;
                val
            }
            _ => {
                let value = self.memory.read_byte(address);
                self.reg.flags.carry = 0x1 & value == 0x1;
                let byte = self.memory.read_byte(address) >> 1;
                self.memory.write_byte(address, byte);
                byte
            }
        };

        self.reg.flags.zero = result == 0;
        self.reg.flags.negative = result & 0x80 == 0x80;
        // println!(
        //     "Shifting one bit right at addr: {}, old: {} new: {}",
        //     address,
        //     byte,
        //     byte << 1
        // );
        self.next();
    }

    pub fn execute(&mut self) {
        // temporary -- TODO find a solution to this, shouldn't need to clone shit each instruction
        let operation = (&self.current.op.clone(), &self.current.mode.clone());

        match operation {
            (Instructions::Jump, AddressingMode::Absolute) => self.jump(self.next_word()),
            (Instructions::Jump, AddressingMode::Indirect) => {
                let address = self.next_word();
                self.jump(self.memory.read_word(address));
            }

            // JSR
            (Instructions::JumpSubroutine, AddressingMode::Absolute) => {
                self.push_stack_u16(self.reg.pc + 2);
                self.jump(self.next_word());
            }
            (Instructions::ReturnFromSubroutine, AddressingMode::Implied) => {
                let addr = self.pop_stack_u16() + 1;
                self.jump(addr);
            }

            // conditional branching
            (Instructions::BranchOnResultPlus, AddressingMode::Relative)
            | (Instructions::BranchOnResultMinus, AddressingMode::Relative)
            | (Instructions::BranchOnResultZero, AddressingMode::Relative)
            | (Instructions::BranchOnResultNotZero, AddressingMode::Relative)
            | (Instructions::BranchOnOverflowSet, AddressingMode::Relative)
            | (Instructions::BranchOnOverflowClear, AddressingMode::Relative)
            | (Instructions::BranchOnCarrySet, AddressingMode::Relative)
            | (Instructions::BranchOnCarryClear, AddressingMode::Relative) => self.branch(),

            (Instructions::CompareAccumulator, AddressingMode::Immediate)
            | (Instructions::CompareAccumulator, AddressingMode::AbsoluteY)
            | (Instructions::CompareAccumulator, AddressingMode::XIndirect)
            | (Instructions::CompareAccumulator, AddressingMode::YIndirect)
            | (Instructions::CompareAccumulator, AddressingMode::ZeroPage)
            | (Instructions::CompareAccumulator, AddressingMode::ZeroPageX)
            | (Instructions::CompareAccumulator, AddressingMode::ZeroPageY)
            | (Instructions::CompareX, AddressingMode::Immediate)
            | (Instructions::CompareX, AddressingMode::ZeroPage)
            | (Instructions::CompareX, AddressingMode::ZeroPageX)
            | (Instructions::CompareX, AddressingMode::ZeroPageY)
            | (Instructions::CompareY, AddressingMode::Immediate)
            | (Instructions::CompareY, AddressingMode::ZeroPage)
            | (Instructions::CompareY, AddressingMode::ZeroPageX)
            | (Instructions::CompareY, AddressingMode::ZeroPageY) => {
                self.compare_register();
            }

            /* storing registers */
            (Instructions::StoreAccumulator, AddressingMode::Absolute)
            | (Instructions::StoreAccumulator, AddressingMode::AbsoluteX)
            | (Instructions::StoreAccumulator, AddressingMode::AbsoluteY)
            | (Instructions::StoreAccumulator, AddressingMode::ZeroPage)
            | (Instructions::StoreAccumulator, AddressingMode::ZeroPageX)
            | (Instructions::StoreAccumulator, AddressingMode::ZeroPageY)
            | (Instructions::StoreAccumulator, AddressingMode::XIndirect)
            | (Instructions::StoreAccumulator, AddressingMode::YIndirect)
            | (Instructions::StoreX, AddressingMode::Absolute)
            | (Instructions::StoreX, AddressingMode::ZeroPage)
            | (Instructions::StoreX, AddressingMode::ZeroPageY)
            | (Instructions::StoreY, AddressingMode::Absolute)
            | (Instructions::StoreY, AddressingMode::ZeroPage)
            | (Instructions::StoreY, AddressingMode::ZeroPageX) => self.store_register(),

            /* load registers */
            (Instructions::LoadAccumulator, AddressingMode::Immediate)
            | (Instructions::LoadAccumulator, AddressingMode::Absolute)
            | (Instructions::LoadAccumulator, AddressingMode::AbsoluteX)
            | (Instructions::LoadAccumulator, AddressingMode::ZeroPage)
            | (Instructions::LoadAccumulator, AddressingMode::ZeroPageX)
            | (Instructions::LoadAccumulator, AddressingMode::ZeroPageY)
            | (Instructions::LoadAccumulator, AddressingMode::XIndirect)
            | (Instructions::LoadAccumulator, AddressingMode::YIndirect)
            | (Instructions::LoadX, AddressingMode::Immediate)
            | (Instructions::LoadX, AddressingMode::ZeroPage)
            | (Instructions::LoadX, AddressingMode::Absolute)
            | (Instructions::LoadY, AddressingMode::Immediate)
            | (Instructions::LoadY, AddressingMode::ZeroPage)
            | (Instructions::LoadY, AddressingMode::Absolute) => {
                self.load_register();
            }

            // TODO
            (Instructions::ReturnFromInterrupt, AddressingMode::Implied) => {
                let value = self.pop_stack();
                self.reg.flags.set_byte(value);
                self.reg.pc = self.pop_stack_u16();
            }

            (Instructions::TransferStackPointerToX, AddressingMode::Implied) => {
                self.reg.idx = self.reg.sp;
                self.next();
            }

            (Instructions::PushAccumulatorOnStack, AddressingMode::Implied) => {
                self.push_stack(self.reg.accumulator);
                self.next();
            }

            (Instructions::PullAccumulatorFromStack, AddressingMode::Implied) => {
                self.reg.accumulator = self.pop_stack();
                self.reg.flags.zero = self.reg.accumulator == 0;
                self.reg.flags.negative = 0x80 & self.reg.accumulator == 0x80;
                self.next()
            }

            (Instructions::ShiftOneLeft, AddressingMode::Absolute)
            | (Instructions::ShiftOneLeft, AddressingMode::ZeroPage)
            | (Instructions::ShiftOneLeft, AddressingMode::ZeroPageX)
            | (Instructions::ShiftOneLeft, AddressingMode::ZeroPageY)
            | (Instructions::ShiftOneLeft, AddressingMode::Accumulator) => self.shift_one_left(),

            (Instructions::ShiftOneRight, AddressingMode::Absolute)
            | (Instructions::ShiftOneRight, AddressingMode::ZeroPage)
            | (Instructions::ShiftOneRight, AddressingMode::ZeroPageX)
            | (Instructions::ShiftOneRight, AddressingMode::ZeroPageY)
            | (Instructions::ShiftOneRight, AddressingMode::Accumulator) => self.shift_one_right(),

            // increment/decrement registers
            (Instructions::IncrementX, AddressingMode::Implied)
            | (Instructions::IncrementY, AddressingMode::Implied) => self.increase_register(),
            (Instructions::DecrementX, AddressingMode::Implied)
            | (Instructions::DecrementY, AddressingMode::Implied) => self.decrease_register(),

            // increase/decrement memory
            (Instructions::IncrementMem, AddressingMode::Absolute) => self.increment_mem(),
            (Instructions::DecrementMem, AddressingMode::Absolute) => self.decrement_mem(),

            // TODO
            (Instructions::SetDecimalMode, AddressingMode::Implied) => self.set_decimal(true),
            (Instructions::ClearCarry, AddressingMode::Implied) => self.set_carry(false),
            (Instructions::SetCarry, AddressingMode::Implied) => self.set_carry(true),
            (Instructions::ClearOverflow, AddressingMode::Implied) => self.set_overflow(false),
            (Instructions::SetInterruptDisable, AddressingMode::Implied) => {
                self.set_interrupts_disabled(true);
            }
            (Instructions::ClearDecimalMode, AddressingMode::Implied) => {
                self.set_decimal(false);
            }

            (Instructions::TestBitsAccumulator, AddressingMode::ZeroPage) => {
                self.test_bit();
            }

            (Instructions::MoveXToStackPointer, AddressingMode::Implied) => {
                self.reg.sp = self.reg.idx;
                println!("Stored X in SP: 0x{:x}", self.reg.sp);
                self.next();
            }

            (Instructions::RotateOneLeft, AddressingMode::Absolute) => {
                let address = self.next_word();
                let value = self.memory.read_byte(address);
                self.memory.write_byte(address, value.rotate_left(1));
                self.next();
            }
            // TODO clean up
            (Instructions::RotateOneLeft, AddressingMode::Accumulator) => {
                self.reg.accumulator = self.reg.accumulator.rotate_left(1);
                self.reg.flags.zero = self.reg.accumulator == 0;
                self.reg.flags.negative = self.reg.accumulator & 0x80 == 0x80;

                self.next();
            }
            (Instructions::RotateOneRight, AddressingMode::Accumulator) => {
                self.reg.accumulator = self.reg.accumulator.rotate_right(1);
                self.reg.flags.zero = self.reg.accumulator == 0;
                self.reg.flags.negative = self.reg.accumulator & 0x80 == 0x80;
                self.next();
            }

            (Instructions::ISC, AddressingMode::Absolute) => self.isc_abs(),

            (Instructions::PushProcessorStatusOnStack, AddressingMode::Implied) => {
                self.push_stack(self.reg.flags.as_byte());
                self.next();
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
                self.next();
            }

            // todo
            (Instructions::TransferAccumulatorToY, AddressingMode::Implied) => {
                self.reg.idy = self.reg.accumulator;
                println!("Transfered A -> Y {}", self.reg.idy);
                self.next();
            }

            // todo
            (Instructions::TransferYToAccumulator, AddressingMode::Implied) => {
                self.reg.accumulator = self.reg.idy;
                println!("Transfered Y -> A {}", self.reg.accumulator);
                self.next();
            }

            // todo
            (Instructions::TransferAccumulatorToX, AddressingMode::Implied) => {
                self.reg.idx = self.reg.accumulator;
                println!("Transfered A -> X {}", self.reg.idx);
                self.next();
            }

            // todo
            (Instructions::TransferXToAccumulator, AddressingMode::Implied) => {
                self.reg.accumulator = self.reg.idx;
                println!("Transfered X -> A {}", self.reg.accumulator);
                self.next();
            }

            // todo
            (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::Immediate)
            | (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::ZeroPage)
            | (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::ZeroPageX)
            | (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::ZeroPageY)
            | (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::AbsoluteY)
            | (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::AbsoluteX)
            | (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::XIndirect)
            | (Instructions::AddMemToAccumulatorWithCarry, AddressingMode::YIndirect) => {
                self.add_mem_to_accumulator_with_carry()
            }

            // todo
            (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::Immediate)
            | (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::ZeroPage)
            | (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::ZeroPageX)
            | (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::ZeroPageY)
            | (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::XIndirect)
            | (Instructions::SubtractAccumulatorWithBorrow, AddressingMode::YIndirect) => {
                self.subtract_accumulator_with_borrow();
            }
            // todo

            /* bitwise */
            (Instructions::ORAccumulator, AddressingMode::Immediate)
            | (Instructions::ORAccumulator, AddressingMode::ZeroPage)
            | (Instructions::ORAccumulator, AddressingMode::ZeroPageX)
            | (Instructions::ORAccumulator, AddressingMode::ZeroPageY)
            | (Instructions::ORAccumulator, AddressingMode::XIndirect)
            | (Instructions::ORAccumulator, AddressingMode::YIndirect) => {
                self.or_accumulator();
            }

            (Instructions::ANDAccumulator, AddressingMode::Immediate)
            | (Instructions::ANDAccumulator, AddressingMode::ZeroPage)
            | (Instructions::ANDAccumulator, AddressingMode::ZeroPageX)
            | (Instructions::ANDAccumulator, AddressingMode::ZeroPageY)
            | (Instructions::ANDAccumulator, AddressingMode::XIndirect)
            | (Instructions::ANDAccumulator, AddressingMode::YIndirect) => {
                self.and_accumulator();
            }

            (Instructions::EORAccumulator, AddressingMode::Immediate)
            | (Instructions::EORAccumulator, AddressingMode::ZeroPage)
            | (Instructions::EORAccumulator, AddressingMode::ZeroPageX)
            | (Instructions::EORAccumulator, AddressingMode::ZeroPageY)
            | (Instructions::EORAccumulator, AddressingMode::XIndirect)
            | (Instructions::EORAccumulator, AddressingMode::YIndirect) => {
                self.eor_accumulator();
            }

            (Instructions::MissingOperation, AddressingMode::Implied) => {
                panic!("Missing operation??")
            }
            (Instructions::NoOperation, _) => {
                println!("NO OP");
                self.next();
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
                // self.reg.pc += operation.1.get_increment();
            }
        }
    }

    fn get_indirect_x(&self) -> u16 {
        let address = self.next_byte();
        let low = self
            .memory
            .read_byte(address.wrapping_add(self.reg.idx) as u16);
        let high = self
            .memory
            .read_byte(address.wrapping_add(self.reg.idx.wrapping_add(1)) as u16);
        (u16::from(high) << 8) | u16::from(low)
    }

    fn get_indirect_y(&self) -> u16 {
        let address = self.next_byte();
        let low = self
            .memory
            .read_byte(address.wrapping_add(self.reg.idy) as u16);
        let high = self
            .memory
            .read_byte(address.wrapping_add(self.reg.idy.wrapping_add(1)) as u16);
        (u16::from(high) << 8) | u16::from(low)
    }

    fn and_accumulator(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Immediate => 0, // unused for immediate
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            AddressingMode::XIndirect => self.get_indirect_x(),
            AddressingMode::YIndirect => self.get_indirect_y(),
            _ => panic!("Invalid mode for or_accumulator {:?}", self.current.mode),
        };

        let value = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            _ => self.memory.read_byte(address),
        };

        let result = value & self.reg.accumulator;
        self.reg.accumulator = result;
        println!("AND {} {} = {}", value, self.reg.accumulator, result);
        self.reg.flags.zero = self.reg.accumulator == 0;
        self.reg.flags.negative = self.reg.accumulator & 0x80 == 0x80;
        self.next();
    }

    fn or_accumulator(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Immediate => 0, // unused for immediate
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            AddressingMode::XIndirect => self.get_indirect_x(),
            AddressingMode::YIndirect => self.get_indirect_y(),
            _ => panic!("Invalid mode for or_accumulator {:?}", self.current.mode),
        };
        let operand = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            _ => self.memory.read_byte(address),
        };

        let result = self.reg.accumulator | operand;
        self.reg.accumulator = result;
        println!("Result: {:X}", result);
        self.reg.flags.negative = 0x80 & result == 0x80;
        self.reg.flags.zero = result == 0;

        self.next();
        println!("ORAccumulator XIndirect: 0x{:x}", address); //tmp // TODO
    }

    fn eor_accumulator(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Immediate => 0, // unused for immediate
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            AddressingMode::XIndirect => self.get_indirect_x(),
            AddressingMode::YIndirect => self.get_indirect_y(),

            _ => panic!(
                "eor_accumulator mode unimplemented! {:?}",
                self.current.mode
            ),
        };
        let value = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            _ => self.memory.read_byte(address),
        };

        self.reg.accumulator ^= value;

        // Update flags (N and Z)
        self.reg.flags.zero = self.reg.accumulator == 0;
        self.reg.flags.negative = (self.reg.accumulator & 0x80) != 0;

        self.next();
    }

    fn subtract_accumulator_with_borrow(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Immediate => 0, // unused
            AddressingMode::XIndirect => self.get_indirect_x(),
            AddressingMode::YIndirect => self.get_indirect_y(),
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            _ => panic!(
                "invalid mode for subtract_accumulator_with_borrow {:?}",
                self.current.mode
            ),
        };

        let operand = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            _ => self.memory.read_byte(address),
        };

        dbg!(self.reg.flags.carry);
        dbg!(self.reg.accumulator, operand);
        // Perform Subtract Immediate with Carry
        // let borrow = if self.reg.flags.carry && (operand > self.reg.accumulator || operand == 0) {
        //     1
        // } else {
        //     0
        // };
        let borrow = if self.reg.flags.carry && (operand >= self.reg.accumulator || operand == 0) {
            1 // Borrow if the carry is set and operand is greater than or equal to the accumulator, or operand is zero
        } else {
            0 // No borrow otherwise
        };
        let result = self
            .reg
            .accumulator
            .wrapping_sub(operand)
            .wrapping_sub(borrow);

        // Update CPU state
        self.reg.accumulator = result;
        self.reg.flags.carry = result as i8 > 0 || borrow == 0;
        self.reg.flags.zero = result == 0;
        self.reg.flags.negative = result & 0x80 == 0x80;
        self.reg.flags.overflow = (self.reg.accumulator ^ result) & 0x80 != 0
            && (self.reg.accumulator ^ result) & 0x80 != 0;

        // ... other flag updates and state modifications
        dbg!(self.reg.accumulator, operand, borrow);

        self.next();
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
            panic!("PC counter too high! {}", self.reg.pc)
        }
        let next_instruction = self.memory.read_byte(self.reg.pc);
        let (instruction, addressing_mode) = Self::decode_instruction(next_instruction);
        self.current = CurrentInstruction {
            op: instruction,
            mode: addressing_mode,
        };
        println!("0x{:X}: {}", self.reg.pc, self.current);
        // increment pc for each instruction based on instruction type

        // dbg!(&instruction);

        self.execute();
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
    // TODO need to push address onto stack and set block bit
    fn breakpoint(&mut self) {
        // add PC
        println!("BREAKPOINT: 0x{:X}", self.reg.pc);

        // Buffer to hold the input
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line!");
        self.next();
    }

    // TODO refactor
    fn compare_register(&mut self) {
        let value = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            AddressingMode::AbsoluteY => {
                let address = self.next_word();
                self.memory
                    .read_byte(address.wrapping_add(self.reg.idy as u16))
            }
            // todo clean these
            AddressingMode::XIndirect => {
                let address = self.get_indirect_x();
                self.memory.read_byte(address)
            }
            AddressingMode::YIndirect => {
                let address = self.get_indirect_y();
                self.memory.read_byte(address)
            }
            AddressingMode::ZeroPage => {
                let address = self.next_byte() as u16;
                self.memory.read_byte(address)
            }
            AddressingMode::ZeroPageX => {
                let address = self.next_byte().wrapping_add(self.reg.idx) as u16;
                self.memory.read_byte(address)
            }
            AddressingMode::ZeroPageY => {
                let address = self.next_byte().wrapping_add(self.reg.idy) as u16;
                self.memory.read_byte(address)
            }
            _ => {
                panic!(
                    "Unimplemented! Compare register {:?} {:?}",
                    self.current.op, self.current.mode
                )
            }
        };

        let register = match self.current.op {
            Instructions::CompareAccumulator => &mut self.reg.accumulator,
            Instructions::CompareX => &mut self.reg.idx,
            Instructions::CompareY => &mut self.reg.idy,
            _ => panic!("invalid current.op {:?}", self.current.op),
        };
        println!("Value: {} Register: {}", value, *register);
        let result = register.wrapping_sub(value);

        self.reg.flags.zero = result == 0;
        self.reg.flags.negative = (result & 0x80) != 0;
        self.reg.flags.carry = *register >= value;

        self.next();
    }

    fn branch(&mut self) {
        let condition = match self.current.op {
            Instructions::BranchOnResultMinus => self.reg.flags.negative,
            Instructions::BranchOnResultZero => self.reg.flags.zero,
            Instructions::BranchOnResultNotZero => !self.reg.flags.zero,
            Instructions::BranchOnResultPlus => !self.reg.flags.negative,
            Instructions::BranchOnOverflowSet => self.reg.flags.overflow,
            Instructions::BranchOnOverflowClear => !self.reg.flags.overflow,
            Instructions::BranchOnCarrySet => self.reg.flags.carry,
            Instructions::BranchOnCarryClear => !self.reg.flags.carry,
            _ => panic!("Invalid instruction for branch: {:?}", self.current.op),
        };

        if condition {
            self.reg.pc = match self.current.mode {
                AddressingMode::Relative => {
                    let value = self.next_byte();
                    self.reg.pc + 2 + value as u16
                }
                _ => panic!("Unimplemented! Branch: {:?}", self.current.mode),
            };
            println!("Branching to addr: 0x{:x}", self.reg.pc);
        } else {
            self.next();
        }

        dbg!(&self.reg.flags);
    }

    // jump to address
    fn jump(&mut self, address: u16) {
        self.set_pc(address);
        println!("Jumped! {:?} {:x}", self.current.op, self.reg.pc);
    }
}

// set interrupt disable status
// fn sei(cpu: &mut NesCpu) {
//     cpu.reg.flags = CPU.reg.flags | CPUFlags::InterruptDisabled;
// }
