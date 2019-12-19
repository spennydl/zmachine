use crate::bits::ZWord;

bitstruct! {
    ZCharWord: u16 {
        last_flag: ZCharLastFlag, Width = U1, Offset = U15,
        first: ZCharFirst, Width = U5, Offset = U10,
        second: ZCharSecond, Width = U5, Offset = U5,
        third: ZCharThird, Width = U5, Offset = U0
    }
}

struct ZCharIter<'a> {
    mem: &'a [u8],
    word_idx: Option<usize>,
    char_idx: usize
}

impl<'a> ZCharIter<'a> {
    pub fn new(mem: &'a [u8]) -> Self {
        ZCharIter { mem, word_idx: Some(0), char_idx: 0 }
    }

    fn next_char(&mut self) {
        if self.char_idx == 2 {
            self.char_idx = 0;
        } else {
            self.char_idx += 1;
        }
    }
}

impl<'a> Iterator for ZCharIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(word_idx) = self.word_idx.take() {
            let word: ZWord = (self.mem[word_idx], self.mem[word_idx + 1]).into();
            let zch = ZCharWord::new(word.into());

            let idx = self.char_idx;
            self.next_char();
            
            match idx {
                0 => {
                    self.word_idx.replace(word_idx);
                    Some(zch.first.value_of() as u8)
                },
                1 => {
                    self.word_idx.replace(word_idx);
                    Some(zch.second.value_of() as u8)
                },
                2 => {
                    if !zch.last_flag.is_set() {
                        self.word_idx.replace(word_idx + 2);
                    }
                    Some(zch.third.value_of() as u8)
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

static ALPH_A0: &'static str = "abcdefghijklmnopqrstuvwxyz";
static ALPH_A1: &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
static ALPH_A2: &'static str = " \n0123456789.,!?_#'\"/\\-:()";

#[derive(Debug, PartialEq)]
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

    fn lookup(n: u8) -> (Alphabet, u8) {
        if let Some(idx) = ALPH_A0.chars().position(|c| c as u8 == n) {
            (Alphabet::A0, idx as u8 + 6)
        } else if let Some(idx) = ALPH_A1.chars().position(|c| c as u8 == n) {
            (Alphabet::A1, idx as u8 + 6)
        } else if let Some(idx) = ALPH_A2.chars().position(|c| c as u8 == n) {
            (Alphabet::A2, idx as u8 + 6)
        } else {
            (Alphabet::A0, 0)
        }
    }
}

pub enum ZChar {
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

    pub fn encode(c: u8) -> Vec<Self> {
        let mut ret: Vec<ZChar> = Vec::new();
        let (alph, ch) = Alphabet::lookup(c);

        match alph {
            Alphabet::A0 => {
                ret.push(ZChar::Char(ch));
                ret
            },
            Alphabet::A1 => {
                ret.push(ZChar::Shift(4));
                ret.push(ZChar::Char(ch));
                ret
            },
            Alphabet::A2 => {
                ret.push(ZChar::Shift(5));
                ret.push(ZChar::Char(ch));
                ret
            },
        }
    }

    pub fn value(&self) -> &u8 {
        match self {
            ZChar::Char(c)
            | ZChar::Abbrev(c)
            | ZChar::Shift(c) => c,
        }
    }
}

impl From<ZChar> for u8 {
    fn from(zc: ZChar) -> u8 {
        match zc {
            ZChar::Char(c)
            | ZChar::Abbrev(c)
            | ZChar::Shift(c) => c,
        }
    }
}

struct ZSCIIChar(Option<u8>, Option<u8>);

impl ZSCIIChar {
    fn new() -> Self {
        ZSCIIChar(None, None)
    }

    fn push_raw_zchar(&mut self, c: u8) {
        if self.0.is_some() {
            self.1.replace(c);
        } else {
            self.0.replace(c);
        }
    }

    fn get(&mut self) -> Option<char> {
        self.0.iter()
            .zip(self.1.iter())
            .map(|(hi, lo)| ((hi << 5) | lo) as char)
            .next()
    }
}

#[derive(Debug)]
pub struct ZString {
    len_words: usize,
    string: String
}

impl ZString {
    pub fn new<'a>(mem: &'a [u8], addr: usize, abbrev_table: &'a [u8]) -> ZString {
        let zchars: Vec<char> = vec![];
        let iter = ZCharIter::new(&mem[addr..]);

        let (n_chars, zchars) = ZString::parse_into(iter, mem, abbrev_table, zchars);

        let string: String = zchars.iter().collect();

        // TODO: n_chars is a hack
        ZString { len_words: (n_chars / 3) * 2, string }
    }

    fn parse_into<'a>(iter: ZCharIter, mem: &'a [u8], abbrev_table: &'a [u8], mut chars: Vec<char>) -> (usize, Vec<char>) {
        let mut alph = Alphabet::A0;
        let mut zscii: Option<ZSCIIChar> = None;
        let mut abbrev_idx: Option<usize> = None;
        let mut n_chars: usize = 0;

        for (i, zc) in iter.enumerate() {
            n_chars = i + 1;

            if let Some(aidx) = abbrev_idx.take() {
                let idx = (aidx + zc as usize) * 2;
                let addr: u16 = ZWord::from((abbrev_table[idx], abbrev_table[idx + 1])).into();
                let abbrev_iter = ZCharIter::new(&mem[addr as usize * 2..]);
                let (_, chs) = ZString::parse_into(abbrev_iter, mem, abbrev_table, chars);
                chars = chs;
            } else {
                match ZChar::new(zc) {
                    ZChar::Shift(shift_char) => {
                        alph = Alphabet::new(shift_char);
                    },
                    ZChar::Abbrev(aidx) => {
                        abbrev_idx.replace(32 * (aidx as usize - 1));
                    },
                    ZChar::Char(c) => {
                        if let Some(ref mut zsc) = zscii {
                            zsc.push_raw_zchar(c);
                            if let Some(c) = zsc.get() {
                                chars.push(c);
                                alph = Alphabet::A0;
                                zscii.take();
                            }
                        } else {
                            if c == 6 && alph == Alphabet::A2 {
                                zscii.replace(ZSCIIChar::new());
                            } else {
                                let ch = alph.get(c);
                                chars.push(ch);

                                alph = Alphabet::A0;
                            }
                        }
                    },
                }
            }
        }
        (n_chars, chars)
    }

    pub fn string(self) -> String {
        self.string
    }

    pub fn offset(&self) -> usize {
        self.len_words
    }
}

