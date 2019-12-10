use crate::bits::ZWord;
use crate::zstr::ZString;
use typenum::{U0, U3, U5};
use std::convert::AsRef;
use std::convert::TryInto;

pub struct ZGlobals<T: AsRef<[u8]>> {
    table: T
}

impl<T: AsRef<[u8]>> ZGlobals<T> {
    fn new(table: T) -> ZGlobals<T> {
        ZGlobals { table }
    }

    fn get(&self, idx: usize) -> ZWord {
        let table = self.table.as_ref();

        let idx = idx * 2;
        (table[idx], table[idx + 1]).into()
    }

}

impl <T: AsRef<[u8]> + AsMut<[u8]>> ZGlobals<T> {
    fn set(&mut self, idx: usize, val: ZWord) {
        let table = self.table.as_mut();

        let (hi, lo) = val.into();
        table[idx] = hi;
        table[idx + 1] = lo;
    }
}

pub(crate) struct ZObjectEntry<'a, T: 'a + AsRef<[u8]>> {
    attributes: u32,
    properties: u16,
    data: T,
    _lifetime: std::marker::PhantomData<&'a T>
}

impl<'a, T: 'a + AsRef<[u8]>> ZObjectEntry<'a, T> {
    fn new(d: T) -> ZObjectEntry<'a, T> {
        let data = d.as_ref();
        let attributes = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let properties: u16 = u16::from_be_bytes([data[7], data[8]]);

        ZObjectEntry {
            attributes,
            properties: properties,
            data: d,
            _lifetime: std::marker::PhantomData
        }
    }

    fn parent_num(&self) -> Option<u8> {
        let data = self.data.as_ref();
        let num = data[4];
        if num != 0 {
            Some(num)
        } else {
            None
        }
    }

    fn child_num(&self) -> Option<u8> {
        let data = self.data.as_ref();
        let num = data[6];
        if num != 0 {
            Some(num)
        } else {
            None
        }
    }

    fn sibling_num(&self) -> Option<u8> {
        let data = self.data.as_ref();
        let num = data[5];
        if num != 0 {
            Some(num)
        } else {
            None
        }
    }
}

impl<'a, T: 'a + AsRef<[u8]> + AsMut<[u8]>> ZObjectEntry<'a, T> {
    fn set_parent(&mut self, parent_num: u8) {
        let data = self.data.as_mut();
        data[4] = parent_num;
    }

    fn set_child(&mut self, child_num: u8) {
        let data = self.data.as_mut();
        data[6] = child_num;
    }

    fn set_sibling(&mut self, sibling_num: u8) {
        let data = self.data.as_mut();
        data[5] = sibling_num;
    }
}

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
}

pub(crate) struct ZObjectProps<'a> {
    props: &'a mut [u8]
}

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

pub(crate) struct ZObjectTable<'a, T: 'a + AsRef<[u8]>> {
    table: T,
    _lifetime: std::marker::PhantomData<&'a T>
}

impl<'a, T: 'a + AsRef<[u8]>> ZObjectTable<'a, T> {
    fn new(table: T) -> ZObjectTable<'a, T> {
        ZObjectTable { table, _lifetime: std::marker::PhantomData }
    }

    fn get_object(&self, obj: u8) -> ZObjectEntry<'a, &[u8]> {
        // skip the defaults table
        let table = self.table.as_ref();
        let objects = &table[62..];
        let obj_idx = obj as usize * 9; //entries are 9 bytes

        ZObjectEntry::new(&objects[obj_idx..obj_idx + 9])
    }
}

impl<'a, T: 'a + AsRef<[u8]> + AsMut<[u8]>> ZObjectTable<'a, T> {
    fn get_object_mut(&mut self, obj: u8) -> ZObjectEntry<'a, &mut [u8]> {
        // skip the defaults table
        let table = self.table.as_mut();
        let objects = &mut table[62..];
        let obj_idx = obj as usize * 9; //entries are 9 bytes

        ZObjectEntry::new(&mut objects[obj_idx..obj_idx + 9])
    }
}

pub(crate) struct ZDictionary {
    entries: Vec<u32>,
    separators: Vec<u8>,
}

impl ZDictionary {
    pub fn new(mem: &[u8]) -> ZDictionary {
        let sep_len = mem[0];

        let separators: Vec<u8> = (0..sep_len)
            .map(|i| { mem[i as usize + 1] })
            .collect();

        let mut idx = sep_len as usize + 1;
        let entry_len = mem[idx]; idx += 1;
        let n_entries = u16::from_be_bytes([mem[idx], mem[idx + 1]]) as usize;
        idx += 2;

        let entries: Vec<u32> = (0..n_entries)
            .step_by(entry_len as usize)
            .map(|i| { u32::from_be_bytes(mem[idx + i..idx + i + 4].try_into().unwrap()) })
            .collect();
        
        ZDictionary { separators, entries }
    }

