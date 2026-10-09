use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

struct Exited(Arc<AtomicBool>);

impl Drop for Exited {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn failed_waiter_wakeup_restores_store_ownership() {
    let mut config = crate::Config::new();
    config.wasm_component_model_async(true);
    let engine = crate::Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, ());
    let store = store.as_context_mut().0;

    let fiber = unsafe { fiber::make_fiber_unchecked(store, |_| Ok(())) }.unwrap();
    let state = store.concurrent_state_mut().unwrap();
    let set = state.push(WaitableSet::default()).unwrap();
    let group = state.make_task_group().unwrap();
    let host = state
        .push(HostTask {
            common: WaitableCommon::default(),
            call_context: CallContext::default(),
            state: HostTaskState::CalleeStarted,
            group,
        })
        .unwrap();
    let thread = QualifiedThreadId {
        task: TableId::new(u32::MAX),
        thread: TableId::new(u32::MAX),
    };
    state.get_mut(host).unwrap().common.set = Some(set);
    state
        .get_mut(set)
        .unwrap()
        .waiting
        .insert(thread, WaitMode::Fiber(fiber));

    let error = Waitable::Host(host).mark_ready(state).unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ResourceTableError>(),
        Some(ResourceTableError::NotPresent)
    ));
    assert!(state.get_mut(set).unwrap().waiting.contains_key(&thread));
    // Dropping the store disposes the restored live fiber.
}

#[test]
#[cfg_attr(miri, ignore)]
fn invalid_waiting_set_returns_error_without_dropping_live_fiber() {
    let mut config = crate::Config::new();
    config.wasm_component_model_async(true);
    let engine = crate::Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, ());
    let store = store.as_context_mut().0;
    let exited = Arc::new(AtomicBool::new(false));
    let fiber_exited = exited.clone();

    // A suspended fiber names a waitable set that does not exist. The
    // destination lookup must return its original table error while the
    // live fiber is disposed using the store.
    let fiber = unsafe {
        fiber::make_fiber_unchecked(store, move |store| {
            let _exited = Exited(fiber_exited);
            store.concurrent_state_mut()?.suspend_reason = Some(SuspendReason::Waiting {
                set: TableId::new(u32::MAX),
                thread: QualifiedThreadId {
                    task: TableId::new(u32::MAX),
                    thread: TableId::new(u32::MAX),
                },
            });
            store.with_blocking(|_, cx| cx.suspend(StoreFiberYield::ReleaseStore))?;
            Ok(())
        })
    }
    .unwrap();

    let result = {
        let mut future = Box::pin(store.resume_fiber(fiber));
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
    };
    let Poll::Ready(Err(error)) = result else {
        panic!("expected resource table error from invalid waitable set");
    };
    assert!(matches!(
        error.downcast_ref::<ResourceTableError>(),
        Some(ResourceTableError::NotPresent)
    ));
    assert!(exited.load(Ordering::SeqCst));
}

// Exercise each suspend destination with an invalid thread handle. The
// returned table error and the exit marker show that a live fiber did not
// escape both store ownership and disposal.
fn invalid_thread_suspend_returns_original_error(reason: fn(QualifiedThreadId) -> SuspendReason) {
    let mut config = crate::Config::new();
    config.wasm_component_model_async(true);
    let engine = crate::Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, ());
    let store = store.as_context_mut().0;
    let exited = Arc::new(AtomicBool::new(false));
    let fiber_exited = exited.clone();

    let fiber = unsafe {
        fiber::make_fiber_unchecked(store, move |store| {
            let _exited = Exited(fiber_exited);
            store.concurrent_state_mut()?.suspend_reason = Some(reason(QualifiedThreadId {
                task: TableId::new(u32::MAX),
                thread: TableId::new(u32::MAX),
            }));
            store.with_blocking(|_, cx| cx.suspend(StoreFiberYield::ReleaseStore))?;
            Ok(())
        })
    }
    .unwrap();

    let result = {
        let mut future = Box::pin(store.resume_fiber(fiber));
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
    };
    let Poll::Ready(Err(error)) = result else {
        panic!("expected resource table error from invalid thread");
    };
    assert!(matches!(
        error.downcast_ref::<ResourceTableError>(),
        Some(ResourceTableError::NotPresent)
    ));
    assert!(exited.load(Ordering::SeqCst));
}

