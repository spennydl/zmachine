use typenum::{Unsigned};

use std::marker::PhantomData;

pub struct ZWord(u16);

impl From<ZWord> for (u8, u8) {
    fn from(v: ZWord) -> (u8, u8) {
        ((v.0 >> 8) as u8, (v.0 | 0x00ff) as u8)
    }
}

impl From<(u8, u8)> for ZWord {
    fn from(v: (u8, u8)) -> ZWord {
        let (hi, lo) = v;
        ZWord((hi as u16) << 8 | lo as u16)
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
    N: std::ops::BitAnd<Output = N> + std::ops::Shr<Output = N> + PartialOrd + PartialEq + Eq + Copy,
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
}
