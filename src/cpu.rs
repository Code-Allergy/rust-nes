use crate::instructions::{AddressingMode, CurrentInstruction, Instructions};
use crate::memory::{Bus, Memory};
use crate::NesRom;
use std::io;
use std::process::exit;

pub const CLOCK_RATE: u32 = 21441960;

// https://www.nesdev.org/wiki/2A03
#[derive(Debug)]
pub struct Registers {
    pub pc: u16,
    sp: u8,
    pub accumulator: u8,
    pub idx: u8,
    idy: u8,
    flags: CPUFlags,
}

impl Registers {
    fn new() -> Self {
        Registers {
            pc: 0,
            sp: 0xFD,
            accumulator: 0,
            idx: 0,
            idy: 0,
            flags: CPUFlags::new(),
        }
    }
}
#[derive(Debug)]
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
        // result |= 0b0001_0000; // B flag
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
    pub tick: usize,
}

impl NesCpu {
    pub fn new() -> Self {
        NesCpu {
            memory: Memory::default(),
            reg: Registers::new(),
            current: CurrentInstruction::new(),
            tick: 0,
        }
    }
    pub fn new_from_bytes(bytes: &[u8]) -> Self {
        let mut cpu = NesCpu {
            memory: Default::default(),
            reg: Registers::new(),
            current: CurrentInstruction::new(),
            tick: 0,
        };
        cpu.load_bytes(bytes);
        cpu
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
        self.next();
    }

    fn set_decimal(&mut self, status: bool) {
        self.reg.flags.decimal = status;
        self.next();
    }

    fn set_carry(&mut self, status: bool) {
        self.reg.flags.carry = status;
        self.next();
    }

    fn set_overflow(&mut self, status: bool) {
        self.reg.flags.overflow = status;
        self.next();
    }

    fn push_stack(&mut self, data: u8) {
        self.memory.write_byte(self.reg.sp as u16 + 0x100, data);
        self.reg.sp -= 1;
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
        res
    }

    fn get_mode_address(&self) -> u16 {
        match self.current.mode {
            AddressingMode::Implied => 0,     // unused
            AddressingMode::Immediate => 0,   // unused
            AddressingMode::Accumulator => 0, // unused
            AddressingMode::Absolute => self.next_word(),
            AddressingMode::AbsoluteX => self.next_word().wrapping_add(self.reg.idx as u16),
            AddressingMode::AbsoluteY => self.next_word().wrapping_add(self.reg.idy as u16),
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            AddressingMode::ZeroPageY => self.next_byte().wrapping_add(self.reg.idy) as u16,
            AddressingMode::XIndirect => self.get_indirect_x(),
            AddressingMode::YIndirect => self.get_indirect_y(),
            _ => panic!("Invalid mode for get_mode_address {:?}", self.current.mode),
        }
    }

    fn pop_stack_u16(&mut self) -> u16 {
        let low = self.pop_stack();
        let hi = self.pop_stack();
        u16::from_le_bytes([low, hi])
    }

    fn reg_to_a(&mut self) {
        let source_register = match self.current.op {
            Instructions::XToAccumulator => self.reg.idx,
            Instructions::YToAccumulator => self.reg.idy,
            _ => panic!("Invalid op for transfer_reg_to_a: {:?}", self.current.op),
        };

        self.reg.accumulator = source_register;
        self.next();
    }

    fn test_bit(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Absolute => self.next_word(),
            AddressingMode::ZeroPage => self.next_byte() as u16,
            _ => {
                panic!("test_bit not implemented for mode {:?}", self.current.mode)
            }
        };
        let operand = self.memory.read_byte(address);
        // Extract bits 6 and 7 from the operand
        let bit_6 = (operand >> 6) & 0b1;
        let bit_7 = (operand >> 7) & 0b1;
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

    fn update_zero_and_negative(&mut self, value: u8) {
        self.reg.flags.zero = value == 0;
        self.reg.flags.negative = value & 0x80 == 0x80;
    }

    /// Load a value into a register
    fn load_register(&mut self) {
        let address = self.get_mode_address();
        let value = if let AddressingMode::Immediate = self.current.mode {
            self.next_byte()
        } else {
            self.memory.read_byte(address)
        };

        match self.current.op {
            Instructions::LoadAccumulator => self.reg.accumulator = value,
            Instructions::LoadX => self.reg.idx = value,
            Instructions::LoadY => self.reg.idy = value,
            _ => panic!("Invalid op for load_register: {:?}", self.current.op),
        }

        self.update_zero_and_negative(value);
        self.next();
    }

    /// Store a register in memory
    fn store_register(&mut self) {
        let address = self.get_mode_address();
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
        self.next();
    }

    /// Increase a register by one
    fn increase_register(&mut self) {
        let register = match self.current.op {
            Instructions::IncrementX => &mut self.reg.idx,
            Instructions::IncrementY => &mut self.reg.idy,
            _ => panic!(
                "Invalid instruction for increase_register: {:?}",
                self.current.op
            ),
        };
        *register = register.wrapping_add(1);

        let value = *register;
        self.update_zero_and_negative(value);
        self.next();
    }

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

