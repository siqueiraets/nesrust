enum InterruptType {
    None,
    Reset,
    Nmi,
    Brk,
    Irq,
}

enum Flags {
    Negative,         // 0x80
    Overflow,         // 0x40
    BFlag1,           // 0x20
    BFlag2,           // 0x10
    Decimal,          // 0x08
    InterruptDisable, // 0x04
    Zero,             // 0x02
    Carry,            // 0x01
}

impl Flags {
    fn to_int(&self) -> u8 {
        match *self {
            Flags::Negative => 0x80,
            Flags::Overflow => 0x40,
            Flags::BFlag1 => 0x20,
            Flags::BFlag2 => 0x10,
            Flags::Decimal => 0x08,
            Flags::InterruptDisable => 0x04,
            Flags::Zero => 0x02,
            Flags::Carry => 0x01,
        }
    }
}

pub enum CycleResult {
    EndCycle,
    EndInstruction,
    Continue,
    Error,
}

#[derive(Copy, Clone)]
enum AddressingMode {
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    AbsoluteIndirect,
    IndirectX,
    IndirectY,
    Relative,
}

#[derive(Copy, Clone)]
enum InstructionType {
    Branching,
    Read,
    ReadModifyWrite,
    Write,
}

pub trait BusOps {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, data: u8);
}

pub struct Cpu6502 {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub pc: u16,
    pub sp: u8,
    pub sr: u8,
    pub value: u8,
    pub address: u16,
    pub stage: u8,

    interrupt_type: InterruptType,
    addressing_mode: AddressingMode,
    instruction_type: InstructionType,
    instruction_pointer: fn(&mut Cpu6502, &mut dyn BusOps) -> CycleResult,
}

impl Cpu6502 {
    pub fn new() -> Self {
        Cpu6502 {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0,
            sr: 0,
            value: 0,
            address: 0,
            stage: 1,
            interrupt_type: InterruptType::None,
            addressing_mode: AddressingMode::Accumulator,
            instruction_type: InstructionType::Read,
            instruction_pointer: Cpu6502::nop,
        }
    }

    pub fn reset(&mut self) {
        self.sp = 0xFD;
        self.sr = 0x24;
        self.interrupt_type = InterruptType::Reset;
    }

    pub fn set_nmi(&mut self) {
        self.interrupt_type = InterruptType::Nmi;
    }

    pub fn tick(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        if self.stage == 1 {
            if let InterruptType::None = self.interrupt_type {
                let opcode = self.read_memory(bus, self.pc);
                self.pc += 1;
                self.stage += 1;
                self.fetch_instruction(opcode);
                return CycleResult::EndCycle;
            } else {
                self.load_interrupt();
            }
            return CycleResult::EndCycle;
        }

        let result = (self.instruction_pointer)(self, bus);
        match result {
            CycleResult::EndInstruction => {
                self.stage = 1;
                result
            }
            CycleResult::EndCycle => {
                self.stage += 1;
                result
            }
            _ => CycleResult::Error,
        }
    }

    fn load_interrupt(&mut self) {
        self.instruction_pointer = match self.interrupt_type {
            InterruptType::Brk => Cpu6502::brk,
            InterruptType::Reset => Cpu6502::rst,
            InterruptType::Nmi => Cpu6502::nmi,
            InterruptType::Irq => Cpu6502::irq,
            InterruptType::None => Cpu6502::nop,
        };

        self.interrupt_type = InterruptType::None;
        self.stage += 1;
    }

    fn read_memory(&mut self, bus: &mut dyn BusOps, memory_address: u16) -> u8 {
        return bus.read(memory_address);
    }

    fn write_memory(&mut self, bus: &mut dyn BusOps, memory_address: u16, data: u8) {
        bus.write(memory_address, data);
    }

    fn read_stack(&mut self, bus: &mut dyn BusOps) -> u8 {
        let stack_base = 0x100;
        let stack_address = stack_base + self.sp as u16;
        return self.read_memory(bus, stack_address);
    }

    fn write_stack(&mut self, bus: &mut dyn BusOps, data: u8) {
        let stack_base = 0x100;
        let stack_address = stack_base + self.sp as u16;
        self.write_memory(bus, stack_address, data);
    }

    fn is_flag_set(&self, flag: Flags) -> bool {
        let flag = flag.to_int();
        (self.sr & flag) != 0
    }

    fn set_flag(&mut self, flag: Flags, enabled: bool) {
        let flag = flag.to_int();
        if enabled {
            self.sr = self.sr | flag;
        } else {
            self.sr = !flag & self.sr;
        }
    }

