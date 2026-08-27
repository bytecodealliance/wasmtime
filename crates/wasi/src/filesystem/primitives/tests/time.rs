// This file is derived from Rust's library/std/src/time/tests.rs at revision
// 108e90ca78f052c0c1c49c42a22c85620be19712.

use cap_std::ambient_authority;
use cap_std::time::{Duration, MonotonicClock, SystemClock};

macro_rules! assert_almost_eq {
    ($a:expr, $b:expr) => {{
        let (a, b) = ($a, $b);
        if a != b {
            let (a, b) = if a > b { (a, b) } else { (b, a) };
            assert!(
                a - Duration::new(0, 1000) <= b,
                "{:?} is not almost equal to {:?}",
                a,
                b
            );
        }
    }};
}

#[test]
fn instant_monotonic() {
    let clock = MonotonicClock::new(ambient_authority());
    let a = clock.now();
    let b = clock.now();
    assert!(b >= a);
}

#[test]
fn instant_elapsed() {
    let clock = MonotonicClock::new(ambient_authority());
    let a = clock.now();
    clock.elapsed(a);
}

#[test]
fn instant_math() {
    let clock = MonotonicClock::new(ambient_authority());
    let a = clock.now();
    let b = clock.now();
    println!("a: {:?}", a);
    println!("b: {:?}", b);
    let dur = b.duration_since(a);
    println!("dur: {:?}", dur);
    assert_almost_eq!(b - dur, a);
    assert_almost_eq!(a + dur, b);

    let second = Duration::new(1, 0);
    assert_almost_eq!(a - second + second, a);
    assert_almost_eq!(
        a.checked_sub(second).unwrap().checked_add(second).unwrap(),
        a
    );

    // checked_add_duration will not panic on overflow
    let mut maybe_t = Some(clock.now());
    let max_duration = Duration::from_secs(u64::MAX);
    // in case `Instant` can store `>= now + max_duration`.
    for _ in 0..2 {
        maybe_t = maybe_t.and_then(|t| t.checked_add(max_duration));
    }
    assert_eq!(maybe_t, None);

    // checked_add_duration calculates the right time and will work for another
    // year
    let year = Duration::from_secs(60 * 60 * 24 * 365);
    assert_eq!(a + year, a.checked_add(year).unwrap());
}

#[test]
fn instant_math_is_associative() {
    let clock = MonotonicClock::new(ambient_authority());
    let now = clock.now();
    let offset = Duration::from_millis(5);
    // Changing the order of instant math shouldn't change the results,
    // especially when the expression reduces to X + identity.
    assert_eq!((now + offset) - now, (now - now) + offset);
}

// Ignore this test for now. As of rust-lang/rust#89926, Rust no longer panics
// on this test.
//
// TODO: Once our MSRV no longer contains the old behavior, we should update
// to the latest version of this test.
#[test]
#[should_panic]
#[ignore]
fn instant_duration_since_panic() {
    let clock = MonotonicClock::new(ambient_authority());
    let a = clock.now();
    (a - Duration::new(1, 0)).duration_since(a);
}

#[test]
fn instant_checked_duration_since_nopanic() {
    let clock = MonotonicClock::new(ambient_authority());
    let now = clock.now();
    let earlier = now - Duration::new(1, 0);
    let later = now + Duration::new(1, 0);
    assert_eq!(earlier.checked_duration_since(now), None);
    assert_eq!(later.checked_duration_since(now), Some(Duration::new(1, 0)));
    assert_eq!(now.checked_duration_since(now), Some(Duration::new(0, 0)));
}

#[test]
fn instant_saturating_duration_since_nopanic() {
    let clock = MonotonicClock::new(ambient_authority());
    let a = clock.now();
    let ret = (a - Duration::new(1, 0)).saturating_duration_since(a);
    assert_eq!(ret, Duration::new(0, 0));
}

#[test]
fn system_time_math() {
    let clock = SystemClock::new(ambient_authority());
    let a = clock.now();
    let b = clock.now();
    match b.duration_since(a) {
        Ok(dur) if dur == Duration::new(0, 0) => {
            assert_almost_eq!(a, b);
        }
        Ok(dur) => {
            assert!(b > a);
            assert_almost_eq!(b - dur, a);
            assert_almost_eq!(a + dur, b);
        }
        Err(dur) => {
            let dur = dur.duration();
            assert!(a > b);
            assert_almost_eq!(b + dur, a);
            assert_almost_eq!(a - dur, b);
        }
    }

    let second = Duration::new(1, 0);
    assert_almost_eq!(a.duration_since(a - second).unwrap(), second);
    assert_almost_eq!(a.duration_since(a + second).unwrap_err().duration(), second);

    assert_almost_eq!(a - second + second, a);
    assert_almost_eq!(
        a.checked_sub(second).unwrap().checked_add(second).unwrap(),
        a
    );

    let one_second_from_epoch = SystemClock::UNIX_EPOCH + Duration::new(1, 0);
    let one_second_from_epoch2 =
        SystemClock::UNIX_EPOCH + Duration::new(0, 500_000_000) + Duration::new(0, 500_000_000);
    assert_eq!(one_second_from_epoch, one_second_from_epoch2);

    // checked_add_duration will not panic on overflow
    let mut maybe_t = Some(SystemClock::UNIX_EPOCH);
    let max_duration = Duration::from_secs(u64::MAX);
    // in case `SystemTime` can store `>= UNIX_EPOCH + max_duration`.
    for _ in 0..2 {
        maybe_t = maybe_t.and_then(|t| t.checked_add(max_duration));
    }
    assert_eq!(maybe_t, None);

    // checked_add_duration calculates the right time and will work for another
    // year
    let year = Duration::from_secs(60 * 60 * 24 * 365);
    assert_eq!(a + year, a.checked_add(year).unwrap());
}

#[test]
fn system_time_elapsed() {
    let clock = SystemClock::new(ambient_authority());
    let a = clock.now();
    drop(clock.elapsed(a));
}

#[test]
fn since_epoch() {
    let clock = SystemClock::new(ambient_authority());
    let ts = clock.now();
    let a = ts
        .duration_since(SystemClock::UNIX_EPOCH + Duration::new(1, 0))
        .unwrap();
    let b = ts.duration_since(SystemClock::UNIX_EPOCH).unwrap();
    assert!(b > a);
    assert_eq!(b - a, Duration::new(1, 0));

    let thirty_years = Duration::new(1, 0) * 60 * 60 * 24 * 365 * 30;

    // Right now for CI this test is run in an emulator, and apparently the
    // aarch64 emulator's sense of time is that we're still living in the
    // 70s. This is also true for riscv (also qemu)
    //
    // Otherwise let's assume that we're all running computers later than
    // 2000.
    if !cfg!(target_arch = "aarch64") && !cfg!(target_arch = "riscv64") {
        assert!(a > thirty_years);
    }

    // let's assume that we're all running computers earlier than 2090.
    // Should give us ~70 years to fix this!
    let hundred_twenty_years = thirty_years * 4;
    assert!(a < hundred_twenty_years);
}
