pub struct Cpu {
    pub regs: [u32; 32],
    pub pc: u32,
    pub dram: Vec<u8>,
}

impl Cpu {
    pub fn new(code: Vec<u8>) -> Self {
        Self {
            regs: [0; 32],
            pc: 0,
            dram: code,
        }
    }
    pub fn fetch(&self) -> u32 {
        let index = self.pc as usize;
        return (self.dram[index] as u32)
            | ((self.dram[index + 1] as u32) << 8)
            | ((self.dram[index + 2] as u32) << 16)
            | ((self.dram[index + 3] as u32) << 24);
    }
    pub fn execute(&mut self, inst: u32) {
        let opcode = inst & 0x7f;
        let funct3 = (inst >> 12) & 0x07;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;

        self.regs[0] = 0; // Simulate hard wired x0

        match opcode {
            0x13 => {
                match funct3 {
                    0x00 => {
                        // addi
                        let imm = ((inst & 0xfff00000) as i32 as i64 >> 20) as u32;
                        self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                    }
                    _ => {
                        dbg!("not implemented yet");
                    }
                }
            }
            0x33 => {
                // add
                self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
            }
            _ => {
                dbg!("not implemented yet");
            }
        }
    }
}