#[test]
#[cfg_attr(miri, ignore)]
fn occupied_switch_keeps_incoming_fiber_in_store() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let mut config = crate::Config::new();
    config.wasm_component_model_async(true);
    let engine = crate::Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, ());
    let store = store.as_context_mut().0;
    let fiber = unsafe { fiber::make_fiber_unchecked(store, |_| Ok(())) }.unwrap();
    let instance = RuntimeInstance {
        instance: crate::component::ComponentInstanceId::from_u32(0),
        index: RuntimeComponentInstanceIndex::from_u32(0),
    };
    let thread = QualifiedThreadId {
        task: TableId::new(u32::MAX),
        thread: TableId::new(u32::MAX),
    };
    let state = store.concurrent_state_mut().unwrap();
    state.switch_item = Some(WorkItem::ResumeThread { instance, thread });

    let result = catch_unwind(AssertUnwindSafe(|| {
        state.push_work_item(
            WorkItem::ResumeFiber {
                instance,
                thread,
                fiber,
            },
            Priority::Switch,
        )
    }));
    if cfg!(debug_assertions) {
        let panic = result.expect_err("debug builds must keep the original bail_bug panic");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(message.contains("BUG: switch item already set"));
    } else {
        let error = result.unwrap().unwrap_err();
        assert!(error.is::<crate::WasmtimeBug>());
        assert!(error.to_string().contains("switch item already set"));
    }
    assert_eq!(state.high_priority.len(), 1);
    // The store disposes the fiber retained in its queue.
}

#[test]
#[cfg_attr(miri, ignore)]
fn yielding_invalid_thread_returns_table_error() {
    invalid_thread_suspend_returns_original_error(|thread| SuspendReason::Yielding { thread });
}

#[test]
#[cfg_attr(miri, ignore)]
fn explicitly_suspending_invalid_thread_returns_table_error() {
    invalid_thread_suspend_returns_original_error(|thread| SuspendReason::ExplicitlySuspending {
        thread,
    });
}

#[test]
#[cfg_attr(miri, ignore)]
fn yielding_to_subtask_invalid_task_returns_table_error() {
    invalid_thread_suspend_returns_original_error(|thread| SuspendReason::YieldingToSubtask {
        thread,
    });
}

fn store() -> Store<()> {
    let mut config = crate::Config::new();
    config.wasm_component_model_async(true);
    Store::new(&crate::Engine::new(&config).unwrap(), ())
}

fn instance() -> RuntimeInstance {
    RuntimeInstance {
        instance: crate::component::ComponentInstanceId::from_u32(0),
        index: RuntimeComponentInstanceIndex::from_u32(0),
    }
}

fn thread() -> QualifiedThreadId {
    QualifiedThreadId {
        task: TableId::new(u32::MAX),
        thread: TableId::new(u32::MAX),
    }
}

fn live(store: &mut StoreOpaque) -> StoreFiber<'static> {
    unsafe { fiber::make_fiber_unchecked(store, |_| Ok(())) }.unwrap()
}

fn bug(f: impl FnOnce() -> Result<()>, message: &str) {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let result = catch_unwind(AssertUnwindSafe(f));
    if cfg!(debug_assertions) {
        let panic = result.expect_err("debug bail_bug must panic");
        let text = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(text.contains(message), "{text}");
    } else {
        let error = result.unwrap().unwrap_err();
        assert!(error.is::<crate::WasmtimeBug>());
        assert!(error.to_string().contains(message), "{error}");
    }
}

fn add_thread(state: &mut ConcurrentState, fiber: StoreFiber<'static>) -> QualifiedThreadId {
    let sync_call_set = state.push(WaitableSet::default()).unwrap();
    let q = thread();
    let t = state
        .push(GuestThread {
            context: [0; NUM_COMPONENT_CONTEXT_SLOTS],
            parent_task: q.task,
            wake_on_cancel: WakeOnCancel::None,
            state: GuestThreadState::Suspended(fiber),
            instance_rep: None,
            sync_call_set,
            old_do_not_suspend: None,
        })
        .unwrap();
    QualifiedThreadId { thread: t, ..q }
}

