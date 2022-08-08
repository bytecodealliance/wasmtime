/// Represents the possible sizes in bytes of the discriminant of a variant type in the component model
#[derive(Debug, Copy, Clone)]
pub enum DiscriminantSize {
    /// 8-bit discriminant
    Size1,
    /// 16-bit discriminant
    Size2,
    /// 32-bit discriminant
    Size4,
}

impl DiscriminantSize {
    /// Calculate the size of discriminant needed to represent a variant with the specified number of cases.
    pub fn from_count(count: usize) -> Option<Self> {
        if count <= 0xFF {
            Some(Self::Size1)
        } else if count <= 0xFFFF {
            Some(Self::Size2)
        } else if count <= 0xFFFF_FFFF {
            Some(Self::Size4)
        } else {
            None
        }
    }
}

impl From<DiscriminantSize> for u32 {
    /// Size of the discriminant as a `u32`
    fn from(size: DiscriminantSize) -> u32 {
        match size {
            DiscriminantSize::Size1 => 1,
            DiscriminantSize::Size2 => 2,
            DiscriminantSize::Size4 => 4,
        }
    }
}

impl From<DiscriminantSize> for usize {
    /// Size of the discriminant as a `usize`
    fn from(size: DiscriminantSize) -> usize {
        match size {
            DiscriminantSize::Size1 => 1,
            DiscriminantSize::Size2 => 2,
            DiscriminantSize::Size4 => 4,
        }
    }
}

/// Represents the number of bytes required to store a flags value in the component model
pub enum FlagsSize {
    /// There are no flags
    Size0,
    /// Flags can fit in a u8
    Size1,
    /// Flags can fit in a u16
    Size2,
    /// Flags can fit in a specified number of u32 fields
    Size4Plus(usize),
}

impl FlagsSize {
    /// Calculate the size needed to represent a value with the specified number of flags.
    pub fn from_count(count: usize) -> FlagsSize {
        if count == 0 {
            FlagsSize::Size0
        } else if count <= 8 {
            FlagsSize::Size1
        } else if count <= 16 {
            FlagsSize::Size2
        } else {
            FlagsSize::Size4Plus(ceiling_divide(count, 32))
        }
    }
}

/// Divide `n` by `d`, rounding up in the case of a non-zero remainder.
fn ceiling_divide(n: usize, d: usize) -> usize {
    (n + d - 1) / d
}

/// A simple bump allocator which can be used with modules
pub const REALLOC_AND_FREE: &str = r#"
    (global $last (mut i32) (i32.const 8))
    (func $realloc (export "realloc")
        (param $old_ptr i32)
        (param $old_size i32)
        (param $align i32)
        (param $new_size i32)
        (result i32)

        ;; Test if the old pointer is non-null
        local.get $old_ptr
        if
            ;; If the old size is bigger than the new size then
            ;; this is a shrink and transparently allow it
            local.get $old_size
            local.get $new_size
            i32.gt_u
            if
                local.get $old_ptr
                return
            end

            ;; ... otherwise this is unimplemented
            unreachable
        end

        ;; align up `$last`
        (global.set $last
            (i32.and
                (i32.add
                    (global.get $last)
                    (i32.add
                        (local.get $align)
                        (i32.const -1)))
                (i32.xor
                    (i32.add
                        (local.get $align)
                        (i32.const -1))
                    (i32.const -1))))

        ;; save the current value of `$last` as the return value
        global.get $last

        ;; ensure anything necessary is set to valid data by spraying a bit
        ;; pattern that is invalid
        global.get $last
        i32.const 0xde
        local.get $new_size
        memory.fill

        ;; bump our pointer
        (global.set $last
            (i32.add
                (global.get $last)
                (local.get $new_size)))
    )
"#;
