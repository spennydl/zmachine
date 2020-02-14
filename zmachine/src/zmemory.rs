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
        let idx = idx * 2;

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

    fn set_attr(&mut self, attr_num: u8) {
        self.attributes = self.attributes | (0x80000000 >> attr_num as u32);
        let bytes = self.attributes.to_be_bytes();
        let data = self.data.as_mut();
        data[0] = bytes[0];
        data[1] = bytes[1];
        data[2] = bytes[2];
        data[3] = bytes[3];
    }

    fn clear_attr(&mut self, attr_num: u8) {
        self.attributes = self.attributes & !(0x80000000 >> attr_num as u32);
        let bytes = self.attributes.to_be_bytes();
        let data = self.data.as_mut();
        data[0] = bytes[0];
        data[1] = bytes[1];
        data[2] = bytes[2];
        data[3] = bytes[3];
    }
}

bitstruct! {
    PropertySize: u8 {
        size: PropSize, Width = U3, Offset = U5,
        number: PropNum, Width = U5, Offset = U0
    }
}

struct ZObjectProperty<'a, T: AsRef<[u8]>> {
    size: PropertySize,
    data: T,
    _lifetime: std::marker::PhantomData<&'a T>
}


impl<'a, T: AsRef<[u8]>> ZObjectProperty<'a, T> {
    fn new(size: u8, data: T) -> ZObjectProperty<'a, T> {
        ZObjectProperty {
            data,
            size: PropertySize::new(size),
            _lifetime: std::marker::PhantomData,
        }
    }

    fn get(&self) -> u16 {
        let size = self.size.size.value_of();
        let data = self.data.as_ref();

        if size == 0 {
            let lo = data[0];
            u16::from_be_bytes([0, lo])
        } else {
            let hi = data[0];
            let lo = data[1];
            u16::from_be_bytes([hi, lo])
        }
    }
}

impl<'a, T: AsRef<[u8]> + AsMut<[u8]>> ZObjectProperty<'a, T> {
    fn put(&mut self, val: u16) {
        let [hi, lo] = val.to_be_bytes();
        let data = self.data.as_mut();
        if self.size.size.value_of() == 0 {
            data[0] = lo;
        } else { // there's another condition here?
            data[0] = hi;
            data[1] = lo;
        }
    }
}

pub(crate) struct ZObjectProps<'a, T: AsRef<[u8]>> {
    props: T,
    _lifetime: std::marker::PhantomData<&'a T>
}

impl<'a, T: AsRef<[u8]>> ZObjectProps<'a, T> {
    fn new(props: T) -> ZObjectProps<'a, T> {
        ZObjectProps { props, _lifetime: std::marker::PhantomData }
    }

    fn get(self, num: u8) -> Option<u16> {
        let props = self.props.as_ref();
        let mut idx = 0;
        loop {
            let prop_size = PropertySize::new(props[idx]);
            let prop_num = prop_size.number.value_of();

            if prop_num == num {
                let prop = ZObjectProperty::new(props[idx], &props[idx + 1..]);
                return Some(prop.get());
            } else if prop_size.get() == 0 {
                return None;
            }
            idx += prop_size.size.value_of() as usize + 2;
        }
    }

    fn idx_of(self, num: u8) -> Option<u16> {
        let props = self.props.as_ref();
        let mut idx = 0;
        loop {
            let prop_size = PropertySize::new(props[idx]);
            let prop_num = prop_size.number.value_of();

            if prop_num == num {
                return Some(idx as u16 + 1); // skip size byte
            } else if prop_size.get() == 0 {
                return None;
            }
            idx += prop_size.size.value_of() as usize + 2;
        }
    }

    fn after(self, num: u8) -> Option<u8> {
        let props = self.props.as_ref();
        let mut idx = 0;

        if num == 0 {
            let prop = PropertySize::new(props[idx]);
            if prop.get() == 0 {
                return None;
            }
            return Some(prop.number.value_of());
        }

        loop {
            let prop_size = PropertySize::new(props[idx]);
            let prop_num = prop_size.number.value_of();

            if prop_num == num {
                idx += prop_size.size.value_of() as usize + 2;
                let next_prop = PropertySize::new(props[idx]);
                return Some(next_prop.number.value_of());
            } else if prop_size.get() == 0 {
                return None;
            }
            idx += prop_size.size.value_of() as usize + 2;
        }
    }
}