#[test]
#[cfg_attr(miri, ignore)]
fn unpolled_resume_disposes_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = live(store);
    drop(store.resume_fiber(fiber));
}

#[test]
#[cfg_attr(miri, ignore)]
fn missing_suspend_reason_disposes_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = unsafe {
        fiber::make_fiber_unchecked(store, |store| {
            store.with_blocking(|_, cx| cx.suspend(StoreFiberYield::ReleaseStore))?;
            Ok(())
        })
    }
    .unwrap();
    bug(
        || {
            let mut future = Box::pin(store.resume_fiber(fiber));
            match future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                Poll::Ready(r) => r,
                _ => panic!("expected completion"),
            }
        },
        "suspend reason missing when resuming fiber",
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn full_table_during_switch_save_keeps_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = live(store);
    let state = store.concurrent_state_mut().unwrap();
    state.next_switch_item = Some(WorkItem::ResumeFiber {
        instance: instance(),
        thread: thread(),
        fiber,
    });
    state.table.get_mut().set_max_capacity(0);
    let error = store
        .suspend(SuspendReason::YieldingToSubtask { thread: thread() })
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ResourceTableError>(),
        Some(ResourceTableError::Full)
    ));
    assert!(
        store
            .concurrent_state_mut()
            .unwrap()
            .next_switch_item
            .is_some()
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn occupied_restore_keeps_both_fibers() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let a = live(store);
    let b = live(store);
    let state = store.concurrent_state_mut().unwrap();
    state.next_switch_item = Some(WorkItem::ResumeFiber {
        instance: instance(),
        thread: thread(),
        fiber: a,
    });
    let saved = state.save_next_switch_item().unwrap();
    state.next_switch_item = Some(WorkItem::ResumeFiber {
        instance: instance(),
        thread: thread(),
        fiber: b,
    });
    bug(
        || state.restore_next_switch_item(saved),
        "next switch item already set when restoring",
    );
    assert!(state.get_mut(saved).unwrap().is_some());
    assert!(state.next_switch_item.is_some());
}

#[test]
#[cfg_attr(miri, ignore)]
fn callback_cancel_rejects_fiber_waiter_without_removing_it() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = live(store);
    let state = store.concurrent_state_mut().unwrap();
    let set = state.push(WaitableSet::default()).unwrap();
    state
        .get_mut(set)
        .unwrap()
        .waiting
        .insert(thread(), WaitMode::Fiber(fiber));
    bug(
        || state.take_callback_waiter(set, thread()).map(|_| ()),
        "expected",
    );
    assert!(state.get_mut(set).unwrap().waiting.contains_key(&thread()));
}

#[test]
#[cfg_attr(miri, ignore)]
fn ready_dispatch_rejects_suspended_thread_without_taking_fiber() {
    let mut s = store();
    let fiber = live(s.as_context_mut().0);
    let q = add_thread(s.as_context_mut().0.concurrent_state_mut().unwrap(), fiber);
    bug(
        || {
            let mut future =
                Box::pin(s.as_context_mut().handle_work_item(WorkItem::ResumeThread {
                    instance: instance(),
                    thread: q,
                }));
            match future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                Poll::Ready(r) => r,
                _ => panic!("expected completion"),
            }
        },
        "cannot resume non-pending thread",
    );
    assert!(matches!(
        s.as_context_mut()
            .0
            .concurrent_state_mut()
            .unwrap()
            .get_mut(q.thread)
            .unwrap()
            .state,
        GuestThreadState::Suspended(_)
    ));
}

#[test]
#[cfg_attr(miri, ignore)]
fn running_update_keeps_suspended_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = live(store);
    let state = store.concurrent_state_mut().unwrap();
    let q = add_thread(state, fiber);
    bug(
        || state.set_thread_running(q.thread),
        "thread already owns a fiber",
    );
    assert!(matches!(
        state.get_mut(q.thread).unwrap().state,
        GuestThreadState::Suspended(_)
    ));
}

