bitstruct! {
    zi16: u16 {
        sign: SignBit, Width = U1, Offset = U15,
        val: SignedValue, Width = U15, Offset = U0
    }
}

pub struct ZSigned(zi16);

impl From<u16> for ZSigned {
    fn from(o: u16) -> ZSigned {
        ZSigned(zi16::new(o))
    }
}

impl ZSigned {
    pub fn value(&self) -> i16 {
        if self.0.sign.is_set() {
            (!self.0.val.value_of()) as i16 * -1
        } else {
            self.0.val.value_of() as i16
        }
    }
}