impl<'a, T: AsRef<[u8]> + AsMut<[u8]>> ZObjectProps<'a, T> {
    fn put(mut self, num: u8, val: ZWord) {
        let props = self.props.as_mut();
        let mut idx = 0;
        loop {
            let prop_size = PropertySize::new(props[idx]);
            let prop_num = prop_size.number.value_of();

            if prop_num == num {
                let mut prop = ZObjectProperty::new(props[idx], &mut props[idx + 1..]);
                prop.put(val.into());
                return;
            } else if prop_size.get() == 0 {
                return;
            }
            idx += prop_size.size.value_of() as usize + 2;
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
        if obj == 0 {
            panic!("read from obj 0");
        } else {
            let table = self.table.as_ref();
            let objects = &table[62..]; // skip defaults

            let obj_idx = (obj as usize - 1) * 9; //entries are 9 bytes

            ZObjectEntry::new(&objects[obj_idx..obj_idx + 9])
        }
    }

    fn get_prop_default(&self, prop_num: u8) -> u16 {
        if prop_num == 0 {
            panic!("tried to read property 0");
        }
        let table = self.table.as_ref();
        let prop_idx = (prop_num - 1) as usize * 2;
        let hi = table[prop_idx];
        let lo = table[prop_idx + 1];

        u16::from_be_bytes([hi, lo])
    }

}

impl<'a, T: 'a + AsRef<[u8]> + AsMut<[u8]>> ZObjectTable<'a, T> {
    fn get_object_mut(&mut self, obj: u8) -> ZObjectEntry<'a, &mut [u8]> {
        if obj == 0 {
            panic!("reading from obj 0")
        } else {
            let table = self.table.as_mut();
            let objects = &mut table[62..];

            let obj_idx = (obj as usize - 1) * 9; //entries are 9 bytes

            ZObjectEntry::new(&mut objects[obj_idx..obj_idx + 9])
        }
    }
}

pub(crate) struct ZDictionary {
    addr: usize,
    entry_len: usize,
    entries: Vec<u32>,
    separators: Vec<u8>,
}

impl ZDictionary {
    pub fn new(mem: &[u8], addr: usize) -> ZDictionary {
        let sep_len = mem[0];

        let separators: Vec<u8> = (0..sep_len)
            .map(|i| { mem[i as usize + 1] })
            .collect();

        let mut idx = sep_len as usize + 1;
        let entry_len = mem[idx]; idx += 1;
        let n_entries = u16::from_be_bytes([mem[idx], mem[idx + 1]]) as usize;
        idx += 2;
        let entries: Vec<u32> = (0..n_entries * entry_len as usize)
            .step_by(entry_len as usize)
            .map(|i| { u32::from_be_bytes(mem[idx + i..idx + i + 4].try_into().unwrap()) })
            .collect();

        ZDictionary { addr: addr + idx, entry_len: entry_len as usize, separators, entries }
    }

    pub fn separators(&self) -> &Vec<u8> {
        &self.separators
    }

    pub fn lookup(&self, word: &u32) -> Option<usize> {
        self.entries.iter()
            .position(|e| *e == *word)
            .map(|idx| {
                self.addr + (idx * self.entry_len)
            })
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
        //println!("Resetting mem, len: {}", data.len());
        self.bytes = data;

        let globals_idx = self.read_word(0x0C);
        self.globals_idx = u16::from(globals_idx) as usize;
        //println!("globals at {:x}", self.globals_idx);

        let objects_idx = self.read_word(0x0A);
        self.objects_idx = u16::from(objects_idx) as usize;
        //println!("objects at {:x}", self.objects_idx);

        let abbrev_idx = self.read_word(0x18);
        self.abbrev_idx = u16::from(abbrev_idx) as usize;
        //println!("abbreviations table at {:x}", self.abbrev_idx);

        let dictionary_idx = self.read_word(0x08);
        self.dictionary_idx = u16::from(dictionary_idx) as usize;
        //println!("dictionary table at {:x}", self.dictionary_idx);
    }

    fn set_object_parent(&mut self, obj_num: u8, new_parent: u8) {
        //println!("setting obj {} parent to {}", obj_num, new_parent);
        ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object_mut(obj_num)
            .set_parent(new_parent);
    }

