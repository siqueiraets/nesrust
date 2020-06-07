use crate::cpu::BusOps;

pub struct Dma {
    requested: bool,
    page: u8,
    value: u8,
    progress: usize,
}

impl Dma {
    pub fn new() -> Self {
        Dma {
            requested: false,
            page: 0,
            value: 0,
            progress: 0,
        }
    }

    pub fn cpu_write(&mut self, address: u16, data: u8) {
        if address == 0x4014 {
            self.requested = true;
            self.page = data;
            self.progress = 0;
        }
    }

    pub fn active(&self) -> bool {
        self.requested
    }

    pub fn execute(&mut self, bus: &mut dyn BusOps) {
        if self.progress == 0 {
            bus.write(0x2003, 0);
        }

        if self.progress & 1 != 0 {
            bus.write(0x2004, self.value);
        } else {
            let address_high = (self.page as u16) << 8;
            let address_low = (self.progress / 2) as u16;
            let memory_address = address_high | address_low;
            self.value = bus.read(memory_address);
        }

        self.progress += 1;
        if self.progress == 512 {
            self.requested = false;
            self.progress = 0;
        }
    }
}
