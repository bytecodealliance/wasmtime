pub trait GuestType {
    fn len() -> u32;
}

impl GuestType for u8 {
    fn len() -> u32 {
        1
    }
}

impl GuestType for i8 {
    fn len() -> u32 {
        1
    }
}

impl GuestType for u16 {
    fn len() -> u32 {
        2
    }
}

impl GuestType for i16 {
    fn len() -> u32 {
        2
    }
}

impl GuestType for u32 {
    fn len() -> u32 {
        4
    }
}

impl GuestType for i32 {
    fn len() -> u32 {
        4
    }
}

impl GuestType for f32 {
    fn len() -> u32 {
        4
    }
}

impl GuestType for u64 {
    fn len() -> u32 {
        8
    }
}

impl GuestType for i64 {
    fn len() -> u32 {
        8
    }
}

impl GuestType for f64 {
    fn len() -> u32 {
        8
    }
}

impl GuestType for char {
    fn len() -> u32 {
        1
    }
}

impl GuestType for usize {
    fn len() -> u32 {
        4
    }
}
