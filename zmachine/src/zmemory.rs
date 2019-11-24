#[derive(Default, Debug)]
pub(crate) struct ZMemory {
    bytes: Vec<u8>,
    globals: Vec<u16>,
}

impl ZMemory {

    pub(crate) fn read_word(&self, idx: u16) -> u16 {
        let top = self.bytes[idx as usize] as u16;
        let lower = self.bytes[idx as usize + 1] as u16;

        (top << 8) | lower
    }

    pub(crate) fn read_byte(&self, idx: u16) -> u8 {
        self.bytes[idx as usize]
    }

    pub(crate) fn reset(&mut self, data: Vec<u8>) {
        println!("Resetting mem, len: {}", data.len());
        self.bytes = data;
        let header = self.header();

        let glob_idx = (header[0x0C] as u16) << 8 | header[0x0D] as u16;
        println!("globals at {:x}", glob_idx);
        let globals = &self.bytes[glob_idx as usize..];

        self.globals.clear();
        for (i, g) in globals.iter().enumerate().step_by(2).take(240) {
            let val = ((*g as u16) << 8) | globals[i + 1] as u16;
            self.globals.push(val);
        }
    }

    pub(crate) fn header(&self) -> &[u8] {
        &self.bytes[0..64]
    }

    pub(crate) fn global(&self, idx: u8) -> u16 {
        self.globals[idx as usize]
    }

    pub(crate) fn set_global(&mut self, idx: u8, val: u16) {
        self.globals[idx as usize] = val;
    }

    pub(crate) fn set_word(&mut self, idx: usize, val: u16) {
        let hi = (val >> 8) as u8;
        let lo = (val & 0x00ff) as u8;

        self.bytes[idx] = hi;
        self.bytes[idx + 1] = lo;
    }

    pub(crate) fn slice(&self, idx: usize) -> &[u8] {
        &self.bytes[idx..]
    }
}

