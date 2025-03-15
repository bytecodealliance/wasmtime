use crate::obj::ELF_WASMTIME_STACK_MAP;
use crate::prelude::*;
use cranelift_bitset::CompoundBitSet;
use object::write::{Object, StandardSegment};
use object::{LittleEndian, SectionKind, U32Bytes};

/// Builder for the `ELF_WASMTIME_STACK_MAP` section in compiled executables.
///
/// This format is parsed by `crate::stack_map`.
///
/// The current layout of the format is:
///
/// * A 4-byte little-endian count of how many stack maps there are: `N`.
/// * `N` 4-byte little endian program counters, in ascending order.
/// * `N` 4-byte little endian offsets.
/// * Stack map data as 4-bit little endian integers.
///
/// The "offset" is an offset into the "stack map data" field which are encoded
/// as:
///
/// * A 4-byte little-endian frame size
/// * A 4-byte little-endian count of remaining bits: `M`
/// * `M` 4-byte little-endian integers as a bit map.
///
/// Entries in the bit map represent `stack_slot / 4` so must be muliplied by 4
/// to get the actual stack offset entry.
#[derive(Default)]
pub struct StackMapSection {
    pcs: Vec<U32Bytes<LittleEndian>>,
    pointers_to_stack_map: Vec<U32Bytes<LittleEndian>>,
    stack_map_data: Vec<U32Bytes<LittleEndian>>,
    last_offset: u32,
}

impl StackMapSection {
    /// Appends stack map information for `pc` which has the specified
    /// `frame_size` and `offsets` are the active GC references.
    pub fn push(&mut self, pc: u64, frame_size: u32, offsets: impl ExactSizeIterator<Item = u32>) {
        // NB: for now this only supports <=4GB text sections in object files.
        // Alternative schemes will need to be created for >32-bit offsets to
        // avoid making this section overly large.
        let pc = u32::try_from(pc).unwrap();

        // Sanity-check to ensure that functions are pushed in-order, otherwise
        // the `pcs` array won't be sorted which is our goal.
        assert!(pc >= self.last_offset);
        self.last_offset = pc;

        if offsets.len() == 0 {
            return;
        }

        // Record parallel entries in `pcs`/`pointers_to_stack_map`.
        self.pcs.push(U32Bytes::new(LittleEndian, pc));
        self.pointers_to_stack_map.push(U32Bytes::new(
            LittleEndian,
            u32::try_from(self.stack_map_data.len()).unwrap(),
        ));

        // The frame data starts with the frame size and is then followed by
        // `offsets` represented as a bit set.
        self.stack_map_data
            .push(U32Bytes::new(LittleEndian, frame_size));

        let mut bits = CompoundBitSet::<u32>::default();
        for offset in offsets {
            assert!(offset % 4 == 0);
            bits.insert((offset / 4) as usize);
        }
        let count = bits.iter_words().count();
        self.stack_map_data
            .push(U32Bytes::new(LittleEndian, count as u32));
        for word in bits.iter_words() {
            self.stack_map_data.push(U32Bytes::new(LittleEndian, word));
        }
    }

    /// Finishes encoding this section into the `Object` provided.
    pub fn append_to(self, obj: &mut Object) {
        if self.pcs.is_empty() {
            return;
        }
        let section = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_WASMTIME_STACK_MAP.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );

        // NB: this matches the encoding expected by `lookup` in the
        // `crate::stack_maps` module.
        let amt = u32::try_from(self.pcs.len()).unwrap();
        obj.append_section_data(section, &amt.to_le_bytes(), 1);
        obj.append_section_data(section, object::bytes_of_slice(&self.pcs), 1);
        obj.append_section_data(
            section,
            object::bytes_of_slice(&self.pointers_to_stack_map),
            1,
        );
        obj.append_section_data(section, object::bytes_of_slice(&self.stack_map_data), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack_map::StackMap;
    use object::{Object, ObjectSection};

    fn roundtrip(maps: &[(u64, u32, &[u32])]) {
        let mut section = StackMapSection::default();
        for (pc, frame, offsets) in maps {
            println!("append {pc}");
            section.push(*pc, *frame, offsets.iter().copied());
        }
        let mut object = object::write::Object::new(
            object::BinaryFormat::Elf,
            object::Architecture::X86_64,
            object::Endianness::Little,
        );
        section.append_to(&mut object);
        let elf = object.write().unwrap();

        let image = object::File::parse(&elf[..]).unwrap();
        let data = image
            .sections()
            .find(|s| s.name().ok() == Some(ELF_WASMTIME_STACK_MAP))
            .unwrap()
            .data()
            .unwrap();

        for (pc, frame, offsets) in maps {
            println!("lookup {pc}");
            let map = match StackMap::lookup(*pc as u32, data) {
                Some(map) => map,
                None => {
                    assert!(offsets.is_empty());
                    continue;
                }
            };
            assert_eq!(map.frame_size(), *frame);

            let map_offsets = map.offsets().collect::<Vec<_>>();
            assert_eq!(map_offsets, *offsets);
        }

        let mut expected = maps.iter();
        'outer: for (pc, map) in StackMap::iter(data).unwrap() {
            while let Some((expected_pc, expected_frame, expected_offsets)) = expected.next() {
                if expected_offsets.is_empty() {
                    continue;
                }
                assert_eq!(*expected_pc, u64::from(pc));
                assert_eq!(*expected_frame, map.frame_size());
                let offsets = map.offsets().collect::<Vec<_>>();
                assert_eq!(offsets, *expected_offsets);
                continue 'outer;
            }
            panic!("didn't find {pc:#x} in expected list");
        }
        assert!(expected.next().is_none());
    }

    #[test]
    fn roundtrip_many() {
        roundtrip(&[(0, 4, &[0])]);
        roundtrip(&[
            (0, 4, &[0]),
            (4, 200, &[0, 4, 20, 180]),
            (200, 20, &[12]),
            (600, 0, &[]),
            (800, 20, &[0, 4, 8, 12, 16]),
            (1200, 2000, &[1800, 1804, 1808, 1900]),
        ]);
    }
}
