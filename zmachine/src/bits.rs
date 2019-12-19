use typenum::{Unsigned};

use std::marker::PhantomData;

pub struct ZWord(u16);

impl From<ZWord> for (u8, u8) {
    fn from(v: ZWord) -> (u8, u8) {
        let [ hi, lo ] = v.0.to_be_bytes();
        (hi, lo)
    }
}

impl From<(u8, u8)> for ZWord {
    fn from(v: (u8, u8)) -> ZWord {
        let (hi, lo) = v;
        ZWord(u16::from_be_bytes([hi, lo]))
    }
}

impl From<u16> for ZWord {
    fn from(v: u16) -> ZWord {
        ZWord(v)
    }
}

impl From<ZWord> for u16 {
    fn from(v: ZWord) -> u16 {
        v.0
    }
}

pub trait To<N> {
    fn to() -> N;
}

macro_rules! impl_to {
    ($n:ty, $c:ident) => {
        impl<T: Unsigned> To<$n> for T {
            fn to() -> $n {
                T::$c
            }
        }
    }
}

impl_to!(u8, U8);
impl_to!(u16, U16);

macro_rules! bitstruct {
    ($($name:ident: $numtype:ty {
        $($field:ident: $type:ident, Width = $W:ident, Offset = $O:ident),+
    }),+) => {
        use typenum::*;
        use crate::bits::BitField;
        $(
            $(
                type $type = BitField<$numtype, $W, $O, op!(((U1 << $W) - U1) << $O)>;
            )+

            #[derive(Debug)]
            pub struct $name {
                $(pub $field: $type),+
            }

            impl $name {
                pub fn new(val: $numtype) -> $name {
                    $name {
                        $($field: $type::new(val)),+
                    }
                }

                pub fn get(&self) -> $numtype {
                    let mut output: $numtype = 0;
                    $(
                        self.$field.write(&mut output);
                    )+

                    output
                }
            }
        )+
    };
}

#[derive(Debug)]
pub struct BitField<N, W: Unsigned, O: Unsigned, M: Unsigned> {
    val: N,
    _width: PhantomData<W>,
    _offset: PhantomData<O>,
    _mask: PhantomData<M>,
}

impl<N, W: Unsigned, O: Unsigned, M: Unsigned> BitField<N, W, O, M> 
where
    N: std::ops::BitOr<Output = N> + std::ops::Not<Output = N> + std::ops::BitAnd<Output = N> + std::ops::Shr<Output = N> + std::ops::Shl<Output = N> + PartialOrd + PartialEq + Eq + Copy,
    W: To<N>,
    O: To<N>,
    M: To<N> {
    pub fn new(val: N) -> BitField<N, W, O, M> {
        BitField {
            val,
            _width: PhantomData,
            _offset: PhantomData,
            _mask: PhantomData,
        }
    }

    pub fn is_set(&self) -> bool {
        self.val & M::to() == M::to()
    }

    pub fn value_of(&self) -> N {
        (self.val & M::to()) >> O::to()
    }

    pub fn set(&mut self, val: N) {
        let val = (val << O::to()) & M::to();
        self.val = self.val & !M::to() | val;
    }

    pub fn write(&self, out: &mut N) {
        *out = *out & !M::to() | (self.val & M::to());
    }
}