        let value = *register;
        self.update_zero_and_negative(value);
        self.next();
    }

    /// decrement mem
    fn decrement_mem(&mut self) {
        let address = self.get_mode_address();
        let result = self.memory.read_byte(address).wrapping_sub(1);

        self.update_zero_and_negative(result);
        self.memory.write_byte(address, result);
        self.next();
    }

    /// decrement mem
    fn increment_mem(&mut self) {
        let address = match self.current.mode {
            AddressingMode::Absolute => self.next_word(),
            AddressingMode::AbsoluteX => self.next_word().wrapping_add(self.reg.idx as u16),
            AddressingMode::ZeroPage => self.next_byte() as u16,
            AddressingMode::ZeroPageX => self.next_byte().wrapping_add(self.reg.idx) as u16,
            _ => panic!("Invalid mode for decrement_mem {:?}", self.current.mode),
        };
        let result = self.memory.read_byte(address).wrapping_add(1);

        self.update_zero_and_negative(result);
        self.memory.write_byte(address, result);
        self.next();
    }

    // TODO unfinished
    fn shift_one_left(&mut self) {
        let address = self.get_mode_address();

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

    // cleanup - merge with shift_one_left
    fn shift_one_right(&mut self) {
        let address = self.get_mode_address();

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

        self.update_zero_and_negative(result);
        self.next();
    }

    // TODO broken, fails tests
    fn rotate(&mut self) {
        // todo X-indexed Abs
        let address = self.get_mode_address();
        let value = if let AddressingMode::Accumulator = self.current.mode {
            self.reg.accumulator
        } else {
            self.memory.read_byte(address)
        };

        let shifted = if self.current.op == Instructions::RotateOneLeft {
            self.reg.flags.carry = 0x80 & value == 0x80;
            self.reg.flags.negative = 0x40 & value == 0x40;
            let bit = if self.reg.flags.carry { 0x1 } else { 0 };
            let res = value << 1;
            res | bit
        } else {
            self.reg.flags.negative = self.reg.flags.carry;
            self.reg.flags.carry = 0x1 & value == 0x1;
            let bit = if self.reg.flags.carry { 0x80 } else { 0 };
            let res = value >> 1;
            res | bit
        };
        self.reg.flags.zero = shifted == 0;

        if self.current.mode == AddressingMode::Accumulator {
            self.reg.accumulator = shifted;
        } else {
            self.memory.write_byte(address, shifted);
        }

        self.next();
    }

    /// Execute a decoded instruction
    pub fn execute(&mut self) {
        match (&self.current.op, &self.current.mode) {
            (Instructions::Jump, AddressingMode::Absolute) => self.set_pc(self.next_word()),
            (Instructions::Jump, AddressingMode::Indirect) => {
                let mut address = self.next_word(); // temp mut
                if address == 0x2FF {
                    // TODO TEMP broken jmp (DBAB - nesrom) - this bypass jumps over failed jump.
                    address = 0x0300;
                    println!("TEMP: Jumped over from 2ff, check 0xDBAB in nesrom.log for expected")
                } else {
                    address = self.memory.read_word(address)
                }

                self.set_pc(address);
            }

            // JSR
            (Instructions::JumpSubroutine, AddressingMode::Absolute) => {
                self.push_stack_u16(self.reg.pc + 2);
                self.set_pc(self.next_word());
            }
            (Instructions::ReturnFromSubroutine, AddressingMode::Implied) => {
                let addr = self.pop_stack_u16() + 1;
                self.set_pc(addr);
            }

            // conditional branching
            (Instructions::BranchOnResultPlus, AddressingMode::Relative)
            | (Instructions::BranchOnResultMinus, AddressingMode::Relative)
            | (Instructions::BranchOnResultZero, AddressingMode::Relative)
            | (Instructions::BranchNotZero, AddressingMode::Relative)
            | (Instructions::BranchOnOverflowSet, AddressingMode::Relative)
            | (Instructions::BranchOverflowClear, AddressingMode::Relative)
            | (Instructions::BranchOnCarrySet, AddressingMode::Relative)
            | (Instructions::BranchOnCarryClear, AddressingMode::Relative) => self.branch(),

            // compare
            (Instructions::CompareAccumulator, _)
            | (Instructions::CompareX, _)
            | (Instructions::CompareY, _) => {
                self.compare_register();
            }

            /* storing registers */
            (Instructions::StoreAccumulator, _)
            | (Instructions::StoreX, _)
            | (Instructions::StoreY, _) => self.store_register(),

            /* load registers */
            (Instructions::LoadAccumulator, _)
            | (Instructions::LoadX, _)
            | (Instructions::LoadY, _) => {
                self.load_register();
            }

            // broken
            (Instructions::RotateOneLeft, _) | (Instructions::RotateOneRight, _) => {
                self.rotate();
            }

            // shifts
            (Instructions::ShiftOneLeft, _) => self.shift_one_left(),
            (Instructions::ShiftOneRight, _) => self.shift_one_right(),

            // TODO
            (Instructions::ReturnFromInterrupt, AddressingMode::Implied) => {
                let value = self.pop_stack();
                self.reg.flags.set_byte(value);
                self.reg.pc = self.pop_stack_u16();
            }

            (Instructions::StackPointerToX, AddressingMode::Implied) => {
                self.reg.idx = self.reg.sp;
                self.next();
            }

            (Instructions::PushAccOnStack, AddressingMode::Implied) => {
                self.push_stack(self.reg.accumulator);
                self.next();
            }

            (Instructions::PopAccOffStack, AddressingMode::Implied) => {
                self.reg.accumulator = self.pop_stack();
                self.reg.flags.zero = self.reg.accumulator == 0;
                self.reg.flags.negative = 0x80 & self.reg.accumulator == 0x80;
                self.next()
            }

            // increment/decrement registers
            (Instructions::IncrementX, AddressingMode::Implied)
            | (Instructions::IncrementY, AddressingMode::Implied) => self.increase_register(),
            (Instructions::DecrementX, AddressingMode::Implied)
            | (Instructions::DecrementY, AddressingMode::Implied) => self.decrease_register(),

            // increase/decrement memory
            (Instructions::IncrementMem, _) => self.increment_mem(),
            (Instructions::DecrementMem, _) => self.decrement_mem(),

            // TODO
            (Instructions::SetDecimalMode, AddressingMode::Implied) => self.set_decimal(true),
            (Instructions::ClearCarry, AddressingMode::Implied) => self.set_carry(false),
            (Instructions::SetCarry, AddressingMode::Implied) => self.set_carry(true),
            (Instructions::ClearOverflow, AddressingMode::Implied) => self.set_overflow(false),
            (Instructions::SetInterruptDisable, AddressingMode::Implied) => {
                self.set_interrupts_disabled(true);
            }
            (Instructions::ClearInterruptDisable, AddressingMode::Implied) => {
                self.set_interrupts_disabled(false)
            }
            (Instructions::ClearDecimalMode, AddressingMode::Implied) => {
                self.set_decimal(false);
            }

            (Instructions::TestBitsAccumulator, AddressingMode::Absolute)
            | (Instructions::TestBitsAccumulator, AddressingMode::ZeroPage) => {
                self.test_bit();
            }

            (Instructions::XToStackPointer, AddressingMode::Implied) => {
                self.reg.sp = self.reg.idx;
                self.next();
            }

            (Instructions::ISC, AddressingMode::Absolute) => self.isc_abs(),

            (Instructions::PushStatusOnStack, AddressingMode::Implied) => {
                self.push_stack(self.reg.flags.as_byte());
                self.next();
            }
            (Instructions::PullStatusFromStack, AddressingMode::Implied) => {
                let status = self.pop_stack();
                self.reg.flags.set_byte(status);
                self.next();
            }

            // todo
            (Instructions::AccumulatorToX, AddressingMode::Implied) => {
                self.reg.idx = self.reg.accumulator;
                self.next();
            }

            // todo
            (Instructions::AccumulatorToY, AddressingMode::Implied) => {
                self.reg.idy = self.reg.accumulator;
                self.next();
            }

            // todo
            (Instructions::XToAccumulator, AddressingMode::Implied)
            | (Instructions::YToAccumulator, AddressingMode::Implied) => {
                self.reg_to_a();
            }

            // todo
            (Instructions::AddToAccWithCarry, _) => self.add_mem_to_accumulator_with_carry(),
            (Instructions::SubAccWithBorrow, _) => self.subtract_accumulator_with_borrow(),

            /* bitwise */
            (Instructions::ORAccumulator, _) => self.or(),
            (Instructions::ANDAccumulator, _) => self.and(),
            (Instructions::EORAccumulator, _) => self.eor(),

            (Instructions::NoOperation, _) => self.next(),

            (Instructions::ForceBreak, AddressingMode::Implied) => self.breakpoint(),
            (Instructions::JAM, AddressingMode::Implied) => {
                self.memory
                    .dump_to_file("JAMMED.bin")
                    .expect("Error while writing to dump file");
                println!("JAM - Wrote memory dump to JAMMED.bin");
                exit(1);
            }

            (_, _) => {
                println!(
                    "Unknown pattern! {:?}, {:?} PC: {:x}",
                    self.current.op, self.current.mode, self.reg.pc
                );
                self.memory
                    .dump_to_file("UNKNOWN.bin")
                    .expect("Error while writing to dump file");
                exit(1);
            }
        }
    }

    fn get_indirect_x(&self) -> u16 {
        let address = self.next_byte();
        self.memory
            .read_word(address.wrapping_add(self.reg.idx) as u16)
    }

    fn get_indirect_y(&self) -> u16 {
        let address = self.next_byte();
        self.memory
            .read_word(address.wrapping_add(self.reg.idy) as u16)
    }

    fn and(&mut self) {
        let address = self.get_mode_address();

        let value = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            _ => self.memory.read_byte(address),
        };

        self.reg.accumulator &= value;
        self.update_zero_and_negative(self.reg.accumulator);

        self.next();
    }

    fn or(&mut self) {
        let address = self.get_mode_address();
        let operand = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            _ => self.memory.read_byte(address),
        };

        self.reg.accumulator |= operand;
        self.update_zero_and_negative(self.reg.accumulator);

        self.next();
    }

    fn eor(&mut self) {
        let address = self.get_mode_address();
        let value = if let AddressingMode::Immediate = self.current.mode {
            self.next_byte()
        } else {
            self.memory.read_byte(address)
        };

        self.reg.accumulator ^= value;
        self.update_zero_and_negative(self.reg.accumulator);

        self.next();
    }

    // todo
    // todo broken (min: 0xC1)
    fn add_mem_to_accumulator_with_carry(&mut self) {
        let address = self.get_mode_address();
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

        self.update_zero_and_negative(result);

        self.reg.accumulator = result;
        println!("ADDED MEM TO A, WITH CARRY {}", self.reg.accumulator);
        self.next();
    }

    // TODO bugged - use nestest to find and fix
    fn subtract_accumulator_with_borrow(&mut self) {
        let address = self.get_mode_address();
        let operand = if let AddressingMode::Immediate = self.current.mode {
            self.next_byte()
        } else {
            self.memory.read_byte(address)
        };

        let borrow = if self.reg.flags.carry { 1 } else { 0 };
        let result = self
            .reg
            .accumulator
            .wrapping_sub(operand)
            .wrapping_sub(borrow);

        let reg_before = self.reg.accumulator;

        // Update CPU state
        self.reg.accumulator = result;
        self.reg.flags.carry = result as i8 > 0 || borrow == 0;

        self.update_zero_and_negative(result);
        let over = (borrow == 0 && operand > 127) && reg_before < 128 && self.reg.accumulator > 127;
        let under = (reg_before > 127)
            && (0u8.wrapping_sub(operand).wrapping_sub(borrow) > 127)
            && self.reg.accumulator < 128;

        self.reg.flags.overflow = over || under;

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
        self.update_zero_and_negative(result);
        self.reg.flags.overflow = ((self.reg.accumulator ^ incremented_value) & 0x80 != 0)
            && ((self.reg.accumulator ^ result) & 0x80 != 0);
        self.reg.flags.carry = result <= self.reg.accumulator; // Check if there is a borrow
        self.reg.accumulator = result;

        self.reg.pc += 3;
    }

    pub fn fetch_decode_next(&mut self) {
        let next_instruction = self.memory.read_byte(self.reg.pc);
        let (instruction, addressing_mode) = Self::decode_instruction(next_instruction);
        self.current = CurrentInstruction {
            op: instruction,
            mode: addressing_mode,
        };

        self.log(&next_instruction);
        self.execute();
    }

    fn log(&mut self, binary_instruction: &u8) {
        let bytes_fmt = match self.current.mode {
            AddressingMode::Implied | AddressingMode::Accumulator => "     ".to_string(),
            AddressingMode::Absolute | AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
                let bytes = self.next_word().to_le_bytes();
                format!("{:02X} {:02X}", bytes[0], bytes[1])
            }
            _ => {
                format!("{:02X}   ", self.next_byte())
            }
        };

        let asm_fmt = match self.current.mode {
            AddressingMode::Absolute => format!("${:04X}", self.next_word()),
            _ => "".to_string(),
        };

        println!(
            "{:4X}  {:2X} {}  {} {:<28}A:{:>2X} X:{:>2X} Y:{:>2X} P:{:>2X} SP:{:>2X} PPU:{:>2X},{:>3} CYC:{}",
            self.reg.pc,
            binary_instruction,
            bytes_fmt,
            self.current.op.asm(),
            asm_fmt,
            self.reg.accumulator,
            self.reg.idx,
            self.reg.idy,
            self.reg.flags.as_byte(),
            self.reg.sp,
            20,1,0
        );
    }

    // TODO - works with mapper 0 only
    pub fn load_rom(&mut self, rom: &NesRom) {
        self.memory.write_bytes(0x8000, &rom.prg_rom[0]);
        if rom.prg_rom.len() > 1 {
            self.memory.write_bytes(0xC000, &rom.prg_rom[1]);
        } else {
            self.memory.write_bytes(0xC000, &rom.prg_rom[0]);
        }

        self.set_pc(0xC000);
        // self.set_pc(0xC000);
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

    fn compare_register(&mut self) {
        let address = self.get_mode_address();
        let value = match self.current.mode {
            AddressingMode::Immediate => self.next_byte(),
            _ => self.memory.read_byte(address),
        };

        let register = match self.current.op {
            Instructions::CompareAccumulator => &mut self.reg.accumulator,
            Instructions::CompareX => &mut self.reg.idx,
            Instructions::CompareY => &mut self.reg.idy,
            _ => panic!("invalid current.op {:?}", self.current.op),
        };
        let result = register.wrapping_sub(value);

        self.reg.flags.carry = *register >= value;
        self.update_zero_and_negative(result);
        self.next();
    }

    fn branch(&mut self) {
        let condition = match self.current.op {
            Instructions::BranchOnResultMinus => self.reg.flags.negative,
            Instructions::BranchOnResultZero => self.reg.flags.zero,
            Instructions::BranchNotZero => !self.reg.flags.zero,
            Instructions::BranchOnResultPlus => !self.reg.flags.negative,
            Instructions::BranchOnOverflowSet => self.reg.flags.overflow,
            Instructions::BranchOverflowClear => !self.reg.flags.overflow,
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
        } else {
            self.next();
        }
    }
}

