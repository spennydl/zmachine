#[derive(Default, Debug)]
pub(crate) struct ZMemory {
    bytes: Vec<u8>,
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
        self.bytes = data;
    }

    pub(crate) fn header(&self) -> &[u8] {
        &self.bytes[0..64]
    }
}

