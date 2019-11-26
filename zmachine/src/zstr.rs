use crate::bits::ZWord;

bitstruct! {
    ZCharByte: u16 {
        last_flag: ZCharLastFlag, Width = U1, Offset = U15,
        first: ZCharFirst, Width = U1, Offset = U15,
        second: ZCharSecond, Width = U1, Offset = U15,
        third: ZCharThird, Width = U1, Offset = U15
    }
}

enum CharIdx {
    Memory(usize, u8),
    Abbreviation(usize, usize, u8)
}

pub struct ZCharIter<'a> {
    mem: &'a [u8],
    abbrev_table: &'a [u8],
    idx: Option<CharIdx>,
    abbrev_idx: Option<CharIdx>,
}

impl<'a> ZCharIter<'a> {
    pub fn new(mem: &'a [u8], abbrev_table: &'a [u8]) -> ZCharIter<'a> {
        ZCharIter { mem, abbrev_table, idx: Some(CharIdx::Memory(0, 0)), abbrev_idx: None }
    }
}

enum Alphabet {
    A0, A1, A2
}

struct ZCharMap;

impl ZCharMap {
    fn get_char(c: u8) -> ZChar {
        ZChar::Control(1)
    }
}

enum ZChar {
    Control(u8),
    ZSCII(u8)
}

impl<'a> ZCharIter<'a> {
    fn read_one(&mut self) -> u8 {
        0
    }
    
    fn read_two(&mut self) -> (u8, u8) {
        (0, 0)
    }

    fn control(&mut self, code: u8) -> char {
        'l'
    }
}

impl<'a> Iterator for ZCharIter<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let mut c_idx = if let Some(abbr_idx) = self.abbrev_idx.take() {
            abbr_idx
        } else if let Some(mem_idx) = self.idx.take() {
            mem_idx
        } else {
            return None
        };

        match c_idx {
             CharIdx::Memory(idx, ch) => {
                 // todo: this is the same for both, just in different spots
                let zc = ZCharByte::new(ZWord::from((self.mem[idx], self.mem[idx + 1])).into());
                match ch {
                    0 => {
                        let c = zc.first.value_of() as u8;
                        // this isn't gonna work,
                        // there's context to worry about!! >:[
                        match ZCharMap::get_char(c) {
                            ZChar::Control(f) => {
                                Some(self.control(f))
                            },
                            ZChar::ZSCII(c) => {
                                self.idx = Some(CharIdx::Memory(idx, ch + 1));
                                Some(c as char)
                            },
                        }
                    },
                    1 => {
                        None
                    },
                    2 => {
                        // in this one we will do the "are we done" check
                        None
                    },
                    _ => None,
                }
            },
            CharIdx::Abbreviation(addr, idx, c) => {
                None
            },
            _ => {
                None
            },
        }
    }
}
