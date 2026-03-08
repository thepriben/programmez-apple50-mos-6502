use std::io::{self, Write};

const MEM_SIZE: usize = 65536;
const OUT_PORT: u16 = 0xF001;

const FLAG_Z: u8 = 0b0000_0010;
const FLAG_N: u8 = 0b1000_0000;

struct Bus {
    mem: [u8; MEM_SIZE],
}

impl Bus {
    fn new() -> Self {
        Self { mem: [0; MEM_SIZE] }
    }

    fn read8(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write8(&mut self, addr: u16, value: u8) {
        if addr == OUT_PORT {
            let mut out = io::stdout();
            let _ = out.write_all(&[value]);
            let _ = out.flush();
            return;
        }
        self.mem[addr as usize] = value;
    }

    fn read16(&self, addr: u16) -> u16 {
        let lo = self.read8(addr) as u16;
        let hi = self.read8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    fn load(&mut self, addr: u16, bytes: &[u8]) {
        let start = addr as usize;
        let end = start + bytes.len();
        self.mem[start..end].copy_from_slice(bytes);
    }
}

struct Cpu6502 {
    a: u8,
    x: u8,
    #[allow(dead_code)]
    y: u8,
    sp: u8,
    pc: u16,
    status: u8,
}

impl Cpu6502 {
    fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            status: 0,
        }
    }

    fn reset(&mut self, bus: &Bus) {
        self.pc = bus.read16(0xFFFC);
        self.sp = 0xFD;
        self.status = 0;
    }

    fn fetch8(&mut self, bus: &Bus) -> u8 {
        let v = bus.read8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }

    fn fetch16(&mut self, bus: &Bus) -> u16 {
        let lo = self.fetch8(bus) as u16;
        let hi = self.fetch8(bus) as u16;
        (hi << 8) | lo
    }

    fn set_flag(&mut self, flag: u8, on: bool) {
        if on {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
    }

    fn set_zn(&mut self, value: u8) {
        self.set_flag(FLAG_Z, value == 0);
        self.set_flag(FLAG_N, (value & 0x80) != 0);
    }

    fn step(&mut self, bus: &mut Bus) -> bool {
        let opcode = self.fetch8(bus);

        match opcode {
            0xA2 => {
                let imm = self.fetch8(bus);
                self.x = imm;
                self.set_zn(self.x);
            }
            0xBD => {
                let base = self.fetch16(bus);
                let addr = base.wrapping_add(self.x as u16);
                self.a = bus.read8(addr);
                self.set_zn(self.a);
            }
            0x8D => {
                let addr = self.fetch16(bus);
                bus.write8(addr, self.a);
            }
            0xE8 => {
                self.x = self.x.wrapping_add(1);
                self.set_zn(self.x);
            }
            0xF0 => {
                let off = self.fetch8(bus) as i8;
                if (self.status & FLAG_Z) != 0 {
                    let pc = self.pc as i32 + off as i32;
                    self.pc = pc as u16;
                }
            }
            0x4C => {
                let addr = self.fetch16(bus);
                self.pc = addr;
            }
            0x00 => {
                return false;
            }
            _ => {
                panic!("Opcode non supporté : 0x{opcode:02X}");
            }
        }

        true
    }
}

fn main() {
    let mut bus = Bus::new();

    let program: [u8; 16] = [
        0xA2, 0x00,
        0xBD, 0x10, 0x80,
        0xF0, 0x07,
        0x8D, 0x01, 0xF0,
        0xE8,
        0x4C, 0x02, 0x80,
        0x00,
        0xEA,
    ];

    let message: [u8; 17] = *b"HELLO FROM 6502\n\0";

    bus.load(0x8000, &program);
    bus.load(0x8010, &message);

    bus.write8(0xFFFC, 0x00);
    bus.write8(0xFFFD, 0x80);

    let mut cpu = Cpu6502::new();
    cpu.reset(&bus);

    let mut steps: u64 = 0;
    let max_steps: u64 = 100_000;

    while steps < max_steps {
        steps += 1;
        if !cpu.step(&mut bus) {
            break;
        }
    }

    println!();
}
