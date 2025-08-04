macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "just here to ensure a name exists")]
        use self::$name as _;
    };
}

mod store;

#[cfg(feature = "p1")]
mod p1;
#[cfg(feature = "p2")]
mod p2;
#[cfg(feature = "p3")]
mod p3;
