pub struct Memory {
    ram: [u8; 2048],
}

impl Memory {
    pub fn new() -> Memory {
        Memory { ram: [0; 2048] }
    }

    pub fn cpu_write(&mut self, address: u16, data: u8) {
        if address < 0x2000 {
            // println!("Write memory: {:#04X}: {:#02X}\n", address & 0x7FF, data);
            self.ram[address as usize & 0x7FF] = data;
        }
    }

    pub fn cpu_read(&mut self, address: u16) -> u8 {
        if address < 0x2000 {
            // println!("Read memory: {:#04X}: {:#02X}\n", address, self.ram[address as usize & 0x7FF]);
            return self.ram[address as usize & 0x7FF];
        }
        0
    }
}
