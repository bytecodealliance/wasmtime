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

;; Two threads hand off to each other with wait and notify.
;; - $T1 waits on offset 4 until $T2 stores 21 there and notifies.
;; - $T1 then stores 42 at offset 0 and notifies.
;; - $T2 waits on offset 0 until it sees the 42.
;; If a wait starts after the store it waits for, it returns at once.
(module $Mem2
  (memory (export "shared") 1 1 shared)
)

(thread $T1 (shared (module $Mem2))
  (module
    (memory (import "Mem2" "shared") 1 1 shared)
    (func (export "run") (result i32)
      (drop (memory.atomic.wait32 (i32.const 4) (i32.const 0) (i64.const -1)))
      (i32.atomic.store (i32.const 0) (i32.const 42))
      (drop (memory.atomic.notify (i32.const 0) (i32.const -1)))
      (i32.atomic.load (i32.const 0))
    )
  )
  (assert_return (invoke "run") (i32.const 42))
)

(thread $T2 (shared (module $Mem2))
  (module
    (memory (import "Mem2" "shared") 1 1 shared)
    (func (export "run") (result i32)
      (i32.atomic.store (i32.const 4) (i32.const 21))
      (drop (memory.atomic.notify (i32.const 4) (i32.const -1)))
      (drop (memory.atomic.wait32 (i32.const 0) (i32.const 0) (i64.const -1)))
      (i32.atomic.load (i32.const 0))
    )
  )
  (assert_return (invoke "run") (i32.const 42))
)

(wait $T1)
(wait $T2)

(module $Check2
  (memory (import "Mem2" "shared") 1 1 shared)
  (func (export "words") (result i32 i32)
    (i32.load (i32.const 0)) (i32.load (i32.const 4)))
)
(assert_return (invoke $Check2 "words") (i32.const 42) (i32.const 21))
