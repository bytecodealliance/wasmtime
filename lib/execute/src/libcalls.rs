//! Runtime library calls. Note that the JIT may sometimes perform these inline
//! rather than calling them, particularly when CPUs have special instructions
//! which compute them directly.

pub extern "C" fn wasmtime_f32_ceil(x: f32) -> f32 {
    x.ceil()
}

pub extern "C" fn wasmtime_f32_floor(x: f32) -> f32 {
    x.floor()
}

pub extern "C" fn wasmtime_f32_trunc(x: f32) -> f32 {
    x.trunc()
}

pub extern "C" fn wasmtime_f32_nearest(x: f32) -> f32 {
    // Rust doesn't have a nearest function, so do it manually.
    if x == 0.0 {
        // Preserve the sign of zero.
        x
    } else {
        // Nearest is either ceil or floor depending on which is nearest or even.
        let u = x.ceil();
        let d = x.floor();
        let um = (x - u).abs();
        let dm = (x - d).abs();
        if um < dm
            || (um == dm && {
                let h = u / 2.;
                h.floor() == h
            }) {
            u
        } else {
            d
        }
    }
}

pub extern "C" fn wasmtime_f64_ceil(x: f64) -> f64 {
    x.ceil()
}

pub extern "C" fn wasmtime_f64_floor(x: f64) -> f64 {
    x.floor()
}

pub extern "C" fn wasmtime_f64_trunc(x: f64) -> f64 {
    x.trunc()
}

pub extern "C" fn wasmtime_f64_nearest(x: f64) -> f64 {
    // Rust doesn't have a nearest function, so do it manually.
    if x == 0.0 {
        // Preserve the sign of zero.
        x
    } else {
        // Nearest is either ceil or floor depending on which is nearest or even.
        let u = x.ceil();
        let d = x.floor();
        let um = (x - u).abs();
        let dm = (x - d).abs();
        if um < dm
            || (um == dm && {
                let h = u / 2.;
                h.floor() == h
            }) {
            u
        } else {
            d
        }
    }
}
