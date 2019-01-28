use cranelift_entity::PrimaryMap;

use super::regs::{
    RegBank, RegBankBuilder, RegBankIndex, RegClass, RegClassBuilder, RegClassIndex, RegClassProto,
};
use super::settings::SettingGroup;

pub struct TargetIsa {
    pub name: &'static str,
    pub reg_banks: PrimaryMap<RegBankIndex, RegBank>,
    pub reg_classes: PrimaryMap<RegClassIndex, RegClass>,
    pub settings: SettingGroup,
}

impl TargetIsa {
    pub fn new(name: &'static str, settings: SettingGroup) -> Self {
        Self {
            name,
            reg_banks: PrimaryMap::new(),
            reg_classes: PrimaryMap::new(),
            settings,
        }
    }
}

pub struct TargetIsaBuilder {
    isa: TargetIsa,
}

impl TargetIsaBuilder {
    pub fn new(name: &'static str, settings: SettingGroup) -> Self {
        Self {
            isa: TargetIsa::new(name, settings),
        }
    }

    pub fn add_reg_bank(&mut self, builder: RegBankBuilder) -> RegBankIndex {
        let first_unit = if self.isa.reg_banks.len() == 0 {
            0
        } else {
            let last = &self.isa.reg_banks.last().unwrap();
            let first_available_unit = (last.first_unit + last.units) as i8;
            let units = builder.units;
            let align = if units.is_power_of_two() {
                units
            } else {
                units.next_power_of_two()
            } as i8;
            (first_available_unit + align - 1) & -align
        } as u8;

        self.isa.reg_banks.push(RegBank::new(
            builder.name,
            first_unit,
            builder.units,
            builder.names,
            builder.prefix,
            builder
                .pressure_tracking
                .expect("Pressure tracking must be explicitly set"),
        ))
    }

    pub fn add_reg_class(&mut self, builder: RegClassBuilder) -> RegClassIndex {
        let class_index = self.isa.reg_classes.next_key();

        // Finish delayed construction of RegClass.
        let (bank, toprc, start, width) = match builder.proto {
            RegClassProto::TopLevel(bank_index) => {
                self.isa
                    .reg_banks
                    .get_mut(bank_index)
                    .unwrap()
                    .toprcs
                    .push(class_index);
                (bank_index, class_index, builder.start, builder.width)
            }
            RegClassProto::SubClass(parent_class_index) => {
                assert!(builder.width == 0);
                let (bank, toprc, start, width) = {
                    let parent = self.isa.reg_classes.get(parent_class_index).unwrap();
                    (parent.bank, parent.toprc, parent.start, parent.width)
                };
                for reg_class in self.isa.reg_classes.values_mut() {
                    if reg_class.toprc == toprc {
                        reg_class.subclasses.push(class_index);
                    }
                }
                let subclass_start = start + builder.start * width;
                (bank, toprc, subclass_start, width)
            }
        };

        let reg_bank_units = self.isa.reg_banks.get(bank).unwrap().units;
        assert!(start < reg_bank_units);

        let count = if builder.count != 0 {
            builder.count
        } else {
            reg_bank_units / width
        };

        let reg_class = RegClass::new(builder.name, class_index, width, bank, toprc, count, start);
        self.isa.reg_classes.push(reg_class);

        let reg_bank = self.isa.reg_banks.get_mut(bank).unwrap();
        reg_bank.classes.push(class_index);

        class_index
    }

    /// Checks that the set of register classes satisfies:
    ///
    /// 1. Closed under intersection: The intersection of any two register
    ///    classes in the set is either empty or identical to a member of the
    ///    set.
    /// 2. There are no identical classes under different names.
    /// 3. Classes are sorted topologically such that all subclasses have a
    ///    higher index that the superclass.
    pub fn finish(self) -> TargetIsa {
        for reg_bank in self.isa.reg_banks.values() {
            for i1 in reg_bank.classes.iter() {
                for i2 in reg_bank.classes.iter() {
                    if i1 >= i2 {
                        continue;
                    }

                    let rc1 = self.isa.reg_classes.get(*i1).unwrap();
                    let rc2 = self.isa.reg_classes.get(*i2).unwrap();

                    let rc1_mask = rc1.mask(0);
                    let rc2_mask = rc2.mask(0);

                    assert!(
                        rc1.width != rc2.width || rc1_mask != rc2_mask,
                        "no duplicates"
                    );
                    if rc1.width != rc2.width {
                        continue;
                    }

                    let mut intersect = Vec::new();
                    for (a, b) in rc1_mask.iter().zip(rc2_mask.iter()) {
                        intersect.push(a & b);
                    }
                    if intersect == vec![0; intersect.len()] {
                        continue;
                    }

                    // Classes must be topologically ordered, so the intersection can't be the
                    // superclass.
                    assert!(intersect != rc1_mask);

                    // If the intersection is the second one, then it must be a subclass.
                    if intersect == rc2_mask {
                        assert!(self
                            .isa
                            .reg_classes
                            .get(*i1)
                            .unwrap()
                            .subclasses
                            .iter()
                            .find(|x| **x == *i2)
                            .is_some());
                    }
                }
            }
        }

        // This limit should be coordinated with the `RegClassMask` and `RegClassIndex` types in
        // isa/registers.rs of the non-meta code.
        assert!(
            self.isa.reg_classes.len() <= 32,
            "Too many register classes"
        );

        // The maximum number of top-level register classes which have pressure tracking should be
        // kept in sync with the MAX_TRACKED_TOPRCS constant in isa/registers.rs of the non-meta
        // code.
        let num_toplevel = self
            .isa
            .reg_classes
            .values()
            .filter(|x| {
                x.toprc == x.index && self.isa.reg_banks.get(x.bank).unwrap().pressure_tracking
            })
            .count();
        assert!(num_toplevel <= 4, "Too many top-level register classes");

        self.isa
    }
}