#[test]
#[cfg_attr(miri, ignore)]
fn worker_item_conflict_keeps_worker_in_store() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let mut s = store();
    let fiber = live(s.as_context_mut().0);
    let state = s.as_context_mut().0.concurrent_state_mut().unwrap();
    state.worker = Some(fiber);
    state.worker_item = Some(WorkerItem::Function(AlwaysMut::new(Box::new(|_| Ok(())))));
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut future = Box::pin(
            s.as_context_mut()
                .run_on_worker(WorkerItem::Function(AlwaysMut::new(Box::new(|_| Ok(()))))),
        );
        let _ = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
    }));
    assert!(result.is_err());
    assert!(
        s.as_context_mut()
            .0
            .concurrent_state_mut()
            .unwrap()
            .worker
            .is_some()
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn promotion_preserves_remaining_queue_fibers() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let a = live(store);
    let b = live(store);
    let state = store.concurrent_state_mut().unwrap();
    let q = thread();
    state.push_low_priority(WorkItem::ResumeFiber {
        instance: instance(),
        thread: q,
        fiber: a,
    });
    state.push_low_priority(WorkItem::ResumeFiber {
        instance: instance(),
        thread: q,
        fiber: b,
    });
    assert!(state.promote_work_item_matching(|_| true).unwrap());
    assert!(state.switch_item.is_some());
    assert_eq!(state.low_priority.len(), 1);
}

#[test]
#[cfg_attr(miri, ignore)]
fn initial_thread_lookup_error_disposes_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = live(store);
    let state = store.concurrent_state_mut().unwrap();
    state.unforced_current_thread = CurrentThread::Guest(thread());
    // The missing base task is checked before walking deferred frames.
    // This marker is never dereferenced; clear it again after the error.
    *store.vm_store_context_mut().current_thread_mut() =
        VMLazyThread::deferred(NonNull::dangling());
    let mut future = Box::pin(store.resume_fiber(fiber));
    let Poll::Ready(Err(error)) = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    else {
        panic!("expected lookup error");
    };
    assert!(matches!(
        error.downcast_ref::<ResourceTableError>(),
        Some(ResourceTableError::NotPresent)
    ));
    drop(future);
    *store.vm_store_context_mut().current_thread_mut() = VMLazyThread::forced();
}

#[test]
#[cfg_attr(miri, ignore)]
fn post_release_thread_lookup_error_disposes_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    store
        .concurrent_state_mut()
        .unwrap()
        .unforced_current_thread = CurrentThread::Guest(thread());
    let fiber = unsafe {
        fiber::make_fiber_unchecked(store, |store| {
            store.concurrent_state_mut()?.suspend_reason = Some(SuspendReason::NeedWork);
            store.with_blocking(|_, cx| cx.suspend(StoreFiberYield::ReleaseStore))?;
            Ok(())
        })
    }
    .unwrap();
    let mut future = Box::pin(store.resume_fiber(fiber));
    let Poll::Ready(Err(error)) = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    else {
        panic!("expected post-release lookup error");
    };
    assert!(matches!(
        error.downcast_ref::<ResourceTableError>(),
        Some(ResourceTableError::NotPresent)
    ));
}

#[test]
#[cfg_attr(miri, ignore)]
fn suspended_destination_keeps_old_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let old = live(store);
    let q = add_thread(store.concurrent_state_mut().unwrap(), old);
    let fiber = unsafe {
        fiber::make_fiber_unchecked(store, move |store| {
            store.concurrent_state_mut()?.suspend_reason =
                Some(SuspendReason::ExplicitlySuspending { thread: q });
            store.with_blocking(|_, cx| cx.suspend(StoreFiberYield::ReleaseStore))?;
            Ok(())
        })
    }
    .unwrap();
    bug(
        || {
            let mut future = Box::pin(store.resume_fiber(fiber));
            match future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                Poll::Ready(r) => r,
                _ => panic!("expected completion"),
            }
        },
        "thread already owns a fiber",
    );
    assert!(matches!(
        store
            .concurrent_state_mut()
            .unwrap()
            .get_mut(q.thread)
            .unwrap()
            .state,
        GuestThreadState::Suspended(_)
    ));
}

