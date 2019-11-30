use crate::bits::ZWord;

bitstruct! {
    ZCharWord: u16 {
        last_flag: ZCharLastFlag, Width = U1, Offset = U15,
        first: ZCharFirst, Width = U5, Offset = U10,
        second: ZCharSecond, Width = U5, Offset = U5,
        third: ZCharThird, Width = U5, Offset = U0
    }
}

#[derive(Debug)]
enum CharIdx {
    Memory(usize, u8),
    Abbreviation(usize, usize, u8)
}

pub struct ZString {
    len_words: usize,
    string: String
}

impl ZString {
    pub fn new<'a>(mem: &'a [u8], abbrev_table: &'a [u8]) -> ZString {
        let mut offset: usize = 0;
        let string: String = ZCharIterInner::new(mem, abbrev_table, &mut offset).collect();
        ZString { len_words: offset, string }
    }

    pub fn string(self) -> String {
        self.string
    }

    pub fn offset(&self) -> usize {
        self.len_words
    }
}

pub struct ZCharIterInner<'a> {
    offset: &'a mut usize,
    mem: &'a [u8],
    abbrev_table: &'a [u8],
    idxs: Vec<CharIdx>
}



impl<'a> ZCharIterInner<'a> {
    pub fn new(mem: &'a [u8], abbrev_table: &'a [u8], offset: &'a mut usize) -> ZCharIterInner<'a> {
        ZCharIterInner { mem, abbrev_table, idxs: vec![CharIdx::Memory(0, 0)], offset }
    }
}

static ALPH_A0: &'static str = "abcdefghijklmnopqrstuvwxyz";
static ALPH_A1: &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
static ALPH_A2: &'static str = " ^0123456789.,!?_#'\"/\\-:()";

enum Alphabet {
    A0, A1, A2
}

impl Alphabet {
    fn new(idx: u8) -> Self {
        match idx {
            4 => Alphabet::A1,
            5 => Alphabet::A2,
            _ => Alphabet::A0,
        }
    }

    fn get(&self, n: u8) -> char {
        if n == 0 {
            ' '
        } else {
            match self {
                Alphabet::A0 => ALPH_A0.chars().nth(n as usize - 6).unwrap(),
                Alphabet::A1 => ALPH_A1.chars().nth(n as usize - 6).unwrap(),
                Alphabet::A2 => ALPH_A2.chars().nth(n as usize - 6).unwrap(),
            }
        }
    }
}

enum ZChar {
    Shift(u8),
    Abbrev(u8),
    Char(u8)
}

impl ZChar {
    fn new(c: u8) -> Self {
        if c == 0 {
            ZChar::Char(0)
        } else if c < 4 {
            ZChar::Abbrev(c)
        } else if c < 6 {
            ZChar::Shift(c)
        } else {
            ZChar::Char(c)
        }
    }
}

impl<'a> ZCharIterInner<'a> {
    fn read_next(&mut self, mut idx: CharIdx, alphabet: Alphabet) -> Option<char> {
        let (buf, cidx, c) = match idx {
            CharIdx::Memory(ref mut cidx, ref mut c) => {
                *self.offset = *cidx + 2; // this is a hack, but it should work
                (&self.mem[*cidx..], cidx, c)
            },
            CharIdx::Abbreviation(ref addr, ref mut cidx, ref mut c) => {
                (&self.abbrev_table[addr + *cidx..], cidx, c)
            },
        };

        let word = ZWord::from((buf[0], buf[1]));
        let chars = ZCharWord::new(word.into());

        match c {
            0 => {
                match ZChar::new(chars.first.value_of() as u8) {
                    ZChar::Abbrev(aidx) => {
                        *c = 2;
                        self.idxs.push(idx);
                        let idx = CharIdx::Abbreviation(32 * (aidx as usize - 1), chars.second.value_of() as usize, 0);

                        self.read_next(idx, Alphabet::A0)
                    },
                    ZChar::Shift(a) => {
                        let alph = Alphabet::new(a);
                        *c = 1;

                        self.read_next(idx, alph)
                    },
                    ZChar::Char(zc) => {
                        if let Alphabet::A2 = alphabet {
                            if zc == 6 {
                                let char_word: ZWord = (chars.second.value_of() as u8, chars.third.value_of() as u8).into();
                                let char_code = u16::from(char_word) as u8;

                                if !chars.last_flag.is_set() {
                                    *cidx += 2;
                                    *c = 0;
                                    self.idxs.push(idx);
                                }
                                
                                return Some(char_code as char)
                            }
                        }
                        let res = alphabet.get(zc);
                        *c = 1;

                        self.idxs.push(idx);
                        Some(res)
                    }
                }
            },
            1 => {
                match ZChar::new(chars.second.value_of() as u8) {
                    ZChar::Abbrev(aidx) => {
                        if !chars.last_flag.is_set() {
                            *cidx += 2;
                            *c = 0;
                            self.idxs.push(idx);
                        }

                        let idx = CharIdx::Abbreviation(32 * (aidx as usize - 1), chars.third.value_of() as usize, 0);
                        self.read_next(idx, Alphabet::A0)
                    },
                    ZChar::Shift(a) => {
                        let alph = Alphabet::new(a);
                        *c = 2;

                        self.read_next(idx, alph)
                    },
                    ZChar::Char(zc) => {
                        if let Alphabet::A2 = alphabet {
                            if zc == 6 && chars.last_flag.is_set() {
                                return None
                            }
                        }
                        let res = alphabet.get(zc);
                        *c = 2;
                        self.idxs.push(idx);
                        Some(res)
                    }
                }
            },
            2 => {
                match ZChar::new(chars.third.value_of() as u8) {
                    ZChar::Abbrev(aidx) => {
                        if chars.last_flag.is_set() {
                            // errrrrrr we tried to hit an abbreviation but we didn't
                            // have enough chars
                            return None;
                        }
                        *c = 1;
                        *cidx = *cidx + 2;
                        self.idxs.push(idx);

                        let word = ZWord::from((buf[2], buf[3]));
                        let next_chars = ZCharWord::new(word.into());

                        let idx = CharIdx::Abbreviation(32 * (aidx as usize - 1), next_chars.first.value_of() as usize, 0);

                        self.read_next(idx, Alphabet::A0)
                    },
                    ZChar::Shift(a) => {
                        if chars.last_flag.is_set() {
                            // errrrrrr we tried to specify an alphabet
                            // for a char that doesn't exist
                            return None;
                        }

                        let alph = Alphabet::new(a);
                        *c = 0;
                        *cidx = *cidx + 2;

                        self.read_next(idx, alph)
                    },
                    ZChar::Char(zc) => {
                        if let Alphabet::A2 = alphabet {
                            if zc == 6 && chars.last_flag.is_set() {
                                return None;
                            }

                        }
                        let res = alphabet.get(zc);

                        if !chars.last_flag.is_set() {
                            *c = 0;
                            *cidx = *cidx + 2;
                            self.idxs.push(idx);
                        }

                        Some(res)
                    }
                }

            },
            _ => None, // something is wrong
        }
    }
}

impl<'a> Iterator for ZCharIterInner<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        println!("len {}", self.idxs.len());
        if let Some(c_idx) = self.idxs.pop() {
            println!("next called, {:?}", c_idx);
            if let Some(ch) = self.read_next(c_idx, Alphabet::A0) {
                Some(ch)
            } else {
                None
            }
        } else {
            None
        }
    }
}
