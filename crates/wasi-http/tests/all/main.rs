// Assert that each of `sync` and `async` below are testing everything through
// assertion of the existence of the test function itself.
macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "only here to ensure a name exists")]
        use self::$name as _;
    };
}

mod http_server;
mod p2;
#[cfg(feature = "p3")]
mod p3;

mod body {
    use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
    use hyper::Error;
    use hyper::body::Bytes;

    pub fn full(bytes: Bytes) -> BoxBody<Bytes, Error> {
        BoxBody::new(Full::new(bytes).map_err(|x| match x {}))
    }

    pub fn empty() -> BoxBody<Bytes, Error> {
        BoxBody::new(Empty::new().map_err(|x| match x {}))
    }
}