#[test]
#[cfg_attr(miri, ignore)]
fn duplicate_waiter_keeps_old_fiber() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let mut s = store();
    let store = s.as_context_mut().0;
    let old = live(store);
    let state = store.concurrent_state_mut().unwrap();
    let set = state.push(WaitableSet::default()).unwrap();
    state
        .get_mut(set)
        .unwrap()
        .waiting
        .insert(thread(), WaitMode::Fiber(old));
    let fiber = unsafe {
        fiber::make_fiber_unchecked(store, move |store| {
            store.concurrent_state_mut()?.suspend_reason = Some(SuspendReason::Waiting {
                set,
                thread: thread(),
            });
            store.with_blocking(|_, cx| cx.suspend(StoreFiberYield::ReleaseStore))?;
            Ok(())
        })
    }
    .unwrap();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut future = Box::pin(store.resume_fiber(fiber));
        let _ = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
    }));
    assert!(result.is_err());
    assert!(
        store
            .concurrent_state_mut()
            .unwrap()
            .get_mut(set)
            .unwrap()
            .waiting
            .contains_key(&thread())
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn invalid_waiter_task_lookup_keeps_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = live(store);
    let old = live(store);
    let state = store.concurrent_state_mut().unwrap();
    let q = add_thread(state, old);
    let set = state.push(WaitableSet::default()).unwrap();
    let group = state.make_task_group().unwrap();
    let host = state
        .push(HostTask {
            common: WaitableCommon {
                set: Some(set),
                ..WaitableCommon::default()
            },
            call_context: CallContext::default(),
            state: HostTaskState::CalleeStarted,
            group,
        })
        .unwrap();
    state
        .get_mut(set)
        .unwrap()
        .waiting
        .insert(q, WaitMode::Fiber(fiber));
    let error = Waitable::Host(host).mark_ready(state).unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ResourceTableError>(),
        Some(ResourceTableError::NotPresent)
    ));
    assert!(state.get_mut(set).unwrap().waiting.contains_key(&q));
}

#[test]
#[cfg_attr(miri, ignore)]
fn subtask_lookup_error_keeps_parked_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let old = live(store);
    store.concurrent_state_mut().unwrap().next_switch_item = Some(WorkItem::ResumeFiber {
        instance: instance(),
        thread: thread(),
        fiber: old,
    });
    // Invalid task lookup still precedes the occupied-slot check.
    // Both incoming and parked fibers must survive that early error.
    let fiber = unsafe {
        fiber::make_fiber_unchecked(store, move |store| {
            store.concurrent_state_mut()?.suspend_reason =
                Some(SuspendReason::YieldingToSubtask { thread: thread() });
            store.with_blocking(|_, cx| cx.suspend(StoreFiberYield::ReleaseStore))?;
            Ok(())
        })
    }
    .unwrap();
    let mut future = Box::pin(store.resume_fiber(fiber));
    let Poll::Ready(Err(error)) = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    else {
        panic!("expected original task lookup error");
    };
    assert!(matches!(
        error.downcast_ref::<ResourceTableError>(),
        Some(ResourceTableError::NotPresent)
    ));
    drop(future);
    assert!(
        store
            .concurrent_state_mut()
            .unwrap()
            .next_switch_item
            .is_some()
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn occupied_resume_schedule_keeps_suspended_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = live(store);
    let state = store.concurrent_state_mut().unwrap();
    let q = add_thread(state, fiber);
    state.switch_item = Some(WorkItem::ResumeThread {
        instance: instance(),
        thread: thread(),
    });
    bug(
        || state.schedule_suspended_thread(q, instance(), Priority::Switch),
        "switch item already set",
    );
    assert!(matches!(
        state.get_mut(q.thread).unwrap().state,
        GuestThreadState::Suspended(_)
    ));
    assert!(state.high_priority.is_empty());
}
#[test]
#[cfg_attr(miri, ignore)]
fn schedule_rejects_ready_thread_without_dropping_fiber() {
    let mut s = store();
    let store = s.as_context_mut().0;
    let fiber = live(store);
    let state = store.concurrent_state_mut().unwrap();
    let q = add_thread(state, fiber);
    let slot = &mut state.get_mut(q.thread).unwrap().state;
    let GuestThreadState::Suspended(fiber) = mem::replace(slot, GuestThreadState::Running) else {
        unreachable!()
    };
    *slot = GuestThreadState::Ready { fiber };
    let error = state
        .schedule_suspended_thread(q, instance(), Priority::Low)
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<Trap>(),
        Some(Trap::CannotResumeThread)
    ));
    assert!(matches!(
        state.get_mut(q.thread).unwrap().state,
        GuestThreadState::Ready { .. }
    ));
}