    fn set_object_child(&mut self, obj_num: u8, new_child: u8) {
        //println!("setting obj {} child to {}", obj_num, new_child);
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
            .unwrap_or(0)
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

    pub(crate) fn get_object_name(&self, obj_num: u8) -> Option<String> {
        if obj_num == 0 {
            None
        } else {
            let address: u16 = ZObjectTable::new(&self.bytes[self.objects_idx..])
                .get_object(obj_num)
                .properties.into();
            let (name, _) = self.read_string(address as usize + 1);

            Some(name)
        }
    }

    pub(crate) fn remove_obj(&mut self, obj: u8) {
        let parent = self.get_object_parent(obj);
        self.set_object_parent(obj, 0);

        if parent == 0 {
            return;
        }

        if let Some(child) = self.get_object_child(parent) {
            if obj == child {
                let sib = self.get_object_sibling(obj).unwrap_or(0);
                self.set_object_child(parent, sib);
                self.set_object_sibling(obj, 0);
            } else if child != 0 { // shouldn't need to check for 0?
                let mut current = child;
                while let Some(sib) = self.get_object_sibling(current) {
                    if sib == obj {
                        let new_sib = self.get_object_sibling(sib).unwrap_or(0);
                        self.set_object_sibling(current, new_sib);
                        self.set_object_sibling(obj, 0);
                        current = new_sib;
                    } else {
                        current = sib;
                    }
                    if current == 0 {
                        break;
                    }
                }
            }
        }
    }

    pub(crate) fn insert_object(&mut self, obj: u8, dest: u8) {
        self.remove_obj(obj);
        let prev_child = self.get_object_child(dest);
        self.set_object_parent(obj, dest);
        if let Some(sib) = prev_child {
            self.set_object_sibling(obj, sib);
        }
        self.set_object_child(dest, obj);
    }

    pub(crate) fn test_attr(&self, obj_num: u8, attr: u8) -> bool {
        if obj_num == 0 {
            return false;
        }
        let table = ZObjectTable::new(&self.bytes[self.objects_idx..]);
        let obj = table.get_object(obj_num);

        let ret = obj.attributes & (0x80000000 >> attr as u32) > 0;
        ret
    }

    pub(crate) fn set_attr(&mut self, obj_num: u8, attr: u8) {
        ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object_mut(obj_num)
            .set_attr(attr);
    }

    pub(crate) fn clear_attr(&mut self, obj_num: u8, attr: u8) {
        ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object_mut(obj_num)
            .clear_attr(attr);
    }

    pub(crate) fn put_prop(&mut self, obj_num: u8, prop_num: u8, val: ZWord) {
        let address: u16 = ZObjectTable::new(&mut self.bytes[self.objects_idx..])
            .get_object(obj_num)
            .properties.into();
        let n_header = self.bytes[address as usize] as u16 * 2;
        let addr = address as usize + n_header as usize + 1;

        let property = ZObjectProps::new(&mut self.bytes[addr..]);

        property.put(prop_num, val);
    }

    pub(crate) fn get_prop(&self, obj_num: u8, prop_num: u8) -> ZWord {
        let address: u16 = ZObjectTable::new(&self.bytes[self.objects_idx..])
            .get_object(obj_num)
            .properties.into();

        let n_header = self.bytes[address as usize] as usize * 2;
        let addr = address as usize + n_header + 1;

        let property = ZObjectProps::new(&self.bytes[addr..]);

        if let Some(p) = property.get(prop_num) {
            p.into()
        } else {
            ZObjectTable::new(&self.bytes[self.objects_idx..])
                .get_prop_default(prop_num).into()
        }
    }

    pub(crate) fn get_next_prop(&self, obj_num: u8, prop_num: u8) -> u8 {
        let address: u16 = ZObjectTable::new(&self.bytes[self.objects_idx..])
            .get_object(obj_num)
            .properties.into();
        let n_header = self.bytes[address as usize] as usize * 2;
        let addr = address as usize + n_header + 1;

        let property = ZObjectProps::new(&self.bytes[addr..]);

        property.after(prop_num).expect("Couldn't get next property")
    }

    pub(crate) fn get_prop_addr(&self, obj_num: u8, prop_num: u8) -> u16 {
        let address: u16 = ZObjectTable::new(&self.bytes[self.objects_idx..])
            .get_object(obj_num)
            .properties.into();
        let n_header = self.bytes[address as usize] as usize * 2;
        let addr = address as usize + n_header + 1;

        let property = ZObjectProps::new(&self.bytes[addr..]);

        property.idx_of(prop_num).unwrap_or(0)
    }

    pub(crate) fn get_prop_len(&self, prop_addr: u16) -> u16 {
        if prop_addr == 0 {
            0
        } else {
            let size_byte = self.bytes[prop_addr as usize - 1]; // size byte!
            let prop_size = PropertySize::new(size_byte);

            let size = prop_size.size.value_of() as u16 + 1;
            size
        }
    }

    pub(crate) fn read_string(&self, addr: usize) -> (String, usize) {
        let zstr = ZString::new(&self.bytes[..], addr as usize, &self.bytes[self.abbrev_idx..]);
        let offset = zstr.offset();

        (zstr.string(), offset)
    }

    pub(crate) fn write_text(&mut self, addr: u16, text: &str) {
        let start_addr = addr as usize + 1;
        let mut end: usize = 0;
        for (i, c) in text.chars().enumerate() {
            self.set_byte(start_addr + i, (c as u8).to_ascii_lowercase());
            end = i;
        }
        self.set_byte(start_addr + end, 0);
    }

    pub(crate) fn dictionary(&self) -> ZDictionary {
        let dict = &self.bytes[self.dictionary_idx..];
        ZDictionary::new(dict, self.dictionary_idx)
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

