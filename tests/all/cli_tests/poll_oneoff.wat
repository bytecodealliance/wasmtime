;; When the main thread calls proc_exit, it should terminate
;; a thread blocking in a WASI call. (poll_oneoff)
;;
;; linear memory usage:
;;   0: wait
;;   100: poll_oneoff subscription
;;   200: poll_oneoff event
;;   300: poll_oneoff return value

(module
  (func $proc_exit (import "wasi_snapshot_preview1" "proc_exit") (param i32))
  (func $poll_oneoff (import "wasi_snapshot_preview1" "poll_oneoff") (param i32 i32 i32 i32) (result i32))
  (func (export "_start")
    ;; Set up subscription
    i32.const 124 ;; 100 + offsetof(subscription, timeout)
    i64.const 1_000_000 ;; 1ms
    i64.store

    ;; Wait for poll.
    i32.const 100 ;; subscription
    i32.const 200 ;; event (out)
    i32.const 1   ;; nsubscriptions
    i32.const 300 ;; retp (out)
    call $poll_oneoff

    ;; Check that one result is returned.
    i32.const 1
    i32.ne
    if
      unreachable
    end
  )
  (memory (export "memory") 1 1)
)