#[test]
#[cfg_attr(miri, ignore)]
fn occupied_subtask_switch_keeps_both_fibers() {
    let mut s = store();
    let engine = s.engine().clone();
    let component = crate::component::Component::new(
        &engine,
        r#"
        (component
          (core module $m (func (export "f")))
          (core instance $i (instantiate $m))
          (func (export "f") (canon lift (core func $i "f"))))
    "#,
    )
    .unwrap();
    let inst = crate::component::Linker::new(&engine)
        .instantiate(&mut s, &component)
        .unwrap();
    let func = inst.get_func(&mut s, "f").unwrap();
    let mut results = [];
    let call = func
        .start_call_concurrent(&mut s, &[], &mut results)
        .unwrap();
    let task = s
        .as_context_mut()
        .0
        .concurrent_state_mut()
        .unwrap()
        .table
        .get_mut()
        .iter_mut()
        .find_map(|(rep, entry)| entry.downcast_mut::<GuestTask>().map(|_| TableId::new(rep)))
        .unwrap();
    let store = s.as_context_mut().0;
    let q = QualifiedThreadId {
        task,
        thread: thread().thread,
    };
    let parked = live(store);
    store.concurrent_state_mut().unwrap().next_switch_item = Some(WorkItem::ResumeFiber {
        instance: instance(),
        thread: thread(),
        fiber: parked,
    });
    let fiber = unsafe {
        fiber::make_fiber_unchecked(store, move |store| {
            store.concurrent_state_mut()?.suspend_reason =
                Some(SuspendReason::YieldingToSubtask { thread: q });
            store.with_blocking(|_, cx| cx.suspend(StoreFiberYield::ReleaseStore))?;
            Ok(())
        })
    }
    .unwrap();
    bug(
        || {
            let mut future = Box::pin(store.resume_fiber(fiber));
            match future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                Poll::Ready(r) => r,
                _ => panic!("expected completion"),
            }
        },
        "was already",
    );
    assert!(
        store
            .concurrent_state_mut()
            .unwrap()
            .next_switch_item
            .is_some()
    );
    drop(call);
}

#[test]
#[cfg_attr(miri, ignore)]
fn waiter_assertion_keeps_fiber_in_set() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let mut s = store();
    let store = s.as_context_mut().0;
    let old = live(store);
    let fiber = live(store);
    let state = store.concurrent_state_mut().unwrap();
    let q = add_thread(state, old);
    let set = state.push(WaitableSet::default()).unwrap();
    state.get_mut(q.thread).unwrap().wake_on_cancel = WakeOnCancel::Yielding;
    let group = state.make_task_group().unwrap();
    let host = state
        .push(HostTask {
            common: WaitableCommon {
                set: Some(set),
                ..WaitableCommon::default()
            },
            call_context: CallContext::default(),
            state: HostTaskState::CalleeStarted,
            group,
        })
        .unwrap();
    state
        .get_mut(set)
        .unwrap()
        .waiting
        .insert(q, WaitMode::Fiber(fiber));
    let result = catch_unwind(AssertUnwindSafe(|| Waitable::Host(host).mark_ready(state)));
    assert!(result.is_err());
    assert!(state.get_mut(set).unwrap().waiting.contains_key(&q));
}
