const OAM_OFFSET_Y: usize = 0;
const OAM_OFFSET_INDEX: usize = 1;
const OAM_OFFSET_ATTR: usize = 2;
const OAM_OFFSET_X: usize = 3;
const OAM_DATA_SIZE: usize = 4;
const SPRITES_IN_PRIMARY: usize = 64;
const SPRITES_IN_SECONDARY: usize = 8;

const SCREEN_WIDTH: usize = 256;
const SCREEN_HEIGHT: usize = 240;
pub struct Ppu {
    pub nmi_state: bool,
    pub pixels: [u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
    primary_oam: [u8; SPRITES_IN_PRIMARY * OAM_DATA_SIZE],
    secondary_oam: [u8; SPRITES_IN_SECONDARY * OAM_DATA_SIZE],
    sprite_counter: [u8; SPRITES_IN_SECONDARY],
    sprite_attribute: [u8; SPRITES_IN_SECONDARY],
    sprite_pattern1: [u8; SPRITES_IN_SECONDARY],
    sprite_pattern2: [u8; SPRITES_IN_SECONDARY],
    sprite_pixel: u8,
    sprite_palette: u8,
    sprite_index: usize,
    sprite_priority: u8,
    sprite_zero_hit: bool,
    sprite_zero_present: bool,
    active_sprites: usize,
    secondary_sprites: usize,
    ppu_addr: u16,
    oam_addr: u8,
    first_addr: bool,
    render_x: u16,
    render_y: u16,
    vblank_started: bool,
    base_nametable: u16,
    addr_increment: u8,
    sprite_pattern_table: u16,
    background_pattern_table: u16,
    sprite_size: u8,
    nmi_enable: bool,
    render_sprite_enable: bool,
    render_background_enable: bool,
    first_scroll_write: bool,
    background_pattern1: u8,
    background_pattern2: u8,
    background_attribute: u8,
    background_pixel: u8,
    background_counter: u8,
    scroll_x: u16,
    scroll_y: u16,
    frame: bool,
}

pub trait BusOps {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, data: u8);
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            nmi_state: false,
            pixels: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
            primary_oam: [0; SPRITES_IN_PRIMARY * OAM_DATA_SIZE],
            secondary_oam: [0; SPRITES_IN_SECONDARY * OAM_DATA_SIZE],
            sprite_counter: [0; SPRITES_IN_SECONDARY],
            sprite_attribute: [0; SPRITES_IN_SECONDARY],
            sprite_pattern1: [0; SPRITES_IN_SECONDARY],
            sprite_pattern2: [0; SPRITES_IN_SECONDARY],
            sprite_pixel: 0,
            sprite_palette: 0,
            sprite_index: 0,
            sprite_priority: 0,
            sprite_zero_hit: false,
            sprite_zero_present: false,
            active_sprites: 0,
            secondary_sprites: 0,
            ppu_addr: 0,
            oam_addr: 0,
            first_addr: true,
            render_x: 0,
            render_y: 0,
            vblank_started: false,
            base_nametable: 0,
            addr_increment: 1,
            sprite_pattern_table: 0,
            background_pattern_table: 0,
            sprite_size: 8,
            nmi_enable: false,
            render_sprite_enable: false,
            render_background_enable: false,
            first_scroll_write: true,
            background_pattern1: 0,
            background_pattern2: 0,
            background_attribute: 0,
            background_pixel: 0,
            background_counter: 0,
            scroll_x: 0,
            scroll_y: 0,
            frame: false,
        }
    }

    pub fn fetch_frame(&mut self) -> bool {
        let result = self.frame;
        self.frame = false;
        result
    }

    pub fn tick(&mut self, ppu_bus: &mut dyn BusOps) {
        if self.render_y <= 240 && self.render_x < 256 {
            if self.render_x == 0 {
                self.load_secondary_oam();
            }

            if self.render_background_enable {
                self.fetch_background(ppu_bus);
            }

            if self.render_sprite_enable {
                self.update_x_position(ppu_bus);
                self.render_sprites();
            }
            self.render_pixel(ppu_bus);
        }

        if self.render_x == 1 && self.render_y == 241 {
            self.vblank_started = true;
            if self.nmi_enable {
                self.nmi_state = true;
            }
            self.frame = true;
        }

        self.render_x += 1;
        if self.render_x == 341 {
            self.render_x = 0;
            self.render_y += 1;
            if self.render_y == 262 {
                self.vblank_started = false;
                self.sprite_zero_hit = false;
                self.render_y = 0;
            }
        }
    }

    pub fn cpu_write(&mut self, ppu_bus: &mut dyn BusOps, address: u16, data: u8) {
        if address & 0xE000 == 0x2000
        // Handle addresses 0x2000 - 0x3FFF
        {
            match address & 0x7 {
                0 =>
                // ppuctrl
                {
                    self.base_nametable = match data & 3 {
                        0 => 0x2000,
                        1 => 0x2400,
                        2 => 0x2800,
                        3 => 0x2C00,
                        _ => 0x2000,
                    };
                    self.addr_increment = if data & 0x4 != 0 { 32 } else { 1 };
                    self.sprite_pattern_table = if data & 0x8 != 0 { 0x1000 } else { 0x0 };
                    self.background_pattern_table = if data & 0x10 != 0 { 0x1000 } else { 0x0 };
                    self.sprite_size = if data & 0x20 != 0 { 0x16 } else { 0x8 };
                    self.nmi_enable = data & 0x80 != 0;
                    if self.nmi_enable && self.vblank_started {
                        // TODO enabling this breaks rendering
                        // self.nmi_state = true;
                    }
                }
                1 =>
                // ppumask
                {
                    self.render_background_enable = data & 0x08 != 0;
                    self.render_sprite_enable = data & 0x10 != 0;
                }
                2 => (), // ppustatus
                3 =>
                // oamaddr
                {
                    self.oam_addr = data;
                }
                4 =>
                // oamdata
                {
                    self.primary_oam[self.oam_addr as usize] = data;
                    self.oam_addr = self.oam_addr.wrapping_add(1);
                }
                5 =>
                // ppuscroll
                {
                    if self.first_scroll_write {
                        self.scroll_x = data as u16;
                    } else {
                        self.scroll_y = data as u16;
                    }
                    self.first_scroll_write = !self.first_scroll_write;
                }
                6 =>
                // ppuaddr
                {
                    if self.first_addr {
                        self.ppu_addr = (data as u16) << 8;
                    } else {
                        self.ppu_addr |= data as u16;
                    }
                    self.first_addr = !self.first_addr;
                }
                7 =>
                // ppudata
                {
                    ppu_bus.write(self.ppu_addr, data);
                    self.ppu_addr += self.addr_increment as u16;
                }
                _ => (),
            }
        }
    }

    pub fn cpu_read(&mut self, ppu_bus: &mut dyn BusOps, address: u16) -> u8 {
        if address & 0xE000 == 0x2000
        // Handle addresses 0x2000 - 0x3FFF
        {
            match address & 0x7 {
                0 => 0, // ppuctrl
                1 => 0, // ppumask
                2 =>
                // ppustatus
                {
                    let mut data = 0;
                    if self.vblank_started {
                        self.vblank_started = false;
                        data |= 0x80;
                    }
                    if self.sprite_zero_hit {
                        data |= 0x40;
                    }
                    data
                }
                3 => 0, // oamaddr
                4 => 0, // oamdata
                5 => 0, // ppuscroll
                6 => 0, // ppuaddr
                7 =>
                // ppudata
                {
                    ppu_bus.read(self.ppu_addr)
                }
                _ => 0,
            }
        } else {
            0
        }
    }

    fn load_secondary_oam(&mut self) {
        self.active_sprites = 0;
        self.secondary_sprites = 0;
        self.secondary_oam = [0xFF; SPRITES_IN_SECONDARY * OAM_DATA_SIZE];
        self.sprite_counter = [0; SPRITES_IN_SECONDARY];
        self.sprite_zero_present = false;
        let range = 0..(SPRITES_IN_PRIMARY * OAM_DATA_SIZE);
        for sprite_offset in range.step_by(4) {
            let sprite_x = self.primary_oam[sprite_offset + OAM_OFFSET_X];
            let sprite_y = self.primary_oam[sprite_offset + OAM_OFFSET_Y];
            let sprite_attr = self.primary_oam[sprite_offset + OAM_OFFSET_ATTR];
            let sprite_index = self.primary_oam[sprite_offset + OAM_OFFSET_INDEX];
            if self.render_y > self.primary_oam[sprite_offset + OAM_OFFSET_Y] as u16
                && self.render_y
                    <= (self.primary_oam[sprite_offset + OAM_OFFSET_Y] as u16
                        + self.sprite_size as u16)
            {
                self.secondary_oam[self.secondary_sprites * OAM_DATA_SIZE + OAM_OFFSET_Y] =
                    sprite_y;
                self.secondary_oam[self.secondary_sprites * OAM_DATA_SIZE + OAM_OFFSET_INDEX] =
                    sprite_index;
                self.secondary_oam[self.secondary_sprites * OAM_DATA_SIZE + OAM_OFFSET_X] =
                    sprite_x;
                self.secondary_oam[self.secondary_sprites * OAM_DATA_SIZE + OAM_OFFSET_ATTR] =
                    sprite_attr;
                self.sprite_counter[self.secondary_sprites] =
                    self.primary_oam[sprite_offset + OAM_OFFSET_X];

                self.secondary_sprites += 1;

                if sprite_offset == 0 {
                    self.sprite_zero_present = true;
                }
            }

            if self.secondary_sprites >= 8 {
                break;
            }
        }
    }

    fn update_x_position(&mut self, ppu_bus: &mut dyn BusOps) {
        // TODO Check sprites at X=0
        for sprite_offset in 0..self.secondary_sprites {
            self.sprite_counter[sprite_offset] = self.sprite_counter[sprite_offset].wrapping_sub(1);
            if self.sprite_counter[sprite_offset] != 0 {
                continue;
            }

            let sprite_attribute =
                self.secondary_oam[sprite_offset * OAM_DATA_SIZE + OAM_OFFSET_ATTR];
            let vertical_flip = sprite_attribute & 0x80 != 0;

            let offset_y = self.render_y
                - self.secondary_oam[sprite_offset * OAM_DATA_SIZE + OAM_OFFSET_Y] as u16
                - 1;

            if self.sprite_size == 8 {
                let sprite_index =
                    self.secondary_oam[sprite_offset * OAM_DATA_SIZE + OAM_OFFSET_INDEX] as u16;

                if vertical_flip {
                    self.sprite_pattern1[self.active_sprites] = ppu_bus
                        .read(self.sprite_pattern_table + (sprite_index * 16) + 8 - offset_y);
                    self.sprite_pattern2[self.active_sprites] = ppu_bus
                        .read(self.sprite_pattern_table + (sprite_index * 16) + 8 - offset_y + 8);
                } else {
                    self.sprite_pattern1[self.active_sprites] =
                        ppu_bus.read(self.sprite_pattern_table + (sprite_index * 16) + offset_y);
                    self.sprite_pattern2[self.active_sprites] = ppu_bus
                        .read(self.sprite_pattern_table + (sprite_index * 16) + offset_y + 8);
                }
            } else {
                // TODO handle vertical flip
                let is_bottom = offset_y >= 8;

                let sprite_index_value =
                    self.secondary_oam[sprite_offset * OAM_DATA_SIZE + OAM_OFFSET_INDEX];
                let pattern_table = if sprite_index_value & 0x1 != 0 {
                    0x1000
                } else {
                    0x0
                };

                let sprite_index_offset = if is_bottom { 1 } else { 0 };
                let sprite_index = (sprite_index_value as u16 >> 1) + sprite_index_offset;

                self.sprite_pattern1[self.active_sprites] =
                    ppu_bus.read(pattern_table + (sprite_index * 16) + offset_y);
                self.sprite_pattern2[self.active_sprites] =
                    ppu_bus.read(pattern_table + (sprite_index * 16) + offset_y + 8);
            }

            self.sprite_attribute[self.active_sprites as usize] = sprite_attribute;
            self.active_sprites += 1;
        }
    }

    fn fetch_background(&mut self, ppu_bus: &mut dyn BusOps) {
        let mut fetch_x = self.render_x + self.scroll_x;
        let mut fetch_y = self.render_y + self.scroll_y;
        let mut nametable_addr = self.base_nametable;
        if fetch_y >= 240 {
            fetch_y -= 240;
            nametable_addr += 0x800;
        }
        let mut fetch_nametable2 = false;
        if fetch_x >= 256 {
            fetch_x -= 256;
            nametable_addr += 0x400;
            fetch_nametable2 = true;
        }
        let nametable_col_index = fetch_x / 8;
        let nametable_row_index = fetch_y / 8;
        let nametable_index = nametable_col_index + (nametable_row_index * 32);
        let pattern_index = ppu_bus.read(nametable_addr + nametable_index);
        self.background_pattern1 = ppu_bus
            .read(self.background_pattern_table + (pattern_index as u16 * 16) + (fetch_y % 8));
        self.background_pattern2 = ppu_bus
            .read(self.background_pattern_table + (pattern_index as u16 * 16) + (fetch_y % 8) + 8);
        self.background_counter = 8;
        let attribute_table = (nametable_addr & 0xFC00) | 0x3C0;
        let attribute_addr = attribute_table + (fetch_x / 32) + ((fetch_y / 32) * 8);
        self.background_attribute = ppu_bus.read(attribute_addr);
        if (fetch_x / 16) & 1 != 0 {
            self.background_attribute >>= 2;
        }
        if (fetch_y / 16) & 1 != 0 {
            self.background_attribute >>= 4;
        }
        self.background_attribute &= 0x3;
        self.background_pattern1 <<= fetch_x & 0b111;
        self.background_pattern2 <<= fetch_x & 0b111;

        self.background_pixel =
            ((self.background_pattern1 & 0x80) >> 6) | ((self.background_pattern2 & 0x80) >> 7);
        self.background_counter -= 1;
    }

    fn render_sprites(&mut self) {
        self.sprite_pixel = 0;
        for index in 0..self.active_sprites {
            let sprite_attribute = self.sprite_attribute[index];
            let horizontal_flip = sprite_attribute & 0x40 != 0;
            let bitmask = if horizontal_flip { 0x1 } else { 0x80 };
            let pattern1 = self.sprite_pattern1[index] & bitmask != 0;
            let pattern2 = self.sprite_pattern2[index] & bitmask != 0;
            if self.sprite_pixel == 0 && (pattern1 || pattern2) {
                self.sprite_index = index;
                self.sprite_pixel = if pattern1 { 1 } else { 0 };
                self.sprite_pixel += if pattern2 { 2 } else { 0 };
                self.sprite_palette = sprite_attribute & 0x3;
                self.sprite_priority = if sprite_attribute & 0x20 != 0 { 1 } else { 0 };
            }

            if horizontal_flip {
                self.sprite_pattern1[index] >>= 1;
                self.sprite_pattern2[index] >>= 1;
            } else {
                self.sprite_pattern1[index] <<= 1;
                self.sprite_pattern2[index] <<= 1;
            }
        }
    }

    fn render_pixel(&mut self, ppu_bus: &mut dyn BusOps) {
        let mut color = 0xF;
        if self.sprite_pixel != 0 && (self.sprite_priority == 0 || self.background_pixel == 0) {
            color =
                ppu_bus.read(0x3F10 + (self.sprite_palette as u16 * 4) + self.sprite_pixel as u16);
        } else if self.background_pixel != 0 {
            color = ppu_bus.read(
                0x3F00 + (self.background_attribute as u16 * 4) + self.background_pixel as u16,
            );
        } else {
            color = ppu_bus.read(0x3F00);
        }

        if self.background_pixel != 0
            && self.sprite_pixel != 0
            && self.sprite_index == 0
            && self.sprite_zero_present
        {
            self.sprite_zero_hit = true;
        }
        self.write_pixel(color);
    }

    fn write_pixel(&mut self, color: u8) {
        let colors = [
            84, 84, 84, 0, 30, 116, 8, 16, 144, 48, 0, 136, 68, 0, 100, 92, 0, 48, 84, 4, 0, 60,
            24, 0, 32, 42, 0, 8, 58, 0, 0, 64, 0, 0, 60, 0, 0, 50, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            152, 150, 152, 8, 76, 196, 48, 50, 236, 92, 30, 228, 136, 20, 176, 160, 20, 100, 152,
            34, 32, 120, 60, 0, 84, 90, 0, 40, 114, 0, 8, 124, 0, 0, 118, 40, 0, 102, 120, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 236, 238, 236, 76, 154, 236, 120, 124, 236, 176, 98, 236, 228, 84,
            236, 236, 88, 180, 236, 106, 100, 212, 136, 32, 160, 170, 0, 116, 196, 0, 76, 208, 32,
            56, 204, 108, 56, 180, 204, 60, 60, 60, 0, 0, 0, 0, 0, 0, 236, 238, 236, 168, 204, 236,
            188, 188, 236, 212, 178, 236, 236, 174, 236, 236, 174, 212, 236, 180, 176, 228, 196,
            144, 204, 210, 120, 180, 222, 120, 168, 226, 144, 152, 226, 180, 160, 214, 228, 160,
            162, 160, 0, 0, 0, 0, 0, 0,
        ];

        if (self.render_x as usize) < SCREEN_WIDTH
            && (self.render_y as usize) < SCREEN_HEIGHT
            && color < 64
        {
            let pixel_index =
                self.render_y as usize * SCREEN_WIDTH * 4 + self.render_x as usize * 4;
            let color_index = color as usize * 3;
            self.pixels[pixel_index] = colors[color_index];
            self.pixels[pixel_index + 1] = colors[color_index + 1];
            self.pixels[pixel_index + 2] = colors[color_index + 2];
            self.pixels[pixel_index + 3] = 0xFF;
        }
    }
}
