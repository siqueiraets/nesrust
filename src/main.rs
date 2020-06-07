use anyhow::Result;
use anyhow::*;

use sfml::{
    graphics::{Color, RenderTarget, RenderWindow, Sprite, Texture, Transformable},
    window::{Event, Key, Style},
};
use std::path::Path;

mod cpu;
mod dma;
mod mapper;
mod memory;
mod ppu;

impl<'a> ppu::BusOps for mapper::Mapper {
    fn write(&mut self, address: u16, data: u8) {
        self.ppu_write(address, data);
    }

    fn read(&mut self, address: u16) -> u8 {
        self.ppu_read(address)
    }
}

struct CpuBus<'a> {
    mapper: &'a mut mapper::Mapper,
    memory: &'a mut memory::Memory,
    ppu: &'a mut ppu::Ppu,
    dma: &'a mut dma::Dma,
}

impl<'a> CpuBus<'a> {
    fn new(
        mapper: &'a mut mapper::Mapper,
        memory: &'a mut memory::Memory,
        ppu: &'a mut ppu::Ppu,
        dma: &'a mut dma::Dma,
    ) -> Self {
        CpuBus {
            mapper,
            memory,
            ppu,
            dma,
        }
    }
}

impl<'a> cpu::BusOps for CpuBus<'a> {
    fn write(&mut self, address: u16, data: u8) {
        self.mapper.cpu_write(address, data);
        self.memory.cpu_write(address, data);
        self.ppu.cpu_write(self.mapper, address, data);
        self.dma.cpu_write(address, data);
    }

    fn read(&mut self, address: u16) -> u8 {
        self.mapper.cpu_read(address)
            | self.memory.cpu_read(address)
            | self.ppu.cpu_read(self.mapper, address)
    }
}

struct DmaBus<'a> {
    mapper: &'a mut mapper::Mapper,
    memory: &'a mut memory::Memory,
    ppu: &'a mut ppu::Ppu,
}

impl<'a> DmaBus<'a> {
    fn new(
        mapper: &'a mut mapper::Mapper,
        memory: &'a mut memory::Memory,
        ppu: &'a mut ppu::Ppu,
    ) -> Self {
        DmaBus {
            mapper,
            memory,
            ppu,
        }
    }
}

impl<'a> cpu::BusOps for DmaBus<'a> {
    fn write(&mut self, address: u16, data: u8) {
        self.mapper.cpu_write(address, data);
        self.memory.cpu_write(address, data);
        self.ppu.cpu_write(self.mapper, address, data);
    }

    fn read(&mut self, address: u16) -> u8 {
        self.mapper.cpu_read(address)
            | self.memory.cpu_read(address)
            | self.ppu.cpu_read(self.mapper, address)
    }
}

fn exec_frame(
    cpu: &mut cpu::Cpu6502,
    ppu: &mut ppu::Ppu,
    mapper: &mut mapper::Mapper,
    memory: &mut memory::Memory,
    dma: &mut dma::Dma,
    tick_offset: &mut usize,
) {
    loop {
        *tick_offset += 1;
        if ppu.nmi_state {
            ppu.nmi_state = false;
            cpu.set_nmi();
        }

        if *tick_offset == 3 {
            *tick_offset = 0;

            if dma.active() {
                let mut bus = DmaBus::new(mapper, memory, ppu);
                dma.execute(&mut bus);
            } else {
                let result = {
                    let mut bus = CpuBus::new(mapper, memory, ppu, dma);
                    cpu.tick(&mut bus)
                };

                match result {
                    cpu::CycleResult::Error => {
                        panic!("Error stage: {:#02X}", cpu.stage);
                    }
                    _ => {}
                }
            }
        }

        ppu.tick(mapper);
        if ppu.fetch_frame() {
            break;
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(anyhow!("Usage: <bin> <path>"));
    }

    let path = Path::new(&args[1]);
    let mut mapper = mapper::Mapper::new();
    mapper.load(path)?;

    let mut cpu = cpu::Cpu6502::new();
    cpu.pc = 0xC000;
    cpu.sp = 0xFD;
    cpu.sr = 0x24;
    cpu.reset();

    let mut ppu = ppu::Ppu::new();
    let mut memory = memory::Memory::new();
    let mut dma = dma::Dma::new();
    let mut tick_offset: usize = 0;

    let mut window = RenderWindow::new((800, 600), "Nesrust", Style::CLOSE, &Default::default());
    window.set_framerate_limit(60);

    const WIDTH: usize = 256;
    const HEIGHT: usize = 240;
    let mut texture = Texture::new(WIDTH as u32, HEIGHT as u32).unwrap();

    loop {
        while let Some(event) = window.poll_event() {
            match event {
                Event::Closed => return Ok(()),
                Event::KeyPressed { code, .. } => {
                    match code {
                        Key::Escape => return Ok(()),
                        Key::S => {
                            // nesSystem.js.jd1 |= 1U << 4;
                        }
                        Key::Left => {
                            // nesSystem.js.jd1 |= 1U << 1;
                        }
                        Key::Right => {
                            // nesSystem.js.jd1 |= 1U << 0;
                        }
                        Key::Up => {
                            // nesSystem.js.jd1 |= 1U << 3;
                        }
                        Key::Down => {
                            // nesSystem.js.jd1 |= 1U << 2;
                        }
                        Key::Z => {
                            // nesSystem.js.jd1 |= 1U << 7;
                        }
                        Key::X => {
                            // nesSystem.js.jd1 |= 1U << 6;
                        }
                        Key::L => {
                            // nesSystem.js.jd1 |= 1U << 5;
                        }
                        _ => (),
                    }
                }
                Event::KeyReleased { code, .. } => {
                    match code {
                        Key::S => {
                            // nesSystem.js.jd1 |= 1U << 4;
                        }
                        Key::Left => {
                            // nesSystem.js.jd1 |= 1U << 1;
                        }
                        Key::Right => {
                            // nesSystem.js.jd1 |= 1U << 0;
                        }
                        Key::Up => {
                            // nesSystem.js.jd1 |= 1U << 3;
                        }
                        Key::Down => {
                            // nesSystem.js.jd1 |= 1U << 2;
                        }
                        Key::Z => {
                            // nesSystem.js.jd1 |= 1U << 7;
                        }
                        Key::X => {
                            // nesSystem.js.jd1 |= 1U << 6;
                        }
                        Key::L => {
                            // nesSystem.js.jd1 |= 1U << 5;
                        }
                        _ => (),
                    }
                }
                _ => {}
            }
        }

        exec_frame(
            &mut cpu,
            &mut ppu,
            &mut mapper,
            &mut memory,
            &mut dma,
            &mut tick_offset,
        );

        if window.is_open() {
            window.clear(Color::BLACK);
            unsafe {
                texture.update_from_pixels(&ppu.pixels, WIDTH as u32, HEIGHT as u32, 0, 0);
            }

            let mut sprite = Sprite::new();
            sprite.set_texture(&texture, false);
            sprite.set_scale(sfml::system::Vector2f::new(2f32, 2f32));
            window.draw(&sprite);
            window.display();
        } else {
            break;
        }
    }

    Ok(())
}
