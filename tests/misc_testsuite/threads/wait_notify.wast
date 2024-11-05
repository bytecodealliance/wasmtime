;;! threads = true

;; test that looping notify eventually unblocks a parallel waiting thread
(module $Mem
  (memory (export "shared") 1 1 shared)
)

(thread $T1 (shared (module $Mem))
  (register "mem" $Mem)
  (module
    (memory (import "mem" "shared") 1 10 shared)
    (func (export "run") (result i32)
      (memory.atomic.wait32 (i32.const 0) (i32.const 0) (i64.const -1))
    )
  )
  ;; test that this thread eventually gets unblocked
  (assert_return (invoke "run") (i32.const 0))
)

(thread $T2 (shared (module $Mem))
  (register "mem" $Mem)
  (module
    (memory (import "mem" "shared") 1 1 shared)
    (func (export "notify-0") (result i32)
      (memory.atomic.notify (i32.const 0) (i32.const 0))
    )
    (func (export "notify-1-while")
      (loop
        (i32.const 1)
        (memory.atomic.notify (i32.const 0) (i32.const 1))
        (i32.ne)
        (br_if 0)
      )
    )
  )
  ;; notifying with a count of 0 will not unblock
  (assert_return (invoke "notify-0") (i32.const 0))
  ;; loop until something is notified
  (assert_return (invoke "notify-1-while"))
)

(wait $T1)
(wait $T2)