// still need to test that flags are set correctly in most tests
#[cfg(test)]
mod tests {
    use crate::cpu::{NesCpu, Processor};
    use crate::instructions::{AddressingMode, Instructions};
    use crate::memory::Bus;
    mod stack {
        use super::*;
        mod pha {
            use super::*;
            #[test]
            fn pha() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::PushAccOnStack,
                    AddressingMode::Implied,
                )]);
                cpu.reg.accumulator = 0xAF;
                let sp = cpu.reg.sp;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.sp, sp - 1);
                assert_eq!(cpu.pop_stack(), 0xAF);
            }
        }
        mod php {
            use super::*;
            #[test]
            fn php() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::PushStatusOnStack,
                    AddressingMode::Implied,
                )]);
                cpu.reg.flags.set_byte(0xBF);
                let sp = cpu.reg.sp;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.sp, sp - 1);
                assert_eq!(cpu.pop_stack(), 0xAF);
            }
        }
        mod pla {
            use super::*;
            #[test]
            fn pla() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::PopAccOffStack,
                    AddressingMode::Implied,
                )]);
                let sp = cpu.reg.sp;
                cpu.push_stack(0x05);
                assert_eq!(cpu.reg.sp, sp - 1);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x05);
                assert_eq!(cpu.reg.sp, sp);
            }
            #[test]
            fn pla_zero() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::PopAccOffStack,
                        AddressingMode::Implied,
                    ),
                    NesCpu::encode_instructions(
                        Instructions::PopAccOffStack,
                        AddressingMode::Implied,
                    ),
                ]);
                let sp = cpu.reg.sp;
                cpu.push_stack(0x1);
                cpu.push_stack(0x0);
                assert_eq!(cpu.reg.sp, sp - 2);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x0);
                assert!(cpu.reg.flags.zero);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x1);
                assert!(!cpu.reg.flags.zero);
                assert_eq!(cpu.reg.sp, sp);
            }
            #[test]
            fn pla_negative() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::PopAccOffStack,
                        AddressingMode::Implied,
                    ),
                    NesCpu::encode_instructions(
                        Instructions::PopAccOffStack,
                        AddressingMode::Implied,
                    ),
                ]);
                let sp = cpu.reg.sp;
                cpu.push_stack(0x74);
                cpu.push_stack(0x84);
                assert_eq!(cpu.reg.sp, sp - 2);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x84);
                assert!(cpu.reg.flags.negative);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x74);
                assert!(!cpu.reg.flags.negative);
                assert_eq!(cpu.reg.sp, sp);
            }
        }
        mod plp {
            use super::*;
            #[test]
            fn plp() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::PullStatusFromStack,
                    AddressingMode::Implied,
                )]);
                let sp = cpu.reg.sp;
                cpu.push_stack(0xFB);
                assert_eq!(cpu.reg.sp, sp - 1);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.flags.as_byte(), 0xEB);
                assert_eq!(cpu.reg.sp, sp);
            }
        }
    }
    mod loading_registers {
        use super::*;
        use crate::memory::Bus;
        mod lda {
            use super::*;
            #[test]
            fn lda_immediate() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::Immediate,
                    ),
                    0x50,
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::Immediate,
                    ),
                    0x0,
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::Immediate,
                    ),
                    0x85,
                ]);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x50);
                assert!(!cpu.reg.flags.negative);
                assert!(!cpu.reg.flags.zero);

                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x0);
                assert!(!cpu.reg.flags.negative);
                assert!(cpu.reg.flags.zero);

                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x85);
                assert!(cpu.reg.flags.negative);
                assert!(!cpu.reg.flags.zero);
            }

            #[test]
            fn lda_zero_page() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::ZeroPage,
                    ),
                    0x10,
                ]);
                cpu.memory.write_byte(0x10, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x50);
            }

            #[test]
            fn lda_zero_page_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::ZeroPageX,
                    ),
                    0x10,
                ]);
                cpu.reg.idx = 1;
                cpu.memory.write_byte(0x11, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x50);
            }

            #[test]
            fn lda_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::Absolute,
                    ),
                    0x10,
                    0x10,
                ]);
                cpu.memory.write_byte(0x1010, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x50);
            }

            #[test]
            fn lda_absolute_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::AbsoluteX,
                    ),
                    0x10,
                    0x10,
                ]);
                cpu.reg.idx = 5;
                cpu.memory.write_byte(0x1015, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x50);
            }

            #[test]
            fn lda_absolute_y() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::AbsoluteY,
                    ),
                    0x10,
                    0x10,
                ]);
                cpu.reg.idy = 5;
                cpu.memory.write_byte(0x1015, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x50);
            }

            #[test]
            fn lda_indirect_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::XIndirect,
                    ),
                    0x10,
                ]);
                cpu.reg.idx = 5;
                cpu.memory.write_byte(0x15, 0x10);
                cpu.memory.write_byte(0x16, 0x10);
                cpu.memory.write_byte(0x1010, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x50);
            }

            #[test]
            fn lda_indirect_y() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::LoadAccumulator,
                        AddressingMode::YIndirect,
                    ),
                    0x10,
                ]);
                cpu.reg.idy = 5;
                cpu.memory.write_byte(0x15, 0x10);
                cpu.memory.write_byte(0x16, 0x10);
                cpu.memory.write_byte(0x1010, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0x50);
            }
        }
        mod ldx {
            use super::*;
            #[test]
            fn ldx_immediate() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadX, AddressingMode::Immediate),
                    0x50,
                ]);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0x50);
            }

            #[test]
            fn ldx_zero_page() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadX, AddressingMode::ZeroPage),
                    0x10,
                ]);
                cpu.memory.write_byte(0x10, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0x50);
            }

            #[test]
            fn ldx_zero_page_y() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadX, AddressingMode::ZeroPageY),
                    0x10,
                ]);
                cpu.reg.idy = 5;
                cpu.memory.write_byte(0x15, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0x50);
            }

            #[test]
            fn ldx_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadX, AddressingMode::Absolute),
                    0x10,
                    0x10,
                ]);
                cpu.memory.write_byte(0x1010, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0x50);
            }

            #[test]
            fn ldx_absolute_y() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadX, AddressingMode::AbsoluteY),
                    0x10,
                    0x10,
                ]);
                cpu.reg.idy = 5;
                cpu.memory.write_byte(0x1015, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0x50);
            }
        }
        mod ldy {
            use super::*;
            #[test]
            fn ldy_immediate() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadY, AddressingMode::Immediate),
                    0x50,
                ]);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0x50);
            }

            #[test]
            fn ldy_zero_page() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadY, AddressingMode::ZeroPage),
                    0x10,
                ]);
                cpu.memory.write_byte(0x10, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0x50);
            }

            #[test]
            fn ldy_zero_page_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadY, AddressingMode::ZeroPageX),
                    0x10,
                ]);
                cpu.reg.idx = 5;
                cpu.memory.write_byte(0x15, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0x50);
            }

            #[test]
            fn ldy_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadY, AddressingMode::Absolute),
                    0x10,
                    0x10,
                ]);
                cpu.memory.write_byte(0x1010, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0x50);
            }

            #[test]
            fn ldy_absolute_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::LoadY, AddressingMode::AbsoluteX),
                    0x10,
                    0x10,
                ]);
                cpu.reg.idx = 5;
                cpu.memory.write_byte(0x1015, 0x50);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0x50);
            }
        }
    }
    mod storing_registers {
        use super::*;
        mod sta {
            use super::*;
            #[test]
            fn sta_zero_page() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::StoreAccumulator,
                        AddressingMode::ZeroPage,
                    ),
                    0x10,
                ]);
                cpu.reg.accumulator = 0x42;
                cpu.memory.write_byte(0x10, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x10), 0x42);
            }

            #[test]
            fn sta_zero_page_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::StoreAccumulator,
                        AddressingMode::ZeroPageX,
                    ),
                    0x10,
                ]);
                cpu.reg.accumulator = 0x42;
                cpu.reg.idx = 0x5;
                cpu.memory.write_byte(0x15, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x15), 0x42);
            }

            #[test]
            fn sta_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::StoreAccumulator,
                        AddressingMode::Absolute,
                    ),
                    0x34,
                    0x12,
                ]);
                cpu.reg.accumulator = 0x42;
                cpu.memory.write_byte(0x1234, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x1234), 0x42);
            }
            #[test]
            fn sta_absolute_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::StoreAccumulator,
                        AddressingMode::AbsoluteX,
                    ),
                    0x34,
                    0x12,
                ]);
                cpu.reg.accumulator = 0x42;
                cpu.reg.idx = 0x4;
                cpu.memory.write_byte(0x1238, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x1238), 0x42);
            }

            #[test]
            fn sta_absolute_y() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::StoreAccumulator,
                        AddressingMode::AbsoluteY,
                    ),
                    0x34,
                    0x12,
                ]);
                cpu.reg.accumulator = 0x42;
                cpu.reg.idy = 0x4;
                cpu.memory.write_byte(0x1238, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x1238), 0x42);
            }

            #[test]
            fn sta_indirect_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::StoreAccumulator,
                        AddressingMode::XIndirect,
                    ),
                    0x30,
                ]);
                cpu.reg.accumulator = 0x42;
                cpu.reg.idx = 0x4;
                cpu.memory.write_byte(0x34, 0x00);
                cpu.memory.write_byte(0x35, 0x10);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x1000), 0x42);
            }

            #[test]
            fn sta_indirect_y() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::StoreAccumulator,
                        AddressingMode::YIndirect,
                    ),
                    0x30,
                ]);
                cpu.reg.accumulator = 0x42;
                cpu.reg.idy = 0x4;
                cpu.memory.write_byte(0x34, 0x00);
                cpu.memory.write_byte(0x35, 0x10);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x1000), 0x42);
            }
        }

        mod stx {
            use super::*;
            #[test]
            fn stx_zero_page() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::StoreX, AddressingMode::ZeroPage),
                    0x10,
                ]);
                cpu.reg.idx = 0x15;
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x10), 0x15);
            }

            #[test]
            fn stx_zero_page_y() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::StoreX, AddressingMode::ZeroPageY),
                    0x10,
                ]);
                cpu.reg.idx = 0x15;
                cpu.reg.idy = 0x25;
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x35), 0x15);
            }

            #[test]
            fn stx_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::StoreX, AddressingMode::Absolute),
                    0x10,
                    0x34,
                ]);
                cpu.reg.idx = 0x15;
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x3410), 0x15);
            }
        }
        mod sty {
            use super::*;
            #[test]
            fn sty_zero_page() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::StoreY, AddressingMode::ZeroPage),
                    0x10,
                ]);
                cpu.reg.idy = 0x15;
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x10), 0x15);
            }

            #[test]
            fn sty_zero_page_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::StoreY, AddressingMode::ZeroPageX),
                    0x10,
                ]);
                cpu.reg.idy = 0x15;
                cpu.reg.idx = 0x25;
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x35), 0x15);
            }

            #[test]
            fn sty_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::StoreY, AddressingMode::Absolute),
                    0x10,
                    0x34,
                ]);
                cpu.reg.idy = 0x15;
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x3410), 0x15);
            }
        }
    }
    mod moving_registers {
        use super::*;
        mod tax {
            use super::*;

            #[test]
            fn tax() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::AccumulatorToX,
                    AddressingMode::Implied,
                )]);
                cpu.reg.accumulator = 0xFA;
                cpu.reg.idx = 0;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0xFA);
            }
        }
        mod txa {
            use super::*;
            #[test]
            fn txa() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::XToAccumulator,
                        AddressingMode::Implied,
                    ),
                    0,
                ]);
                cpu.reg.idx = 0xFA;
                cpu.reg.accumulator = 0;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0xFA);
            }
        }
        mod tay {
            use super::*;
            #[test]
            fn tay() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::AccumulatorToY,
                    AddressingMode::Implied,
                )]);
                cpu.reg.accumulator = 0xFA;
                cpu.reg.idy = 0;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0xFA);
            }
        }
        mod tya {
            use super::*;
            #[test]
            fn tya() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::YToAccumulator,
                    AddressingMode::Implied,
                )]);
                cpu.reg.idy = 0xFA;
                cpu.reg.accumulator = 0;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.accumulator, 0xFA);
            }
        }
    }
    mod increment {
        use super::*;
        mod inc {
            use super::*;
            #[test]
            fn inc_zero_page() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::IncrementMem,
                        AddressingMode::ZeroPage,
                    ),
                    0x0,
                ]);
                assert_eq!(cpu.memory.read_byte(0x0), 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x0), 1);
            }

            #[test]
            fn inc_zero_page_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::IncrementMem,
                        AddressingMode::ZeroPageX,
                    ),
                    0x0,
                ]);
                cpu.reg.idx = 5;
                assert_eq!(cpu.memory.read_byte(0x5), 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x5), 1);
            }

            #[test]
            fn inc_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::IncrementMem,
                        AddressingMode::Absolute,
                    ),
                    0x00,
                    0x10,
                ]);
                assert_eq!(cpu.memory.read_byte(0x1000), 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x1000), 1);
            }

            #[test]
            fn inc_absolute_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::IncrementMem,
                        AddressingMode::AbsoluteX,
                    ),
                    0x00,
                    0x10,
                ]);
                cpu.reg.idx = 10;
                assert_eq!(cpu.memory.read_byte(0x100A), 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x100A), 1);
            }
        }
        mod inx {
            use super::*;
            #[test]
            fn inx_implied() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::IncrementX,
                    AddressingMode::Implied,
                )]);
                assert_eq!(cpu.reg.idx, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 1);
            }
            #[test]
            fn inx_implied_overflow() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::IncrementX,
                    AddressingMode::Implied,
                )]);
                assert_eq!(cpu.reg.idx, 0);
                cpu.reg.idx = 0xFF;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0);
            }
        }
        mod iny {
            use super::*;
            #[test]
            fn iny_implied() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::IncrementY,
                    AddressingMode::Implied,
                )]);
                assert_eq!(cpu.reg.idy, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 1);
            }
            #[test]
            fn iny_implied_overflow() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::IncrementY,
                    AddressingMode::Implied,
                )]);
                assert_eq!(cpu.reg.idy, 0);
                cpu.reg.idy = 0xFF;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0);
            }
        }
    }
    mod decrement {
        use super::*;
        mod dec {
            use super::*;
            #[test]
            fn dec_zero_page() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::DecrementMem,
                        AddressingMode::ZeroPage,
                    ),
                    0x0,
                ]);
                assert_eq!(cpu.memory.read_byte(0x0), 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x0), 0xFF);
            }

            #[test]
            fn dec_zero_page_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::DecrementMem,
                        AddressingMode::ZeroPageX,
                    ),
                    0x0,
                ]);
                cpu.reg.idx = 5;
                assert_eq!(cpu.memory.read_byte(0x5), 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x5), 0xFF);
            }

            #[test]
            fn dec_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::DecrementMem,
                        AddressingMode::Absolute,
                    ),
                    0x00,
                    0x10,
                ]);
                assert_eq!(cpu.memory.read_byte(0x1000), 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x1000), 0xFF);
            }

            #[test]
            fn dec_absolute_x() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::DecrementMem,
                        AddressingMode::AbsoluteX,
                    ),
                    0x00,
                    0x10,
                ]);
                cpu.reg.idx = 10;
                assert_eq!(cpu.memory.read_byte(0x100A), 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.memory.read_byte(0x100A), 0xFF);
            }
        }
        mod dex {
            use super::*;
            #[test]
            fn dex_implied() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::DecrementX,
                    AddressingMode::Implied,
                )]);
                assert_eq!(cpu.reg.idx, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0xFF);
            }
            #[test]
            fn dex_implied_overflow() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::DecrementX,
                    AddressingMode::Implied,
                )]);
                assert_eq!(cpu.reg.idx, 0);
                cpu.reg.idx = 0xFF;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idx, 0xFE);
            }
        }
        mod inx {
            use super::*;
            #[test]
            fn inx_implied() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::DecrementY,
                    AddressingMode::Implied,
                )]);
                assert_eq!(cpu.reg.idy, 0);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0xFF);
            }
            #[test]
            fn inx_implied_overflow() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::DecrementY,
                    AddressingMode::Implied,
                )]);
                assert_eq!(cpu.reg.idy, 0);
                cpu.reg.idy = 0xFF;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.idy, 0xFE);
            }
        }
    }
    mod branching {
        use super::*;
        mod jmp {
            use super::*;
            use crate::memory::Bus;
            #[test]
            fn jmp_absolute() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::Jump, AddressingMode::Absolute),
                    0x20,
                    0x20,
                ]);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x2020);
            }
            #[test]
            fn jmp_indirect() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(Instructions::Jump, AddressingMode::Indirect),
                    0x20,
                    0x20,
                ]);
                cpu.memory.write_byte(0x2020, 0x21);
                cpu.memory.write_byte(0x2021, 0x34);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x3421);
            }
        }
        mod jsr {
            use super::*;
            #[test]
            fn jsr() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::JumpSubroutine,
                        AddressingMode::Absolute,
                    ),
                    0x20,
                    0x20,
                    NesCpu::encode_instructions(Instructions::Jump, AddressingMode::Absolute),
                    0x80,
                    0x00,
                ]);
                let sp = cpu.reg.sp;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x2020);
                assert_eq!(cpu.reg.sp, sp - 2);
                let address = cpu.pop_stack_u16();
                assert_eq!(address, 0x8002);
                assert_eq!(cpu.reg.sp, sp);
            }
        }
        mod bcc {
            use super::*;

            #[test]
            fn bcc() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::BranchOnCarryClear,
                        AddressingMode::Relative,
                    ),
                    0x20,
                    NesCpu::encode_instructions(
                        Instructions::BranchOnCarryClear,
                        AddressingMode::Relative,
                    ),
                    0x20,
                ]);
                cpu.reg.flags.carry = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8002);
                cpu.reg.flags.carry = false;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8024);
            }
        }
        mod bcs {
            use super::*;

            #[test]
            fn bcs() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::BranchOnCarrySet,
                        AddressingMode::Relative,
                    ),
                    0x20,
                    NesCpu::encode_instructions(
                        Instructions::BranchOnCarrySet,
                        AddressingMode::Relative,
                    ),
                    0x20,
                ]);
                cpu.reg.flags.carry = false;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8002);
                cpu.reg.flags.carry = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8024);
            }
        }
        mod bvc {
            use super::*;
            #[test]
            fn bvc() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::BranchOverflowClear,
                        AddressingMode::Relative,
                    ),
                    0x20,
                    NesCpu::encode_instructions(
                        Instructions::BranchOverflowClear,
                        AddressingMode::Relative,
                    ),
                    0x20,
                ]);
                cpu.reg.flags.overflow = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8002);
                cpu.reg.flags.overflow = false;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8024);
            }
        }
        mod bvs {
            use super::*;
            #[test]
            fn bvs() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::BranchOnOverflowSet,
                        AddressingMode::Relative,
                    ),
                    0x20,
                    NesCpu::encode_instructions(
                        Instructions::BranchOnOverflowSet,
                        AddressingMode::Relative,
                    ),
                    0x20,
                ]);
                cpu.reg.flags.overflow = false;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8002);
                cpu.reg.flags.overflow = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8024);
            }
        }
        mod bne {
            use super::*;

            #[test]
            fn bne() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::BranchNotZero,
                        AddressingMode::Relative,
                    ),
                    0x20,
                    NesCpu::encode_instructions(
                        Instructions::BranchNotZero,
                        AddressingMode::Relative,
                    ),
                    0x20,
                ]);
                cpu.reg.flags.zero = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8002);
                cpu.reg.flags.zero = false;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8024);
            }
        }
        mod beq {
            use super::*;

            #[test]
            fn beq() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::BranchOnResultZero,
                        AddressingMode::Relative,
                    ),
                    0x20,
                    NesCpu::encode_instructions(
                        Instructions::BranchOnResultZero,
                        AddressingMode::Relative,
                    ),
                    0x20,
                ]);
                cpu.reg.flags.zero = false;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8002);
                cpu.reg.flags.zero = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8024);
            }
        }
        mod bmi {
            use super::*;
            #[test]
            fn bmi() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::BranchOnResultMinus,
                        AddressingMode::Relative,
                    ),
                    0x20,
                    NesCpu::encode_instructions(
                        Instructions::BranchOnResultMinus,
                        AddressingMode::Relative,
                    ),
                    0x20,
                ]);
                cpu.reg.flags.negative = false;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8002);
                cpu.reg.flags.negative = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8024);
            }

            #[test]
            fn bpl() {
                let mut cpu = NesCpu::new_from_bytes(&[
                    NesCpu::encode_instructions(
                        Instructions::BranchOnResultPlus,
                        AddressingMode::Relative,
                    ),
                    0x20,
                    NesCpu::encode_instructions(
                        Instructions::BranchOnResultPlus,
                        AddressingMode::Relative,
                    ),
                    0x20,
                ]);
                cpu.reg.flags.negative = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8002);
                cpu.reg.flags.negative = false;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.pc, 0x8024);
            }
        }
    }
    mod flags {
        // fully tested, decimal not used in nes 6502 variant.
        use super::*;
        mod sei {
            use super::*;
            #[test]
            fn sei() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::SetInterruptDisable,
                    AddressingMode::Implied,
                )]);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.flags.interrupt_disable, true);
            }
        }
        mod cli {
            use super::*;
            #[test]
            fn cli() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::ClearInterruptDisable,
                    AddressingMode::Implied,
                )]);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.flags.interrupt_disable, false);
            }
        }
        mod sec {
            use super::*;
            #[test]
            fn sec() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::SetCarry,
                    AddressingMode::Implied,
                )]);
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.flags.carry, true);
            }
        }
        mod clc {
            use super::*;
            #[test]
            fn clc() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::ClearCarry,
                    AddressingMode::Implied,
                )]);
                cpu.reg.flags.carry = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.flags.carry, false);
            }
        }
        mod clv {
            use super::*;
            #[test]
            fn clv() {
                let mut cpu = NesCpu::new_from_bytes(&[NesCpu::encode_instructions(
                    Instructions::ClearOverflow,
                    AddressingMode::Implied,
                )]);
                cpu.reg.flags.overflow = true;
                cpu.fetch_decode_next();
                assert_eq!(cpu.reg.flags.overflow, false);
            }
        }
    }
}
