use crate::bits::ZWord;
use typenum::{U0, U3, U5};

pub struct ZGlobals<'a> {
    table: &'a mut [u8]
}

impl<'a> ZGlobals<'a> {
    fn new(table: &'a mut [u8]) -> ZGlobals<'a> {
        ZGlobals { table }
    }

    fn get(&self, idx: usize) -> ZWord {
        let idx = idx * 2;
        (self.table[idx], self.table[idx + 1]).into()
    }

    fn set(&mut self, idx: usize, val: ZWord) {
        let (hi, lo) = val.into();
        self.table[idx] = hi;
        self.table[idx + 1] = lo;
    }
}

pub(crate) struct ZObjectEntry {
    attributes: u32,
    parent: u8,
    sib: u8,
    child: u8,
    properties: u16
}

impl ZObjectEntry {
    fn new(data: &[u8]) -> ZObjectEntry {
        let attributes = (data[0] as u32) << 24 | (data[1] as u32) << 16 | (data[2] as u32) << 8 | (data[3] as u32);
        let parent = data[4];
        let sib = data[5];
        let child = data[6];
        let properties: ZWord = (data[7], data[8]).into();

        ZObjectEntry {
            attributes,
            parent,
            sib,
            child,
            properties: properties.into()
        }
    }
}

/*
pub(crate) struct ZObjectIter<'a> {
    table: &'a mut [u8],
    idx: usize,
}

impl<'a> ZObjectIter<'a> {
    fn new(table: &'a mut [u8]) -> ZObjectIter {
        let idx = 62;

        ZObjectIter { table, idx }
    }
}

impl<'a> Iterator for ZObjectIter<'a> {
    type Item = ZObjectEntry;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
*/

bitstruct! {
    PropertySize: u8 {
        size: PropSize, Width = U3, Offset = U5,
        number: PropNum, Width = U5, Offset = U0
    }
}

struct ZObjectProperty<'a> {
    size: PropertySize,
    data: &'a mut [u8],
}

impl<'a> ZObjectProperty<'a> {
    fn new(size: u8, data: &'a mut [u8]) -> ZObjectProperty<'a> {
        ZObjectProperty {
            size: PropertySize::new(size),
            data
        }
    }

    fn put(&mut self, val: ZWord) {
        let (hi, lo) = val.into();
        if self.size.size.value_of() == 1 {
            self.data[0] = lo;
        } else { // there's another condition here?
            self.data[0] = hi;
            self.data[1] = lo;
        }
    }

    fn number(&self) -> u8 {
        self.size.number.value_of() as u8
    }
}

pub(crate) struct ZObjectProps<'a> {
    props: &'a mut [u8]
}

/*
struct ZObjectPropsIter<'a> {
    props: &'a mut [u8],
    idx: usize,
}

impl<'a> Iterator for ZObjectPropsIter<'a> {
    type Item = ZObjectProperty<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 9;

        let prop = ZObjectProperty::new(self.props[idx], &mut self.props[idx + 1..]);
        if prop.number() == 0 {
            None
        } else {
            Some(prop)
        }
    }
}
*/

impl<'a> ZObjectProps<'a> {
    fn new(props: &'a mut [u8]) -> ZObjectProps<'a> {
        ZObjectProps { props }
    }

    fn get(self, num: u8) -> Option<ZObjectProperty<'a>> {
        for idx in (0..32).step_by(9) {
            let prop_size = PropertySize::new(self.props[idx]);
            let prop_num = prop_size.number.value_of();
            if prop_num == 0 {
                return None;
            } else if prop_num == num {
                return Some(ZObjectProperty::new(self.props[idx], &mut self.props[idx + 1..]));
            }
        }
        None
    }

    fn put(self, num: u8, val: ZWord) {
        if let Some(mut prop) = self.get(num) {
            prop.put(val);
        }
    }
}

pub(crate) struct ZObjectTable<'a> {
    table: &'a mut [u8]
}

impl<'a> ZObjectTable<'a> {
    fn new(table: &'a mut [u8]) -> ZObjectTable<'a> {
        ZObjectTable { table }
    }

    fn get_object(&mut self, obj: usize) -> ZObjectEntry {
        // skip the defaults table
        let objects = &self.table[62..];
        let obj_idx = obj * 9; //entries are 9 bytes

        ZObjectEntry::new(&objects[obj_idx..obj_idx + 9])
    }
}

#[derive(Default, Debug)]
pub(crate) struct ZMemory {
    bytes: Vec<u8>,
    globals_idx: usize,
    objects_idx: usize,
}

impl ZMemory {

    pub(crate) fn read_word(&self, idx: usize) -> ZWord {
        (self.bytes[idx], self.bytes[idx + 1]).into()
    }

    pub(crate) fn read_byte(&self, idx: usize) -> u8 {
        self.bytes[idx]
    }

    pub(crate) fn reset(&mut self, data: Vec<u8>) {
        println!("Resetting mem, len: {}", data.len());
        self.bytes = data;

        let globals_idx = self.read_word(0x0C);
        self.globals_idx = u16::from(globals_idx) as usize;
        println!("globals at {:x}", self.globals_idx);

        let objects_idx = self.read_word(0x0A);
        self.objects_idx = u16::from(objects_idx) as usize;
        println!("objects at {:x}", self.objects_idx);
    }

    pub(crate) fn put_prop(&mut self, obj_idx: usize, prop_num: u8, val: ZWord) {
        let address: u16 = ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object(obj_idx)
            .properties.into();

        let property = ZObjectProps::new(&mut self.bytes[address as usize..]);

        property.put(prop_num, val);
    }

    pub(crate) fn header(&self) -> &[u8] {
        &self.bytes[0..64]
    }

    pub(crate) fn global(&mut self, idx: usize) -> ZWord {
        ZGlobals::new(&mut self.bytes[self.globals_idx..])
            .get(idx)
    }

    pub(crate) fn set_global(&mut self, idx: usize, val: ZWord) {
        ZGlobals::new(&mut self.bytes[self.globals_idx..])
            .set(idx, val);
    }

    pub(crate) fn set_word(&mut self, idx: usize, val: ZWord) {
        let (hi, lo) = val.into();

        self.bytes[idx] = hi;
        self.bytes[idx + 1] = lo;
    }

    pub(crate) fn slice(&self, idx: usize) -> &[u8] {
        &self.bytes[idx..]
    }
}

