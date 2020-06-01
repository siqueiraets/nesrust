use anyhow::{Result, *};
use std::{fs::File, io::BufReader, io::Read, path::Path};

pub struct Mapper {
    num_blocks: u8,
    first_cpu_bank: usize,
    last_cpu_bank: usize,
    first_ppu_bank: usize,
    last_ppu_bank: usize,
    memory: [u8; 262144],
    shift: u8,
    mirroring: u8,
    prg_mode: u8,
    chr_mode: u8,
    chr_ram: [u8; 32768],
    nametables: [u8; 4096],
    prgram: [u8; 16384],
    palettes: [u8; 32],
}

impl Mapper {
    pub fn new() -> Mapper {
        Mapper {
            num_blocks: 0,
            first_cpu_bank: 0,
            last_cpu_bank: 0,
            first_ppu_bank: 0,
            last_ppu_bank: 0,
            memory: [0; 262144],
            shift: 0,
            mirroring: 0,
            prg_mode: 0,
            chr_mode: 0,
            chr_ram: [0; 32768],
            nametables: [0; 4096],
            prgram: [0; 16384],
            palettes: [0; 32],
        }
    }

    pub fn load(&mut self, path: &Path) -> Result<()> {
        let mut reader = BufReader::new(File::open(path)?); // TODO error handling

        let mut header: [u8; 16] = [0; 16];
        reader.read(&mut header)?;

        let nesheader = ['N' as u8, 'E' as u8, 'S' as u8, 0x1A];
        if nesheader != header[0..4] {
            return Err(anyhow!("Invalid nes file"));
        }

        if header[7] & 0x0C == 0x08 {
            return Err(anyhow!("File is ines2.0"));
        }

        self.num_blocks = header[4];
        println!("NumBlocks: {}", self.num_blocks);

        if self.num_blocks == 0 {
            return Err(anyhow!("No data in nes file"));
        } else if self.num_blocks > 16 {
            return Err(anyhow!("Too much data"));
        }

        let low_nibble = (header[6] & 0xF0) >> 4;
        let high_nibble = header[7] & 0xF0;
        let mapper = low_nibble | high_nibble;
        if mapper != 0 && mapper != 1 {
            return Err(anyhow!("Unsupported mapper {}", mapper));
        }

        let memory_size = 16384 * self.num_blocks as usize;
        reader
            .read_exact(&mut self.memory[0..memory_size])
            .context("Failed to read nes data")?;
        self.first_cpu_bank = 0;
        self.last_cpu_bank = 16384 * (self.num_blocks as usize - 1);
        self.first_ppu_bank = 0;
        self.last_ppu_bank = 0;
        self.shift = 1 << 4;
        self.chr_mode = 0;
        Ok(())
    }

    pub fn cpu_write(&mut self, address: u16, data: u8) {
        if address >= 0x4020 && address <= 0x7FFF {
            self.prgram[address as usize & 0x1FFF] = data;
        }
        if address & 0x8000 == 0 {
            return;
        }
        if data & 0x80 != 0 {
            self.shift = 1 << 4;
            return;
        }

        let last = self.shift & 1 != 0;
        self.shift = (self.shift >> 1) | ((data & 1) << 4);
        if !last {
            return;
        }

        match address & 0x6000 {
            0x0000 => {
                self.mirroring = self.shift & 0x3;
                self.prg_mode = (self.shift & 0xC) >> 2;
                self.chr_mode = (self.shift & 0x10) >> 4;
            }
            0x2000 => match self.chr_mode {
                0 => self.first_ppu_bank = self.shift as usize & 0x1E * 4096,
                _ => self.first_ppu_bank = self.shift as usize * 4096,
            },
            0x4000 => {
                if self.chr_mode != 0
                // 8kb bank is ignored
                {
                    self.last_ppu_bank = self.shift as usize * 4096;
                }
            }
            0x6000 => {
                match self.prg_mode {
                    0 =>
                    // 32k mode
                    {
                        self.first_cpu_bank = (self.shift as usize & 0xE) * 16384;
                        self.last_cpu_bank = self.first_cpu_bank + 16384;
                    }
                    // 32k mode
                    1 => {
                        self.first_cpu_bank = (self.shift as usize & 0xE) * 16384;
                        self.last_cpu_bank = self.first_cpu_bank + 16384;
                    }
                    2 =>
                    // Fix first at 0x8000 and switch last at 0xC000
                    {
                        self.first_cpu_bank = 0;
                        self.last_cpu_bank = self.shift as usize * 16384;
                    }
                    3 =>
                    // Fix last at 0xC000 and switch first at 0x8000
                    {
                        self.first_cpu_bank = self.shift as usize * 16384;
                        self.last_cpu_bank = 16384 * (self.num_blocks as usize - 1);
                    }
                    _ => (),
                }
            }
            _ => (),
        }
        self.shift = 1 << 4;
    }

    pub fn cpu_read(&mut self, address: u16) -> u8 {
        if address >= 0x4020 && address <= 0x7FFF {
            return self.prgram[address as usize & 0x1FFF];
        }
        if address & 0xC000 == 0x8000 {
            return self.memory[(self.first_cpu_bank) + (address as usize & 0x3FFF)];
        }
        if address & 0xC000 == 0xC000 {
            return self.memory[(self.last_cpu_bank) + (address as usize & 0x3FFF)];
        }

        0
    }

    pub fn ppu_write(&mut self, address: u16, data: u8) {
        let address = address & 0x3FFF;
        if address <= 0x1FFF {
            if self.chr_mode == 0
            // 8kb bank
            {
                self.chr_ram[self.first_ppu_bank + address as usize] = data;
            } else if address & 0x1000 != 0 {
                self.chr_ram[self.last_ppu_bank + (address as usize & 0xFFF)] = data;
            } else {
                self.chr_ram[self.first_ppu_bank + address as usize] = data;
            }
        } else if address <= 0x3EFF {
            let mut real_address = address as usize & 0xFFF;
            if self.mirroring == 0
            // one screen lower bank
            {
                real_address &= 0x3FF;
            } else if self.mirroring == 1
            // one screen upper bank
            {
                // TODO upper bank
                real_address &= 0x3FF;
            } else if self.mirroring == 2
            // Vertical
            {
                real_address &= 0x7FF;
            } else if self.mirroring == 3
            // horizontal
            {
                real_address &= 0xBFF;
            }
            self.nametables[real_address] = data;
        } else {
            let mut real_address = address as usize;
            // Backdrop mirroring
            if address & 0x13 == 0x10 {
                real_address = real_address & 0xF;
            }
            self.palettes[real_address & 0x1F] = data;
        }
    }

    pub fn ppu_read(&mut self, address: u16) -> u8 {
        let address = address & 0x3FFF;
        if address <= 0x1FFF {
            return self.chr_ram[self.first_ppu_bank + address as usize];
        } else if address <= 0x3EFF {
            let mut real_address = address as usize & 0xFFF;
            if self.mirroring == 0
            // one screen lower bank
            {
                real_address &= 0x3FF;
            } else if self.mirroring == 1
            // one screen upper bank
            {
                real_address &= 0x3FF;
            } else if self.mirroring == 2
            // Vertical
            {
                real_address &= 0x7FF;
            } else if self.mirroring == 3
            // horizontal
            {
                real_address &= 0xBFF;
            }
            return self.nametables[real_address];
        } else {
            let mut address = address as usize;
            // Backdrop mirroring
            if address & 0x13 == 0x10 {
                address = address & 0xF;
            }
            return self.palettes[address & 0x1F];
        }
    }
}
