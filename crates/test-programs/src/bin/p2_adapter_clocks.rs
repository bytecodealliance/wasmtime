fn main() {
    #[link(wasm_import_module = "wasi_snapshot_preview1")]
    unsafe extern "C" {
        fn adapter_monotonic_clock_set_paused(paused: bool);
    }

    unsafe {
        assert!(wasip1::clock_time_get(wasip1::CLOCKID_REALTIME, 0).unwrap() > 0);
        let monotonic = wasip1::clock_time_get(wasip1::CLOCKID_MONOTONIC, 0).unwrap();
        let realtime = wasip1::clock_time_get(wasip1::CLOCKID_REALTIME, 0).unwrap();
        adapter_monotonic_clock_set_paused(true);
        assert_eq!(
            wasip1::clock_time_get(wasip1::CLOCKID_REALTIME, 0).unwrap(),
            realtime,
        );
        assert_eq!(
            wasip1::clock_time_get(wasip1::CLOCKID_MONOTONIC, 0).unwrap(),
            monotonic,
        );
        // A relative zero timeout is already ready and needs no host call.
        for id in [wasip1::CLOCKID_MONOTONIC, wasip1::CLOCKID_REALTIME] {
            let mut subscription = wasip1::Subscription {
                userdata: 42,
                u: wasip1::SubscriptionU {
                    tag: wasip1::EVENTTYPE_CLOCK.raw(),
                    u: wasip1::SubscriptionUU {
                        clock: wasip1::SubscriptionClock {
                            id,
                            timeout: 0,
                            precision: 0,
                            flags: 0,
                        },
                    },
                },
            };
            let mut event = std::mem::MaybeUninit::<wasip1::Event>::uninit();
            assert_eq!(
                wasip1::poll_oneoff(&subscription, event.as_mut_ptr(), 1),
                Ok(1)
            );
            let event = event.assume_init();
            assert_eq!(event.userdata, subscription.userdata);
            assert_eq!(event.error, wasip1::ERRNO_SUCCESS);
            assert_eq!(event.type_, wasip1::EVENTTYPE_CLOCK);

            // A future timer cannot be answered while host calls are paused.
            subscription.u.u.clock.timeout = 1;
            let mut event = std::mem::MaybeUninit::<wasip1::Event>::uninit();
            assert_eq!(
                wasip1::poll_oneoff(&subscription, event.as_mut_ptr(), 1),
                Err(wasip1::ERRNO_NOTSUP),
            );
        }
        adapter_monotonic_clock_set_paused(false);
        assert!(wasip1::clock_time_get(wasip1::CLOCKID_REALTIME, 0).unwrap() > realtime);
        assert!(wasip1::clock_time_get(wasip1::CLOCKID_MONOTONIC, 0).unwrap() > monotonic);
    }
}
