//! Virtual Addressing Scheme for the Interpreter
//!
//! The interpreter uses virtual memory addresses for its memory operations. These addresses
//! are obtained by the various `_addr` instructions (e.g. `stack_addr`) and can be either 32 or 64
//! bits.
//!
//! Addresses are composed of 3 fields: "region", "entry" and offset.
//!
//! "region" refers to the type of memory that this address points to.
//! "entry" refers to which instance of this memory the address points to (e.g table1 would be
//! "entry" 1 of a `Table` region address).
//! The last field is the "offset", which refers to the offset within the entry.  
//!
//! The address has the "region" field as the 2 most significant bits. The following bits
//! are the "entry" field, the amount of "entry" bits depends on the size of the address and
//! the "region" of the address. The remaining bits belong to the "offset" field
//!
//! An example address could be a 32 bit address, in the `heap` region, which has 2 "entry" bits
//! this address would have 32 - 2 - 2 = 28 offset bits.
//!
//! The only exception to this is the "stack" region, where, because we only have a single "stack"
//! we have 0 "entry" bits, and thus is all offset.
//!
//! | address size | address kind | region value (2 bits) | entry bits (#) | offset bits (#) |
//! |--------------|--------------|-----------------------|----------------|-----------------|
//! | 32           | Stack        | 0b00                  | 0              | 30              |
//! | 32           | Heap         | 0b01                  | 2              | 28              |
//! | 32           | Table        | 0b10                  | 5              | 25              |
//! | 32           | GlobalValue  | 0b11                  | 6              | 24              |
//! | 64           | Stack        | 0b00                  | 0              | 62              |
//! | 64           | Heap         | 0b01                  | 6              | 56              |
//! | 64           | Table        | 0b10                  | 10             | 52              |
//! | 64           | GlobalValue  | 0b11                  | 12             | 50              |

use crate::state::MemoryError;
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{types, Type};
use std::convert::TryFrom;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AddressSize {
    _32,
    _64,
}

impl AddressSize {
    pub fn bits(&self) -> u64 {
        match self {
            AddressSize::_64 => 64,
            AddressSize::_32 => 32,
        }
    }
}

impl TryFrom<Type> for AddressSize {
    type Error = MemoryError;

    fn try_from(ty: Type) -> Result<Self, Self::Error> {
        match ty {
            types::I64 => Ok(AddressSize::_64),
            types::I32 => Ok(AddressSize::_32),
            _ => Err(MemoryError::InvalidAddressType(ty)),
        }
    }
}

/// Virtual Address region
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AddressRegion {
    Stack,
    Heap,
    Table,
    GlobalValue,
}

impl AddressRegion {
    pub fn decode(bits: u64) -> Self {
        assert!(bits < 4);
        match bits {
            0 => AddressRegion::Stack,
            1 => AddressRegion::Heap,
            2 => AddressRegion::Table,
            3 => AddressRegion::GlobalValue,
            _ => unreachable!(),
        }
    }