    fn fetch_instruction(&mut self, opcode: u8) {
        // println!("Fetch instruction {:#02X}", opcode);
        type Instruction = fn(&mut Cpu6502, &mut dyn BusOps) -> CycleResult;
        #[rustfmt::skip]
        let instructions: [(AddressingMode,InstructionType, Instruction);256] = [
            (AddressingMode::Immediate, InstructionType::Branching, Cpu6502::brk),         // 00
            (AddressingMode::IndirectX, InstructionType::Read, Cpu6502::ora),              // 01
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 02
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::slo), // 03
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::nop),               // 04
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::ora),               // 05
            (AddressingMode::ZeroPage, InstructionType::ReadModifyWrite, Cpu6502::asl),    // 06
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::slo), // 07
            (AddressingMode::Accumulator, InstructionType::Write, Cpu6502::php),           // 08
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::ora),              // 09
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::asl), // 0A
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::anc),              // 0B
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::nop),               // 0C
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::ora),               // 0D
            (AddressingMode::Absolute, InstructionType::ReadModifyWrite, Cpu6502::asl),    // 0E
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::slo), // 0F
            (AddressingMode::Relative, InstructionType::Branching, Cpu6502::bpl),          // 10
            (AddressingMode::IndirectY, InstructionType::Read, Cpu6502::ora),              // 11
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 12
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::slo), // 13
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::nop),              // 14
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::ora),              // 15
            (AddressingMode::ZeroPageX, InstructionType::ReadModifyWrite, Cpu6502::asl),   // 16
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::slo), // 17
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::clc),            // 18
            (AddressingMode::AbsoluteY, InstructionType::Read, Cpu6502::ora),              // 19
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::nop),            // 1A
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::slo), // 1B
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::nop),              // 1C
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::ora),              // 1D
            (AddressingMode::AbsoluteX, InstructionType::ReadModifyWrite, Cpu6502::asl),   // 1E
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::slo), // 1F
            (AddressingMode::Absolute, InstructionType::Branching, Cpu6502::jsr),          // 20
            (AddressingMode::IndirectX, InstructionType::Read, Cpu6502::and),              // 21
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 22
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rla), // 23
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::bit),               // 24
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::and),               // 25
            (AddressingMode::ZeroPage, InstructionType::ReadModifyWrite, Cpu6502::rol),    // 26
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rla), // 27
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::plp),            // 28
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::and),              // 29
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rol), // 2A
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::anc),            // 2B
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::bit),               // 2C
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::and),               // 2D
            (AddressingMode::Absolute, InstructionType::ReadModifyWrite, Cpu6502::rol),    // 2E
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rla), // 2F
            (AddressingMode::Relative, InstructionType::Branching, Cpu6502::bmi),          // 30
            (AddressingMode::IndirectY, InstructionType::Read, Cpu6502::and),              // 31
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 32
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rla), // 33
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::nop),              // 34
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::and),              // 35
            (AddressingMode::ZeroPageX, InstructionType::ReadModifyWrite, Cpu6502::rol),   // 36
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rla), // 37
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::sec),            // 38
            (AddressingMode::AbsoluteY, InstructionType::Read, Cpu6502::and),              // 39
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::nop),            // 3A
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rla), // 3B
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::nop),              // 3C
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::and),              // 3D
            (AddressingMode::AbsoluteX, InstructionType::ReadModifyWrite, Cpu6502::rol),   // 3E
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rla), // 3F
            (AddressingMode::Accumulator, InstructionType::Branching, Cpu6502::rti),       // 40
            (AddressingMode::IndirectX, InstructionType::Read, Cpu6502::eor),              // 41
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 42
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::sre), // 43
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::nop),               // 44
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::eor),               // 45
            (AddressingMode::ZeroPage, InstructionType::ReadModifyWrite, Cpu6502::lsr),    // 46
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::sre), // 47
            (AddressingMode::Accumulator, InstructionType::Write, Cpu6502::pha),           // 48
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::eor),              // 49
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::lsr), // 4A
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::alr),            // 4B
            (AddressingMode::Absolute, InstructionType::Branching, Cpu6502::jmp),          // 4C
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::eor),               // 4D
            (AddressingMode::Absolute, InstructionType::ReadModifyWrite, Cpu6502::lsr),    // 4E
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::sre), // 4F
            (AddressingMode::Relative, InstructionType::Branching, Cpu6502::bvc),          // 50
            (AddressingMode::IndirectY, InstructionType::Read, Cpu6502::eor),              // 51
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 52
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::sre), // 53
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::nop),              // 54
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::eor),              // 55
            (AddressingMode::ZeroPageX, InstructionType::ReadModifyWrite, Cpu6502::lsr),   // 56
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::sre), // 57
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::cli),            // 58
            (AddressingMode::AbsoluteY, InstructionType::Read, Cpu6502::eor),              // 59
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::nop),            // 5A
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::sre), // 5B
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::nop),              // 5C
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::eor),              // 5D
            (AddressingMode::AbsoluteX, InstructionType::ReadModifyWrite, Cpu6502::lsr),   // 5E
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::sre), // 5F
            (AddressingMode::Accumulator, InstructionType::Branching, Cpu6502::rts),       // 60
            (AddressingMode::IndirectX, InstructionType::Read, Cpu6502::adc),              // 61
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 62
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rra), // 63
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::nop),               // 64
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::adc),               // 65
            (AddressingMode::ZeroPage, InstructionType::ReadModifyWrite, Cpu6502::ror),    // 66
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rra), // 67
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::pla),            // 68
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::adc),              // 69
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::ror), // 6A
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::arr),            // 6B
            (AddressingMode::AbsoluteIndirect, InstructionType::Branching, Cpu6502::jmp),  // 6C
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::adc),               // 6D
            (AddressingMode::Absolute, InstructionType::ReadModifyWrite, Cpu6502::ror),    // 6E
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rra), // 6F
            (AddressingMode::Relative, InstructionType::Branching, Cpu6502::bvs),          // 70
            (AddressingMode::IndirectY, InstructionType::Read, Cpu6502::adc),              // 71
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 72
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rra), // 73
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::nop),              // 74
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::adc),              // 75
            (AddressingMode::ZeroPageX, InstructionType::ReadModifyWrite, Cpu6502::ror),   // 76
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rra), // 77
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::sei),            // 78
            (AddressingMode::AbsoluteY, InstructionType::Read, Cpu6502::adc),              // 79
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::nop),            // 7A
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rra), // 7B
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::nop),              // 7C
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::adc),              // 7D
            (AddressingMode::AbsoluteX, InstructionType::ReadModifyWrite, Cpu6502::ror),   // 7E
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::rra), // 7F
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::nop),              // 80
            (AddressingMode::IndirectX, InstructionType::Write, Cpu6502::sta),             // 81
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::nop),              // 82
            (AddressingMode::Accumulator, InstructionType::Write, Cpu6502::sax),           // 83
            (AddressingMode::ZeroPage, InstructionType::Write, Cpu6502::sty),              // 84
            (AddressingMode::ZeroPage, InstructionType::Write, Cpu6502::sta),              // 85
            (AddressingMode::ZeroPage, InstructionType::Write, Cpu6502::stx),              // 86
            (AddressingMode::Accumulator, InstructionType::Write, Cpu6502::sax),           // 87
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::dey),            // 88
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::nop),              // 89
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::txa),            // 8A
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::xaa),            // 8B
            (AddressingMode::Absolute, InstructionType::Write, Cpu6502::sty),              // 8C
            (AddressingMode::Absolute, InstructionType::Write, Cpu6502::sta),              // 8D
            (AddressingMode::Absolute, InstructionType::Write, Cpu6502::stx),              // 8E
            (AddressingMode::Accumulator, InstructionType::Write, Cpu6502::sax),           // 8F
            (AddressingMode::Relative, InstructionType::Branching, Cpu6502::bcc),          // 90
            (AddressingMode::IndirectY, InstructionType::Write, Cpu6502::sta),             // 91
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // 92
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::ahx),            // 93
            (AddressingMode::ZeroPageX, InstructionType::Write, Cpu6502::sty),             // 94
            (AddressingMode::ZeroPageX, InstructionType::Write, Cpu6502::sta),             // 95
            (AddressingMode::ZeroPageY, InstructionType::Write, Cpu6502::stx),             // 96
            (AddressingMode::Accumulator, InstructionType::Write, Cpu6502::sax),           // 97
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::tya),            // 98
            (AddressingMode::AbsoluteY, InstructionType::Write, Cpu6502::sta),             // 99
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::txs),            // 9A
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::tas),            // 9B
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::shy),            // 9C
            (AddressingMode::AbsoluteX, InstructionType::Write, Cpu6502::sta),             // 9D
            (AddressingMode::Accumulator, InstructionType::Write, Cpu6502::shx),           // 9E
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::ahx),            // 9F
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::ldy),              // A0
            (AddressingMode::IndirectX, InstructionType::Read, Cpu6502::lda),              // A1
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::ldx),              // A2
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::lax),            // A3
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::ldy),               // A4
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::lda),               // A5
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::ldx),               // A6
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::lax),            // A7
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::tay),            // A8
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::lda),              // A9
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::tax),            // AA
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::lax),            // AB
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::ldy),               // AC
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::lda),               // AD
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::ldx),               // AE
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::lax),            // AF
            (AddressingMode::Relative, InstructionType::Branching, Cpu6502::bcs),          // B0
            (AddressingMode::IndirectY, InstructionType::Read, Cpu6502::lda),              // B1
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // B2
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::lax),            // B3
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::ldy),              // B4
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::lda),              // B5
            (AddressingMode::ZeroPageY, InstructionType::Read, Cpu6502::ldx),              // B6
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::lax),            // B7
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::clv),            // B8
            (AddressingMode::AbsoluteY, InstructionType::Read, Cpu6502::lda),              // B9
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::tsx),            // BA
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::las),            // BB
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::ldy),              // BC
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::lda),              // BD
            (AddressingMode::AbsoluteY, InstructionType::Read, Cpu6502::ldx),              // BE
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::lax),            // BF
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::cpy),              // C0
            (AddressingMode::IndirectX, InstructionType::Read, Cpu6502::cmp),              // C1
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::nop),              // C2
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::dcp), // C3
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::cpy),               // C4
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::cmp),               // C5
            (AddressingMode::ZeroPage, InstructionType::ReadModifyWrite, Cpu6502::dec),    // C6
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::dcp), // C7
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::iny),            // C8
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::cmp),              // C9
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::dex),            // CA
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::axs),            // CB
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::cpy),               // CC
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::cmp),               // CD
            (AddressingMode::Absolute, InstructionType::ReadModifyWrite, Cpu6502::dec),    // CE
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::dcp), // CF
            (AddressingMode::Relative, InstructionType::Branching, Cpu6502::bne),          // D0
            (AddressingMode::IndirectY, InstructionType::Read, Cpu6502::cmp),              // D1
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // D2
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::dcp), // D3
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::nop),              // D4
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::cmp),              // D5
            (AddressingMode::ZeroPageX, InstructionType::ReadModifyWrite, Cpu6502::dec),   // D6
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::dcp), // D7
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::cld),            // D8
            (AddressingMode::AbsoluteY, InstructionType::Read, Cpu6502::cmp),              // D9
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::nop),            // DA
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::dcp), // DB
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::nop),              // DC
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::cmp),              // DD
            (AddressingMode::AbsoluteX, InstructionType::ReadModifyWrite, Cpu6502::dec),   // DE
            (AddressingMode::Accumulator, InstructionType::ReadModifyWrite, Cpu6502::dcp), // DF
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::cpx),              // E0
            (AddressingMode::IndirectX, InstructionType::Read, Cpu6502::sbc),              // E1
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::nop),              // E2
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::isc),            // E3
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::cpx),               // E4
            (AddressingMode::ZeroPage, InstructionType::Read, Cpu6502::sbc),               // E5
            (AddressingMode::ZeroPage, InstructionType::ReadModifyWrite, Cpu6502::inc),    // E6
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::isc),            // E7
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::inx),            // E8
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::sbc),              // E9
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::nop),            // EA
            (AddressingMode::Immediate, InstructionType::Read, Cpu6502::sbc),            // EB
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::cpx),               // EC
            (AddressingMode::Absolute, InstructionType::Read, Cpu6502::sbc),               // ED
            (AddressingMode::Absolute, InstructionType::ReadModifyWrite, Cpu6502::inc),    // EE
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::isc),            // EF
            (AddressingMode::Relative, InstructionType::Branching, Cpu6502::beq),          // F0
            (AddressingMode::IndirectY, InstructionType::Read, Cpu6502::sbc),              // F1
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::stp),            // F2
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::isc),            // F3
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::nop),              // F4
            (AddressingMode::ZeroPageX, InstructionType::Read, Cpu6502::sbc),              // F5
            (AddressingMode::ZeroPageX, InstructionType::ReadModifyWrite, Cpu6502::inc),   // F6
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::isc),            // F7
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::sed),            // F8
            (AddressingMode::AbsoluteY, InstructionType::Read, Cpu6502::sbc),              // F9
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::nop),            // FA
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::isc),            // FB
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::slo),              // FC
            (AddressingMode::AbsoluteX, InstructionType::Read, Cpu6502::sbc),              // FD
            (AddressingMode::AbsoluteX, InstructionType::ReadModifyWrite, Cpu6502::inc),   // FE
            (AddressingMode::Accumulator, InstructionType::Read, Cpu6502::isc)             // FF
        ];

        let (addressing_mode, instruction_type, instruction_pointer) =
            &instructions[opcode as usize];
        self.addressing_mode = *addressing_mode;
        self.instruction_type = *instruction_type;
        self.instruction_pointer = *instruction_pointer;
    }

    fn resolve_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.addressing_mode {
            AddressingMode::Immediate => self.immediate_addressing(bus),
            AddressingMode::Accumulator => self.accumulator_addressing(bus),
            AddressingMode::Absolute => self.absolute_addressing(bus),
            AddressingMode::AbsoluteX => self.absolute_indexed_addressing(bus, self.x),
            AddressingMode::AbsoluteY => self.absolute_indexed_addressing(bus, self.y),
            AddressingMode::AbsoluteIndirect => self.absolute_indirect_addressing(bus),
            AddressingMode::ZeroPage => self.zero_page_addressing(bus),
            AddressingMode::ZeroPageX => self.zero_page_indexed_addressing(bus, self.x),
            AddressingMode::ZeroPageY => self.zero_page_indexed_addressing(bus, self.y),
            AddressingMode::IndirectX => self.indirectx_addressing(bus),
            AddressingMode::IndirectY => self.indirecty_addressing(bus),
            AddressingMode::Relative => self.relative_addressing(bus),
        }
    }

    fn immediate_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.value = self.read_memory(bus, self.pc);
                self.pc += 1;
                CycleResult::Continue
            }
            _ => CycleResult::Error,
        }
    }

    fn accumulator_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::Continue
            }
            _ => CycleResult::Error,
        }
    }

    fn absolute_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                let address_high = self.address & 0xFF00;
                let address_low = self.read_memory(bus, self.pc) as u16;
                self.address = address_high | address_low;
                self.pc += 1;
                return CycleResult::EndCycle;
            }
            3 => {
                let address_high = (self.read_memory(bus, self.pc) as u16) << 8;
                let address_low = self.address & 0xFF;
                self.address = address_high | address_low;
                self.pc += 1;
                match self.instruction_type {
                    InstructionType::Branching => CycleResult::Continue,
                    _ => CycleResult::EndCycle,
                }
            }
            4 => match self.instruction_type {
                InstructionType::Read => {
                    self.value = self.read_memory(bus, self.address);
                    CycleResult::Continue
                }
                InstructionType::ReadModifyWrite => {
                    self.value = self.read_memory(bus, self.address);
                    CycleResult::EndCycle
                }
                InstructionType::Write => CycleResult::Continue,
                _ => CycleResult::Error,
            },
            5 => match self.instruction_type {
                InstructionType::ReadModifyWrite => {
                    self.write_memory(bus, self.address, self.value);
                    CycleResult::EndCycle
                }
                _ => CycleResult::Error,
            },
            6 => match self.instruction_type {
                InstructionType::ReadModifyWrite => CycleResult::Continue,
                _ => CycleResult::Error,
            },

            _ => CycleResult::Error,
        }
    }

    fn absolute_indirect_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                let address_high = self.address & 0xFF00;
                let address_low = self.read_memory(bus, self.pc) as u16;
                self.address = address_high | address_low;
                self.pc += 1;
                CycleResult::EndCycle
            }
            3 => {
                let address_high = (self.read_memory(bus, self.pc) as u16) << 8;
                let address_low = self.address & 0xFF;
                self.address = address_high | address_low;
                self.pc += 1;
                CycleResult::EndCycle
            }
            4 => {
                // Fetch low address to latch
                self.value = self.read_memory(bus, self.address);
                CycleResult::EndCycle
            }
            5 => {
                let address_high = self.address & 0xFF00;
                let address_low = (self.address + 1) & 0xFF;
                self.address = address_high | address_low;

                let cpu_new_address_high = (self.read_memory(bus, self.address) as u16) << 8;
                self.address = cpu_new_address_high | self.value as u16;
                CycleResult::Continue
            }
            _ => return CycleResult::Error,
        }
    }

    fn absolute_indexed_addressing(&mut self, bus: &mut dyn BusOps, index: u8) -> CycleResult {
        match self.stage {
            2 => {
                let address_high = self.address & 0xFF00;
                let address_low = self.read_memory(bus, self.pc) as u16;
                self.address = address_high | address_low;
                self.pc += 1;
                CycleResult::EndCycle
            }
            3 => {
                let address_high = (self.read_memory(bus, self.pc) as u16) << 8;
                let address_low = self.address & 0xFF;
                self.address = address_high | address_low;
                self.pc += 1;
                CycleResult::EndCycle
            }
            4 => {
                let address_high = self.address & 0xFF00;
                let address_low = (self.address.wrapping_add(index as u16)) & 0xFF;
                let address = address_high | address_low;
                self.value = self.read_memory(bus, address);

                if let InstructionType::Read = self.instruction_type {
                    // Check for page boundary
                    if ((self.address & 0xFF).wrapping_add(index as u16)) <= 0xFF {
                        // Boundary not crossed, continue to instruction
                        self.address = self.address.wrapping_add(index as u16);
                        return CycleResult::Continue;
                    }
                }

                self.address = self.address.wrapping_add(index as u16);
                CycleResult::EndCycle
            }
            5 => {
                match self.instruction_type {
                    InstructionType::Read => {
                        // Should arrive here when boundary was crossed
                        self.value = self.read_memory(bus, self.address);
                        CycleResult::Continue
                    }
                    InstructionType::ReadModifyWrite => {
                        self.value = self.read_memory(bus, self.address);
                        CycleResult::EndCycle
                    }
                    InstructionType::Write => CycleResult::Continue,
                    _ => CycleResult::Error,
                }
            }
            6 => match self.instruction_type {
                InstructionType::ReadModifyWrite => {
                    self.write_memory(bus, self.address, self.value);
                    CycleResult::EndCycle
                }
                _ => CycleResult::Error,
            },
            7 => match self.instruction_type {
                InstructionType::ReadModifyWrite => {
                    return CycleResult::Continue;
                }
                _ => CycleResult::Error,
            },
            _ => CycleResult::Error,
        }
    }

    fn zero_page_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.address = self.read_memory(bus, self.pc) as u16;
                self.pc += 1;
                CycleResult::EndCycle
            }
            3 => match self.instruction_type {
                InstructionType::Read => {
                    self.value = self.read_memory(bus, self.address);
                    CycleResult::Continue
                }
                InstructionType::ReadModifyWrite => {
                    self.value = self.read_memory(bus, self.address);
                    CycleResult::EndCycle
                }
                InstructionType::Write => CycleResult::Continue,
                _ => CycleResult::Error,
            },
            4 => match self.instruction_type {
                InstructionType::ReadModifyWrite => {
                    self.write_memory(bus, self.address, self.value);
                    return CycleResult::EndCycle;
                }
                _ => CycleResult::Error,
            },
            5 => match self.instruction_type {
                InstructionType::ReadModifyWrite => CycleResult::Continue,
                _ => CycleResult::Error,
            },
            _ => CycleResult::Error,
        }
    }

    fn zero_page_indexed_addressing(&mut self, bus: &mut dyn BusOps, index: u8) -> CycleResult {
        match self.stage {
            2 => {
                self.address = self.read_memory(bus, self.pc) as u16;
                self.pc += 1;
                CycleResult::EndCycle
            }
            3 => {
                self.value = self.read_memory(bus, self.address);
                self.address = self.address + index as u16;
                self.address = self.address & 0xFF;
                CycleResult::EndCycle
            }
            4 => match self.instruction_type {
                InstructionType::Read => {
                    self.value = self.read_memory(bus, self.address);
                    CycleResult::Continue
                }
                InstructionType::ReadModifyWrite => {
                    self.value = self.read_memory(bus, self.address);
                    CycleResult::EndCycle
                }
                InstructionType::Write => CycleResult::Continue,
                _ => CycleResult::Error,
            },
            5 => match self.instruction_type {
                InstructionType::ReadModifyWrite => {
                    self.write_memory(bus, self.address, self.value);
                    return CycleResult::EndCycle;
                }
                _ => CycleResult::Error,
            },
            6 => match self.instruction_type {
                InstructionType::ReadModifyWrite => CycleResult::Continue,
                _ => CycleResult::Error,
            },
            _ => CycleResult::Error,
        }
    }

    fn relative_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.value = self.read_memory(bus, self.pc);
                self.pc += 1;
                CycleResult::Continue
            }
            3 => {
                // Check for page boundary
                // TODO check these conversions
                let signed_value = self.value as i8;
                let relative_address = (self.pc as i16 + signed_value as i16) as u16;
                let old_address_high = self.pc & 0xFF00;
                let new_address_high = relative_address & 0xFF00;
                if old_address_high != new_address_high {
                    // Boundary crossed, wait for the next cycle
                    return CycleResult::EndCycle;
                }

                self.pc = relative_address;
                CycleResult::EndInstruction
            }
            4 => {
                let signed_value = self.value as i8;
                let relative_address = (self.pc as i16 + signed_value as i16) as u16;

                self.pc = relative_address;
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn indirectx_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.address = self.read_memory(bus, self.pc) as u16;
                self.pc += 1;
                CycleResult::EndCycle
            }
            3 => {
                self.read_memory(bus, self.address);
                self.address = self.address.wrapping_add(self.x as u16);
                self.address = self.address & 0xFF;
                CycleResult::EndCycle
            }
            4 => {
                // Fetch effective address low
                self.value = self.read_memory(bus, self.address);
                CycleResult::EndCycle
            }
            5 => {
                let next_address = self.address.wrapping_add(1) & 0xFF;
                let new_address_high = (self.read_memory(bus, next_address) as u16) << 8;

                self.address = new_address_high | self.value as u16;
                CycleResult::EndCycle
            }
            6 => match self.instruction_type {
                InstructionType::Read => {
                    self.value = self.read_memory(bus, self.address);
                    CycleResult::Continue
                }
                InstructionType::ReadModifyWrite => {
                    self.value = self.read_memory(bus, self.address);
                    CycleResult::EndCycle
                }
                InstructionType::Write => CycleResult::Continue,
                _ => CycleResult::Error,
            },
            7 => match self.instruction_type {
                InstructionType::ReadModifyWrite => {
                    self.write_memory(bus, self.address, self.value);
                    CycleResult::EndCycle
                }
                _ => CycleResult::Error,
            },
            8 => match self.instruction_type {
                InstructionType::ReadModifyWrite => CycleResult::Continue,
                _ => CycleResult::Error,
            },
            _ => CycleResult::Error,
        }
    }

    fn indirecty_addressing(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.address = self.read_memory(bus, self.pc) as u16;
                self.pc += 1;
                CycleResult::EndCycle
            }
            3 => {
                // Fetch effective address low
                self.value = self.read_memory(bus, self.address);
                CycleResult::EndCycle
            }
            4 => {
                let next_address = self.address.wrapping_add(1) & 0xFF;
                let new_address_high = (self.read_memory(bus, next_address) as u16) << 8;

                self.address = new_address_high | self.value as u16;
                return CycleResult::EndCycle;
            }
            5 => {
                let address_high = self.address & 0xFF00;
                let address_low = (self.address.wrapping_add(self.y as u16)) & 0xFF;
                let address = address_high | address_low;

                self.value = self.read_memory(bus, address);

                let new_address = self.address.wrapping_add(self.y as u16);
                // Check for page boundary

                if let InstructionType::Read = self.instruction_type {
                    // Check for page boundary
                    if ((self.address & 0xFF) + self.y as u16) <= 0xFF {
                        // Boundary not crossed, continue to instruction
                        self.address = new_address;
                        return CycleResult::Continue;
                    }
                }

                self.address = new_address;
                return CycleResult::EndCycle;
            }
            6 => {
                match self.instruction_type {
                    InstructionType::Read => {
                        // Should arrive here when boundary was crossed
                        self.value = self.read_memory(bus, self.address);
                        CycleResult::Continue
                    }
                    InstructionType::ReadModifyWrite => {
                        self.value = self.read_memory(bus, self.address);
                        CycleResult::EndCycle
                    }
                    InstructionType::Write => CycleResult::Continue,
                    _ => CycleResult::Error,
                }
            }
            7 => match self.instruction_type {
                InstructionType::ReadModifyWrite => {
                    self.write_memory(bus, self.address, self.value);
                    CycleResult::EndCycle
                }
                _ => CycleResult::Error,
            },
            8 => match self.instruction_type {
                InstructionType::ReadModifyWrite => CycleResult::Continue,
                _ => CycleResult::Error,
            },
            _ => CycleResult::Error,
        }
    }

    fn irq(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::EndCycle
            }
            3 => {
                let stack_value = (self.pc >> 8) as u8;
                self.write_stack(bus, stack_value);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndCycle
            }
            4 => {
                let stack_value = (self.pc & 0xFF) as u8;
                self.write_stack(bus, stack_value);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndCycle
            }
            5 => {
                let mut flags_data = self.sr;
                flags_data |= !Flags::BFlag1.to_int();
                flags_data &= !Flags::BFlag2.to_int();
                self.write_stack(bus, flags_data);
                self.sp = self.sp.wrapping_sub(1);
                self.set_flag(Flags::InterruptDisable, true);
                CycleResult::EndCycle
            }
            6 => {
                let address_high = self.pc & 0xFF00;
                let address_low = self.read_memory(bus, 0xFFFE) as u16;

                self.pc = address_high | address_low;
                CycleResult::EndCycle
            }
            7 => {
                let address_high = (self.read_memory(bus, 0xFFFF) as u16) << 8;
                let address_low = self.pc & 0xFF;

                self.pc = address_high | address_low;
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn rst(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            1 => CycleResult::EndCycle,
            2 => CycleResult::EndCycle,
            3 => CycleResult::EndCycle,
            4 => CycleResult::EndCycle,
            5 => CycleResult::EndCycle,
            6 => {
                let address_high = self.pc & 0xFF00;
                let address_low = self.read_memory(bus, 0xFFFC);

                self.pc = address_high | address_low as u16;
                CycleResult::EndCycle
            }
            7 => {
                let address_high = (self.read_memory(bus, 0xFFFD) as u16) << 8;
                let address_low = self.pc & 0xFF;

                self.pc = address_high | address_low;
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn brk(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                self.pc += 1;
                CycleResult::EndCycle
            }
            3 => {
                let stack_value = (self.pc >> 8) as u8;
                self.write_stack(bus, stack_value);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndCycle
            }
            4 => {
                let stack_value = (self.pc & 0xFF) as u8;
                self.write_stack(bus, stack_value);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndCycle
            }
            5 => {
                let stack_value = self.sr | Flags::BFlag1.to_int() | Flags::BFlag2.to_int();
                self.write_stack(bus, stack_value);
                self.sp = self.sp.wrapping_sub(1);
                self.set_flag(Flags::InterruptDisable, true);
                CycleResult::EndCycle
            }
            6 => {
                let address_high = self.pc & 0xFF00;
                let address_low = self.read_memory(bus, 0xFFFE) as u16;
                self.pc = address_high | address_low;
                CycleResult::EndCycle
            }
            7 => {
                let address_high = (self.read_memory(bus, 0xFFFF) as u16) << 8;
                let address_low = self.pc & 0xFF;
                self.pc = address_high | address_low;
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn nmi(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::EndCycle
            }
            3 => {
                self.write_stack(bus, (self.pc >> 8) as u8);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndCycle
            }
            4 => {
                self.write_stack(bus, (self.pc & 0xFF) as u8);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndCycle
            }
            5 => {
                let mut flags_data = self.sr;
                flags_data |= Flags::BFlag1.to_int();
                flags_data &= !Flags::BFlag2.to_int();
                self.write_stack(bus, flags_data);
                self.sp = self.sp.wrapping_sub(1);
                self.set_flag(Flags::InterruptDisable, true);
                CycleResult::EndCycle
            }
            6 => {
                let address_high = self.pc & 0xFF00;
                let address_low = self.read_memory(bus, 0xFFFA);
                self.pc = address_high | address_low as u16;
                CycleResult::EndCycle
            }
            7 => {
                let address_high = (self.read_memory(bus, 0xFFFB) as u16) << 8;
                let address_low = self.pc & 0xFF;
                self.pc = address_high | address_low;
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn rti(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::EndCycle
            }
            3 => {
                self.sp = self.sp.wrapping_add(1);
                CycleResult::EndCycle
            }
            4 => {
                self.sr = self.read_stack(bus);
                self.set_flag(Flags::BFlag1, true);
                self.set_flag(Flags::BFlag2, false);
                self.sp = self.sp.wrapping_add(1);
                CycleResult::EndCycle
            }
            5 => {
                let address_high = self.pc & 0xFF00;
                let address_low = self.read_stack(bus) as u16;
                self.pc = address_high | address_low;
                self.sp = self.sp.wrapping_add(1);
                CycleResult::EndCycle
            }
            6 => {
                let address_high = (self.read_stack(bus) as u16) << 8;
                let address_low = self.pc & 0xFF;
                self.pc = address_high | address_low;
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn rts(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::EndCycle
            }
            3 => {
                self.sp = self.sp.wrapping_add(1);
                CycleResult::EndCycle
            }
            4 => {
                let address_high = self.pc & 0xFF00;
                let address_low = self.read_stack(bus) as u16;
                self.pc = address_high | address_low;
                self.sp = self.sp.wrapping_add(1);
                return CycleResult::EndCycle;
            }
            5 => {
                let address_high = (self.read_stack(bus) as u16) << 8;
                let address_low = self.pc & 0xFF;
                self.pc = address_high | address_low;
                CycleResult::EndCycle
            }
            6 => {
                self.pc = self.pc + 1;
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn pha(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::EndCycle
            }
            3 => {
                self.write_stack(bus, self.a);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn php(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::EndCycle
            }
            3 => {
                let mut value = self.sr;
                value |= Flags::BFlag1.to_int();
                value |= Flags::BFlag2.to_int();
                self.write_stack(bus, value);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn pla(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::EndCycle
            }
            3 => {
                self.sp = self.sp.wrapping_add(1);
                CycleResult::EndCycle
            }
            4 => {
                self.a = self.read_stack(bus);
                self.set_flag(Flags::Zero, self.a == 0);
                self.set_flag(Flags::Negative, (self.a & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn plp(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                self.read_memory(bus, self.pc);
                CycleResult::EndCycle
            }
            3 => {
                self.sp = self.sp.wrapping_add(1);
                CycleResult::EndCycle
            }
            4 => {
                self.sr = self.read_stack(bus);
                self.set_flag(Flags::BFlag1, true);
                self.set_flag(Flags::BFlag2, false);
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn jsr(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        match self.stage {
            2 => {
                let address_high = self.address & 0xFF00;
                let address_low = self.read_memory(bus, self.pc) as u16;
                self.address = address_high | address_low;
                self.pc += 1;
                CycleResult::EndCycle
            }
            3 => CycleResult::EndCycle,
            4 => {
                let stack_value = (self.pc >> 8) as u8;
                self.write_stack(bus, stack_value);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndCycle
            }
            5 => {
                let stack_value = (self.pc & 0xFF) as u8;
                self.write_stack(bus, stack_value);
                self.sp = self.sp.wrapping_sub(1);
                CycleResult::EndCycle
            }
            6 => {
                let address_high = (self.read_memory(bus, self.pc) as u16) << 8;
                let address_low = self.address & 0xFF;
                self.address = address_high | address_low;
                self.pc = self.address;
                CycleResult::EndInstruction
            }
            _ => CycleResult::Error,
        }
    }

    fn adc(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let carry: u8 = if self.is_flag_set(Flags::Carry) { 1 } else { 0 };
                let (added_value, overflow_add) = self.a.overflowing_add(self.value);
                let (added_carry, overflow_carry) = added_value.overflowing_add(carry);
                let same_signal = (self.a & 0x80) == (self.value & 0x80);
                let sign = self.value & 0x80;
                let total_value = (self.a as u16) + (self.value as u16) + (carry as u16);
                let new_carry = (total_value & 0x100) != 0;
                self.set_flag(Flags::Carry, new_carry);
                self.a = added_carry;
                self.set_flag(Flags::Zero, self.a == 0);
                self.set_flag(Flags::Overflow, same_signal && (sign != (self.a & 0x80)));
                self.set_flag(Flags::Negative, (self.a & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn and(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.a = self.a & self.value;

                self.set_flag(Flags::Zero, self.a == 0);
                self.set_flag(Flags::Negative, (self.a & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn asl(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let (value, old_value, accumulator) = match self.addressing_mode {
                    AddressingMode::Accumulator => {
                        let old_value = self.a;
                        self.a <<= 1;
                        (self.a, old_value, true)
                    }
                    _ => {
                        let old_value = self.value;
                        self.value <<= 1;
                        (self.value, old_value, false)
                    }
                };

                if !accumulator {
                    self.write_memory(bus, self.address, value);
                }

                self.set_flag(Flags::Carry, (old_value & 0x80) != 0);
                self.set_flag(Flags::Zero, value == 0);
                self.set_flag(Flags::Negative, (value & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn bcc(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                if !self.is_flag_set(Flags::Carry) {
                    CycleResult::EndCycle
                } else {
                    CycleResult::EndInstruction
                }
            }
            _ => result,
        }
    }

    fn bcs(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                if self.is_flag_set(Flags::Carry) {
                    CycleResult::EndCycle
                } else {
                    CycleResult::EndInstruction
                }
            }
            _ => result,
        }
    }

    fn beq(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                if self.is_flag_set(Flags::Zero) {
                    CycleResult::EndCycle
                } else {
                    CycleResult::EndInstruction
                }
            }
            _ => result,
        }
    }

    fn bit(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let cmp = self.a & self.value;

                self.set_flag(Flags::Zero, cmp == 0);
                self.set_flag(Flags::Overflow, (self.value & 0x40) != 0);
                self.set_flag(Flags::Negative, (self.value & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn bmi(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                if self.is_flag_set(Flags::Negative) {
                    CycleResult::EndCycle
                } else {
                    CycleResult::EndInstruction
                }
            }
            _ => result,
        }
    }

    fn bne(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                if !self.is_flag_set(Flags::Zero) {
                    return CycleResult::EndCycle;
                } else {
                    CycleResult::EndInstruction
                }
            }
            _ => result,
        }
    }

    fn bpl(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                if !self.is_flag_set(Flags::Negative) {
                    CycleResult::EndCycle
                } else {
                    CycleResult::EndInstruction
                }
            }
            _ => result,
        }
    }

    fn bvc(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                if !self.is_flag_set(Flags::Overflow) {
                    CycleResult::EndCycle
                } else {
                    CycleResult::EndInstruction
                }
            }
            _ => result,
        }
    }

    fn bvs(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                if self.is_flag_set(Flags::Overflow) {
                    CycleResult::EndCycle
                } else {
                    CycleResult::EndInstruction
                }
            }
            _ => result,
        }
    }

    fn clc(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.set_flag(Flags::Carry, false);
        CycleResult::EndInstruction
    }

    fn cld(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.set_flag(Flags::Decimal, false);
        CycleResult::EndInstruction
    }

    fn cli(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.set_flag(Flags::InterruptDisable, false);
        CycleResult::EndInstruction
    }

    fn clv(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.set_flag(Flags::Overflow, false);
        CycleResult::EndInstruction
    }

    fn cmp(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let (cmp, overflow) = self.a.overflowing_sub(self.value);
                self.set_flag(Flags::Carry, self.a >= self.value);
                self.set_flag(Flags::Zero, self.a == self.value);
                self.set_flag(Flags::Negative, (cmp & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn cpx(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let (cmp, overflow) = self.x.overflowing_sub(self.value);
                self.set_flag(Flags::Carry, self.x >= self.value);
                self.set_flag(Flags::Zero, self.x == self.value);
                self.set_flag(Flags::Negative, (cmp & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn cpy(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let (cmp, overflow) = self.y.overflowing_sub(self.value);
                self.set_flag(Flags::Carry, self.y >= self.value);
                self.set_flag(Flags::Zero, self.y == self.value);
                self.set_flag(Flags::Negative, (cmp & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn dec(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.value = self.value.wrapping_sub(1);
                self.write_memory(bus, self.address, self.value);
                self.set_flag(Flags::Zero, self.value == 0);
                self.set_flag(Flags::Negative, (self.value & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn dex(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.x = self.x.wrapping_sub(1);

        self.set_flag(Flags::Zero, self.x == 0);
        self.set_flag(Flags::Negative, (self.x & 0x80) != 0);

        CycleResult::EndInstruction
    }

    fn dey(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.y = self.y.wrapping_sub(1);

        self.set_flag(Flags::Zero, self.y == 0);
        self.set_flag(Flags::Negative, (self.y & 0x80) != 0);

        CycleResult::EndInstruction
    }

    fn eor(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.a = self.a ^ self.value;

                self.set_flag(Flags::Zero, self.a == 0);
                self.set_flag(Flags::Negative, (self.a & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn inc(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.value = self.value.wrapping_add(1);
                self.write_memory(bus, self.address, self.value);
                self.set_flag(Flags::Zero, self.value == 0);
                self.set_flag(Flags::Negative, (self.value & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn inx(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.x = self.x.wrapping_add(1);

        self.set_flag(Flags::Zero, self.x == 0);
        self.set_flag(Flags::Negative, (self.x & 0x80) != 0);

        return CycleResult::EndInstruction;
    }

    fn iny(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.y = self.y.wrapping_add(1);

        self.set_flag(Flags::Zero, self.y == 0);
        self.set_flag(Flags::Negative, (self.y & 0x80) != 0);

        return CycleResult::EndInstruction;
    }

    fn jmp(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.pc = self.address;

                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn lda(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.a = self.value;

                self.set_flag(Flags::Zero, self.a == 0);
                self.set_flag(Flags::Negative, (self.a & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn ldx(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.x = self.value;

                self.set_flag(Flags::Zero, self.x == 0);
                self.set_flag(Flags::Negative, (self.x & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn ldy(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.y = self.value;

                self.set_flag(Flags::Zero, self.y == 0);
                self.set_flag(Flags::Negative, (self.y & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn lsr(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let (value, old_value, accumulator) = match self.addressing_mode {
                    AddressingMode::Accumulator => {
                        let old_value = self.a;
                        self.a >>= 1;
                        (self.a, old_value, true)
                    }
                    _ => {
                        let old_value = self.value;
                        self.value >>= 1;
                        (self.value, old_value, false)
                    }
                };

                if !accumulator {
                    self.write_memory(bus, self.address, value);
                }

                self.set_flag(Flags::Carry, (old_value & 0x01) != 0);
                self.set_flag(Flags::Zero, value == 0);
                self.set_flag(Flags::Negative, (value & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn nop(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => CycleResult::EndInstruction,
            _ => result,
        }
    }

    fn ora(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.a = self.a | self.value;

                self.set_flag(Flags::Zero, self.a == 0);
                self.set_flag(Flags::Negative, (self.a & 0x80) != 0);

                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn rol(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let carry_set = self.is_flag_set(Flags::Carry);
                let (value, old_value, accumulator) = match self.addressing_mode {
                    AddressingMode::Accumulator => {
                        let old_value = self.a;
                        self.a <<= 1;
                        self.a |= if carry_set { 1 } else { 0 };
                        (self.a, old_value, true)
                    }
                    _ => {
                        let old_value = self.value;
                        self.value <<= 1;
                        self.value |= if carry_set { 1 } else { 0 };
                        (self.value, old_value, false)
                    }
                };

                if !accumulator {
                    self.write_memory(bus, self.address, value);
                }

                self.set_flag(Flags::Carry, (old_value & 0x80) != 0);
                self.set_flag(Flags::Zero, value == 0);
                self.set_flag(Flags::Negative, (value & 0x80) != 0);

                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn ror(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let carry_set = self.is_flag_set(Flags::Carry);
                let (value, old_value, accumulator) = match self.addressing_mode {
                    AddressingMode::Accumulator => {
                        let old_value = self.a;
                        self.a >>= 1;
                        self.a |= if carry_set { 0x80 } else { 0 };
                        (self.a, old_value, true)
                    }
                    _ => {
                        let old_value = self.value;
                        self.value >>= 1;
                        self.value |= if carry_set { 0x80 } else { 0 };
                        (self.value, old_value, false)
                    }
                };
                if !accumulator {
                    self.write_memory(bus, self.address, value);
                }

                self.set_flag(Flags::Carry, (old_value & 0x01) != 0);
                self.set_flag(Flags::Zero, value == 0);
                self.set_flag(Flags::Negative, (value & 0x80) != 0);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn sbc(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                let not_value = !self.value;
                let carry: u8 = if self.is_flag_set(Flags::Carry) { 1 } else { 0 };
                let (added_not_value, overflow_add) = self.a.overflowing_add(not_value);
                let (added_carry, overflow_carry) = added_not_value.overflowing_add(carry);
                let signal = not_value;
                let same_signal = (self.a & 0x80) == (signal & 0x80);
                let sign = self.a & 0x80;
                let total_value = (self.a as u16) + (not_value as u16) + (carry as u16);
                self.set_flag(Flags::Carry, (total_value & 0x100) != 0);
                self.a = added_carry;
                self.set_flag(Flags::Zero, self.a == 0);
                self.set_flag(Flags::Overflow, same_signal && (sign != (self.a & 0x80)));
                self.set_flag(Flags::Negative, (self.a & 0x80) != 0);
                return CycleResult::EndInstruction;
            }
            _ => result,
        }
    }

    fn sec(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.set_flag(Flags::Carry, true);
        CycleResult::EndInstruction
    }

    fn sed(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.set_flag(Flags::Decimal, true);
        CycleResult::EndInstruction
    }

    fn sei(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.set_flag(Flags::InterruptDisable, true);
        CycleResult::EndInstruction
    }

    fn sta(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.write_memory(bus, self.address, self.a);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn stx(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.write_memory(bus, self.address, self.x);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn sty(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        let result = self.resolve_addressing(bus);
        match result {
            CycleResult::Continue => {
                self.write_memory(bus, self.address, self.y);
                CycleResult::EndInstruction
            }
            _ => result,
        }
    }

    fn tax(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.x = self.a;

        self.set_flag(Flags::Zero, self.x == 0);
        self.set_flag(Flags::Negative, (self.x & 0x80) != 0);

        CycleResult::EndInstruction
    }

    fn tay(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.y = self.a;

        self.set_flag(Flags::Zero, self.y == 0);
        self.set_flag(Flags::Negative, (self.y & 0x80) != 0);

        CycleResult::EndInstruction
    }

    fn tsx(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.x = self.sp;

        self.set_flag(Flags::Zero, self.x == 0);
        self.set_flag(Flags::Negative, (self.x & 0x80) != 0);

        CycleResult::EndInstruction
    }

    fn txa(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.a = self.x;

        self.set_flag(Flags::Zero, self.a == 0);
        self.set_flag(Flags::Negative, (self.a & 0x80) != 0);

        CycleResult::EndInstruction
    }

    fn txs(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.sp = self.x;
        CycleResult::EndInstruction
    }

    fn tya(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        self.a = self.y;

        self.set_flag(Flags::Zero, self.a == 0);
        self.set_flag(Flags::Negative, (self.a & 0x80) != 0);

        CycleResult::EndInstruction
    }

    fn stp(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn anc(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn slo(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn rla(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn sre(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn alr(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn rra(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn arr(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn sax(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn xaa(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn ahx(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn tas(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn shy(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn shx(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn lax(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn las(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn dcp(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn isc(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
    fn axs(&mut self, bus: &mut dyn BusOps) -> CycleResult {
        CycleResult::Error
    }
}
