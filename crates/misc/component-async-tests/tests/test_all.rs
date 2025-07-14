//! All scenario tests have been put into this binary to ensure that we run
//! tests with concurrently, as separate test binaries run serially.

macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "just here to ensure a name exists")]
        use self::$name as _;
    };
}
test_programs_artifacts::foreach_async!(assert_test_exists);

mod scenario;

use scenario::backpressure::{async_backpressure_callee, async_backpressure_caller};
use scenario::borrowing::{async_borrowing_callee, async_borrowing_caller};
use scenario::error_context::{
    async_error_context, async_error_context_callee, async_error_context_caller,
};
use scenario::post_return::{async_post_return_callee, async_post_return_caller};
use scenario::read_resource_stream::async_read_resource_stream;
use scenario::round_trip::{
    async_round_trip_stackful, async_round_trip_stackless, async_round_trip_stackless_sync_import,
    async_round_trip_synchronous, async_round_trip_wait,
};
use scenario::round_trip_direct::async_round_trip_direct_stackless;
use scenario::round_trip_many::{
    async_round_trip_many_stackful, async_round_trip_many_stackless,
    async_round_trip_many_synchronous, async_round_trip_many_wait,
};
use scenario::streams::async_closed_streams;
use scenario::transmit::{
    async_cancel_callee, async_cancel_caller, async_intertask_communication, async_poll_stackless,
    async_poll_synchronous, async_transmit_callee, async_transmit_caller,
};
use scenario::unit_stream::{async_unit_stream_callee, async_unit_stream_caller};
use scenario::yield_::{
    async_yield_callee_stackless, async_yield_callee_synchronous, async_yield_caller,
};
