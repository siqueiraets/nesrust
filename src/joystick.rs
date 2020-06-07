pub struct Joystick {
    strobe: u8,
    index1: u8,
    index2: u8,
    jd1: u8,
    jd2: u8,
}

impl Joystick {
    pub fn new() -> Self {
        Joystick {
            strobe: 0,
            index1: 0,
            index2: 0,
            jd1: 0,
            jd2: 0,
        }
    }
    pub fn cpu_write(&mut self, address: u16, data: u8) {
        if address == 0x4016 {
            self.strobe = data & 1;
            self.index1 = 0;
            self.index2 = 0;
        }
    }

    pub fn cpu_read(&mut self, address: u16) -> u8 {
        if address == 0x4016 {
            let data = self.jd1 & (0x80 >> self.index1);
            self.index1 += 1;
            if self.index1 == 8 {
                self.index1 = 0;
            }
            return if data != 0 { 1 } else { 0 };
        }

        // TODO handle joystick 2
        return 0;
    }

    pub fn press_start(&mut self) {
        self.jd1 |= 1 << 4;
    }

    pub fn press_select(&mut self) {
        self.jd1 |= 1 << 5;
    }

    pub fn press_up(&mut self) {
        self.jd1 |= 1 << 3;
    }

    pub fn press_down(&mut self) {
        self.jd1 |= 1 << 2;
    }

    pub fn press_left(&mut self) {
        self.jd1 |= 1 << 1;
    }

    pub fn press_right(&mut self) {
        self.jd1 |= 1 << 0;
    }

    pub fn press_a(&mut self) {
        self.jd1 |= 1 << 7;
    }

    pub fn press_b(&mut self) {
        self.jd1 |= 1 << 6;
    }

    pub fn release_start(&mut self) {
        self.jd1 &= !(1 << 4);
    }

    pub fn release_select(&mut self) {
        self.jd1 &= !(1 << 5);
    }

    pub fn release_up(&mut self) {
        self.jd1 &= !(1 << 3);
    }

    pub fn release_down(&mut self) {
        self.jd1 &= !(1 << 2);
    }

    pub fn release_left(&mut self) {
        self.jd1 &= !(1 << 1);
    }

    pub fn release_right(&mut self) {
        self.jd1 &= !(1 << 0);
    }

    pub fn release_a(&mut self) {
        self.jd1 &= !(1 << 7);
    }

    pub fn release_b(&mut self) {
        self.jd1 &= !(1 << 6);
    }
}