    pub fn separators(&self) -> &Vec<u8> {
        &self.separators
    }

    pub fn contains(&self, word: &u32) -> bool {
        self.entries.contains(&word)
    }
}

#[derive(Default, Debug)]
pub(crate) struct ZMemory {
    bytes: Vec<u8>,
    globals_idx: usize,
    objects_idx: usize,
    abbrev_idx: usize,
    dictionary_idx: usize
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

        let abbrev_idx = self.read_word(0x18);
        self.abbrev_idx = u16::from(abbrev_idx) as usize;
        println!("abbreviations table at {:x}", self.abbrev_idx);

        let abbrev_idx = self.read_word(0x18);
        self.abbrev_idx = u16::from(abbrev_idx) as usize;
        println!("abbreviations table at {:x}", self.abbrev_idx);

        let dictionary_idx = self.read_word(0x08);
        self.dictionary_idx = u16::from(dictionary_idx) as usize;
        println!("dictionary table at {:x}", self.dictionary_idx);
    }

    fn set_object_parent(&mut self, obj_num: u8, new_parent: u8) {
        ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object_mut(obj_num)
            .set_parent(new_parent);
    }

    fn set_object_child(&mut self, obj_num: u8, new_child: u8) {
        ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object_mut(obj_num)
            .set_child(new_child);
    }

    fn set_object_sibling(&mut self, obj_num: u8, new_sibling: u8) {
        ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object_mut(obj_num)
            .set_sibling(new_sibling);
    }

    pub(crate) fn get_object_parent(&self, obj_num: u8) -> u8 {
        ZObjectTable::new(&self.bytes[self.objects_idx..])
            .get_object(obj_num)
            .parent_num()
            .expect("object with no parent?")
    }

    pub(crate) fn get_object_child(&self, obj_num: u8) -> Option<u8> {
        ZObjectTable::new(&self.bytes[self.objects_idx..])
            .get_object(obj_num)
            .child_num()
    }

    pub(crate) fn get_object_sibling(&self, obj_num: u8) -> Option<u8> {
        ZObjectTable::new(&self.bytes[self.objects_idx..])
            .get_object(obj_num)
            .sibling_num()
    }

    pub(crate) fn insert_object(&mut self, obj: u8, dest: u8) {
        let prev_child = {
            let table = ZObjectTable::new(&self.bytes[self.objects_idx..]);
            let obj = table.get_object(dest);

            obj.child_num()
        };
        self.set_object_parent(obj, dest);
        if let Some(sib) = prev_child {
            self.set_object_sibling(obj, sib);
            self.set_object_sibling(sib, obj);
        }
        self.set_object_child(dest, obj);
    }

    pub(crate) fn test_attr(&self, obj_num: u8, attr: u8) -> bool {
        let table = ZObjectTable::new(&self.bytes[self.objects_idx..]);
        let obj = table.get_object(obj_num);

        obj.attributes & (1 << attr as u32) > 0
    }

    pub(crate) fn put_prop(&mut self, obj_num: u8, prop_num: u8, val: ZWord) {
        let address: u16 = ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object(obj_num)
            .properties.into();

        let property = ZObjectProps::new(&mut self.bytes[address as usize..]);

        property.put(prop_num, val);
    }

    pub(crate) fn read_string(&self, addr: u16) -> (String, usize) {
        let zstr = ZString::new(&self.bytes[..], addr as usize, &self.bytes[self.abbrev_idx..]);
        let offset = zstr.offset();

        (zstr.string(), offset)
    }

    pub(crate) fn write_text(&mut self, addr: u16, text: &str) {
        let start_addr = addr as usize + 1;
        let mut end: usize = 0;
        for (i, c) in text.chars().enumerate() {
            self.set_byte(start_addr + i, c as u8);
            end = i;
        }
        self.set_byte(start_addr + end, 0);
    }

    pub(crate) fn dictionary(&self) -> ZDictionary {
        let dict = &self.bytes[self.dictionary_idx..];
        ZDictionary::new(dict)
    }

    pub(crate) fn header(&self) -> &[u8] {
        &self.bytes[0..64]
    }

    pub(crate) fn global(&self, idx: usize) -> ZWord {
        ZGlobals::new(&self.bytes[self.globals_idx..])
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

    pub(crate) fn set_byte(&mut self, idx: usize, val: u8) {
        self.bytes[idx] = val;
    }

    pub(crate) fn slice(&self, idx: usize) -> &[u8] {
        &self.bytes[idx..]
    }
}

