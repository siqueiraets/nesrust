mod cpu;
mod mapper;
mod memory;
mod ppu;
use anyhow::Result;
use anyhow::*;
use std::path::Path;

struct CpuBus<'a> {
    mapper: &'a mut mapper::Mapper,
    memory: &'a mut memory::Memory,
    ppu: &'a mut ppu::Ppu,
}

impl<'a> CpuBus<'a> {
    fn new(
        mapper: &'a mut mapper::Mapper,
        memory: &'a mut memory::Memory,
        ppu: &'a mut ppu::Ppu,
    ) -> Self {
        CpuBus {
            mapper,
            memory,
            ppu,
        }
    }
}

impl<'a> ppu::BusOps for mapper::Mapper {
    fn write(&mut self, address: u16, data: u8) {
        self.ppu_write(address, data);
    }

    fn read(&mut self, address: u16) -> u8 {
        self.ppu_read(address)
    }
}

impl<'a> cpu::BusOps for CpuBus<'a> {
    fn write(&mut self, address: u16, data: u8) {
        self.mapper.cpu_write(address, data);
        self.memory.cpu_write(address, data);
        self.ppu.cpu_write(self.mapper, address, data);
    }

    fn read(&mut self, address: u16) -> u8 {
        self.mapper.cpu_read(address) | self.memory.cpu_read(address)
    }
}



fn print_state(cpu: &cpu::Cpu6502, cycle: usize) {
    // println!(
    //     "{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
    //     cpu.pc, cpu.a, cpu.x, cpu.y, cpu.sr, cpu.sp, cycle
    // );
}

fn run(path: &Path) -> Result<()> {
    let mut mapper = mapper::Mapper::new();
    let mut memory = memory::Memory::new();
    let mut cpu = cpu::Cpu6502::new();
    let mut ppu = ppu::Ppu::new();
    let mut cycles = 7 as usize;
    cpu.pc = 0xC000;
    cpu.sp = 0xFD;
    cpu.sr = 0x24;

    cpu.reset();

    mapper.load(path)?;
    loop {
        let mut bus = CpuBus::new(&mut mapper, &mut memory, &mut ppu);
        let result = cpu.tick(&mut bus);

        cycles += 1;
        match result {
            cpu::CycleResult::Error => {
                println!("Error stage: {:#02X}", cpu.stage);
                print_state(&cpu, cycles);

                break;
            }
            cpu::CycleResult::EndInstruction => print_state(&cpu, cycles),
            _ => {}
        }

        let mut test_result: [u8; 4] = [0; 4];
        test_result[0] = mapper.cpu_read(0x6000);
        test_result[1] = mapper.cpu_read(0x6001);
        test_result[2] = mapper.cpu_read(0x6002);
        test_result[3] = mapper.cpu_read(0x6003);

        let result_valid = test_result[1] == 0xDE && test_result[2] == 0xB0;
        if result_valid && test_result[0] != 0x80 {
            println!(
                "Test finished: {:#02X} {:#02X} {:#02X} {:#02X}",
                test_result[0], test_result[1], test_result[2], test_result[3]
            );
            let mut index = 0;
            loop {
                let data = mapper.cpu_read(0x6004 + index);
                if data == 0 {
                    break;
                }
                print!("{}", data as char);
                index += 1;
            }
            break;
        }

        // if cycles > 26554000 {
        //     break;
        // }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(anyhow!("Usage: <bin> <path>"));
    }
    let path = Path::new(&args[1]);
    run(&path)?;
    Ok(())
}