    pub fn encode(self) -> u64 {
        match self {
            AddressRegion::Stack => 0,
            AddressRegion::Heap => 1,
            AddressRegion::Table => 2,
            AddressRegion::GlobalValue => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Address {
    pub size: AddressSize,
    pub region: AddressRegion,
    pub entry: u64,
    pub offset: u64,
}

impl Address {
    pub fn from_parts(
        size: AddressSize,
        region: AddressRegion,
        entry: u64,
        offset: u64,
    ) -> Result<Self, MemoryError> {
        let entry_bits = Address::entry_bits(size, region);
        let offset_bits = Address::offset_bits(size, region);

        let max_entries = (1 << entry_bits) - 1;
        let max_offset = (1 << offset_bits) - 1;

        if entry > max_entries {
            return Err(MemoryError::InvalidEntry {
                entry,
                max: max_entries,
            });
        }

        if offset > max_offset {
            return Err(MemoryError::InvalidOffset {
                offset,
                max: max_offset,
            });
        }

        Ok(Address {
            size,
            region,
            entry,
            offset,
        })
    }

    fn entry_bits(size: AddressSize, region: AddressRegion) -> u64 {
        match (size, region) {
            // We only have one stack, so the whole address is offset
            (_, AddressRegion::Stack) => 0,

            (AddressSize::_32, AddressRegion::Heap) => 2,
            (AddressSize::_32, AddressRegion::Table) => 5,
            (AddressSize::_32, AddressRegion::GlobalValue) => 6,

            (AddressSize::_64, AddressRegion::Heap) => 6,
            (AddressSize::_64, AddressRegion::Table) => 10,
            (AddressSize::_64, AddressRegion::GlobalValue) => 12,
        }
    }

    fn offset_bits(size: AddressSize, region: AddressRegion) -> u64 {
        let region_bits = 2;
        let entry_bits = Address::entry_bits(size, region);
        size.bits() - entry_bits - region_bits
    }
}

impl TryFrom<Address> for DataValue {
    type Error = MemoryError;

    fn try_from(addr: Address) -> Result<Self, Self::Error> {
        let entry_bits = Address::entry_bits(addr.size, addr.region);
        let offset_bits = Address::offset_bits(addr.size, addr.region);

        let entry = addr.entry << offset_bits;
        let region = addr.region.encode() << (entry_bits + offset_bits);

        let value = region | entry | addr.offset;
        Ok(match addr.size {
            AddressSize::_32 => DataValue::I32(value as u32 as i32),
            AddressSize::_64 => DataValue::I64(value as i64),
        })
    }
}

impl TryFrom<DataValue> for Address {
    type Error = MemoryError;

    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        let addr = match value {
            DataValue::U32(v) => v as u64,
            DataValue::I32(v) => v as u32 as u64,
            DataValue::U64(v) => v,
            DataValue::I64(v) => v as u64,
            _ => {
                return Err(MemoryError::InvalidAddress(value));
            }
        };

        let size = match value {
            DataValue::U32(_) | DataValue::I32(_) => AddressSize::_32,
            DataValue::U64(_) | DataValue::I64(_) => AddressSize::_64,
            _ => unreachable!(),
        };

        let region = AddressRegion::decode(addr >> (size.bits() - 2));

        let entry_bits = Address::entry_bits(size, region);
        let offset_bits = Address::offset_bits(size, region);

        let entry = (addr >> offset_bits) & ((1 << entry_bits) - 1);
        let offset = addr & ((1 << offset_bits) - 1);

        Address::from_parts(size, region, entry, offset)
    }
}

impl TryFrom<u64> for Address {
    type Error = MemoryError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let dv = if value > u32::MAX as u64 {
            DataValue::U64(value)
        } else {
            DataValue::U32(value as u32)
        };

        Address::try_from(dv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn address_region_roundtrip_encode_decode() {
        let all_regions = [
            AddressRegion::Stack,
            AddressRegion::Heap,
            AddressRegion::Table,
            AddressRegion::GlobalValue,
        ];

        for region in all_regions {
            assert_eq!(AddressRegion::decode(region.encode()), region);
        }
    }

    #[test]
    fn address_roundtrip() {
        let test_addresses = [
            (AddressSize::_32, AddressRegion::Stack, 0, 0),
            (AddressSize::_32, AddressRegion::Stack, 0, 1),
            (AddressSize::_32, AddressRegion::Stack, 0, 1024),
            (AddressSize::_32, AddressRegion::Stack, 0, 0x3FFF_FFFF),
            (AddressSize::_32, AddressRegion::Heap, 0, 0),
            (AddressSize::_32, AddressRegion::Heap, 1, 1),
            (AddressSize::_32, AddressRegion::Heap, 3, 1024),
            (AddressSize::_32, AddressRegion::Heap, 3, 0x0FFF_FFFF),
            (AddressSize::_32, AddressRegion::Table, 0, 0),
            (AddressSize::_32, AddressRegion::Table, 1, 1),
            (AddressSize::_32, AddressRegion::Table, 31, 0x1FF_FFFF),
            (AddressSize::_32, AddressRegion::GlobalValue, 0, 0),
            (AddressSize::_32, AddressRegion::GlobalValue, 1, 1),
            (AddressSize::_32, AddressRegion::GlobalValue, 63, 0xFF_FFFF),
            (AddressSize::_64, AddressRegion::Stack, 0, 0),
            (AddressSize::_64, AddressRegion::Stack, 0, 1),
            (
                AddressSize::_64,
                AddressRegion::Stack,
                0,
                0x3FFFFFFF_FFFFFFFF,
            ),
            (AddressSize::_64, AddressRegion::Heap, 0, 0),
            (AddressSize::_64, AddressRegion::Heap, 1, 1),
            (AddressSize::_64, AddressRegion::Heap, 3, 1024),
            (AddressSize::_64, AddressRegion::Heap, 3, 0x0FFF_FFFF),
            (AddressSize::_64, AddressRegion::Table, 0, 0),
            (AddressSize::_64, AddressRegion::Table, 1, 1),
            (AddressSize::_64, AddressRegion::Table, 31, 0x1FF_FFFF),
            (AddressSize::_64, AddressRegion::GlobalValue, 0, 0),
            (AddressSize::_64, AddressRegion::GlobalValue, 1, 1),
            (AddressSize::_64, AddressRegion::GlobalValue, 63, 0xFF_FFFF),
        ];

        for (size, region, entry, offset) in test_addresses {
            let original = Address {
                size,
                region,
                entry,
                offset,
            };

            let dv: DataValue = original.clone().try_into().unwrap();
            let addr = dv.try_into().unwrap();

            assert_eq!(original, addr);
        }
    }
}
