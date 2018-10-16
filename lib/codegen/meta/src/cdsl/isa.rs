use cranelift_entity::PrimaryMap;

use super::regs::{
    RegBank, RegBankBuilder, RegBankIndex, RegClass, RegClassBuilder, RegClassIndex,
};

pub struct TargetIsa {
    pub name: &'static str,
    pub reg_banks: PrimaryMap<RegBankIndex, RegBank>,
    pub reg_classes: PrimaryMap<RegClassIndex, RegClass>,
}

impl TargetIsa {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            reg_banks: PrimaryMap::new(),
            reg_classes: PrimaryMap::new(),
        }
    }

    pub fn add_reg_bank(&mut self, builder: RegBankBuilder) -> RegBankIndex {
        let first_unit = if self.reg_banks.len() == 0 {
            0
        } else {
            let last = &self.reg_banks.last().unwrap();
            let first_available_unit = (last.first_unit + last.units) as i8;
            let units = builder.units;
            let align = if units.is_power_of_two() {
                units
            } else {
                units.next_power_of_two()
            } as i8;
            (first_available_unit + align - 1) & -align
        } as u8;

        self.reg_banks.push(RegBank::new(
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
        let reg_bank_units = self.reg_banks.get(builder.bank).unwrap().units;

        let start = builder.start;
        assert!(start < reg_bank_units);

        let count = if builder.count != 0 {
            builder.count
        } else {
            reg_bank_units / builder.width
        };

        let reg_class_index = builder.index;
        assert!(
            self.reg_classes.next_key() == reg_class_index,
            "should have inserted RegClass where expected"
        );

        let reg_class = RegClass::new(
            builder.name,
            reg_class_index,
            builder.width,
            builder.bank,
            builder.toprc,
            count,
            start,
        );
        self.reg_classes.push(reg_class);

        let reg_bank = self.reg_banks.get_mut(builder.bank).unwrap();
        reg_bank.classes.push(reg_class_index);

        reg_class_index
    }

    /// Checks that the set of register classes satisfies:
    ///
    /// 1. Closed under intersection: The intersection of any two register
    ///    classes in the set is either empty or identical to a member of the
    ///    set.
    /// 2. There are no identical classes under different names.
    /// 3. Classes are sorted topologically such that all subclasses have a
    ///    higher index that the superclass.
    pub fn check(&self) {
        for reg_bank in self.reg_banks.values() {
            for i1 in reg_bank.classes.iter() {
                for i2 in reg_bank.classes.iter() {
                    if i1 >= i2 {
                        continue;
                    }

                    let rc1 = self.reg_classes.get(*i1).unwrap();
                    let rc2 = self.reg_classes.get(*i2).unwrap();

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
                        assert!(
                            self.reg_classes
                                .get(*i1)
                                .unwrap()
                                .subclasses
                                .iter()
                                .find(|x| **x == *i2)
                                .is_some()
                        );
                    }
                }
            }
        }
    }
}
