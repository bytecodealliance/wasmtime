#[inline(never)]
#[no_mangle]
pub extern "C" fn gcd(m_: u32, n_: u32) -> u32 {
    let mut m = m_;
    let mut n = n_;
    while m > 0 {
        let tmp = m;
        m = n % m;
        n = tmp;
    }
    return n;
}

#[no_mangle]
pub extern "C" fn test() -> u32 {
    gcd(24, 9)
}
