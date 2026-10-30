;;! threads = true

;; Two instances of one module importing the same shared memory see a write
;; made through a third instance.
(module $Mem
  (memory (export "shared") 1 5 shared)
)

(module $Writer
  (import "Mem" "shared" (memory 1 5 shared))
  (func (export "write") (i32.store (i32.const 0) (i32.const 42)))
)
(invoke $Writer "write")

(module definition $Reader
  (import "Mem" "shared" (memory 1 5 shared))
  (func (export "first-word") (result i32) (i32.load (i32.const 0)))
)
(module instance $Reader1 $Reader)
(module instance $Reader2 $Reader)
(assert_return (invoke $Reader1 "first-word") (i32.const 42))
(assert_return (invoke $Reader2 "first-word") (i32.const 42))

;; Concurrent `memory.grow` on one shared memory from four threads.
;; - Each thread grows by one page 250 times. The memory starts at 1 page with
;;   a maximum of 1000, so exactly one of the 1000 grows fails.
;; - The old sizes a thread sees must increase.
;; - No old size may be returned twice. Each grow marks byte `old size` of a
;;   bitmap at offset 0, and traps if that byte was already marked.
;; - Failed grows are counted at offset 2000.
(module $GrowMem
  (memory (export "shared") 1 1000 shared)
)

(module definition $Grower
  (import "GrowMem" "shared" (memory 1 1000 shared))
  (func (export "run")
    (local $i i32) (local $old i32) (local $last i32)
    (local.set $last (i32.const -1))
    (loop $loop
      (local.set $old (memory.grow (i32.const 1)))
      (if (i32.eq (local.get $old) (i32.const -1))
        (then
          (drop (i32.atomic.rmw.add (i32.const 2000) (i32.const 1))))
        (else
          ;; Old sizes from one thread must increase.
          (if (i32.and
                (i32.ne (local.get $last) (i32.const -1))
                (i32.le_u (local.get $old) (local.get $last)))
            (then unreachable))
          (local.set $last (local.get $old))
          ;; No other grow may have returned this size.
          (if (i32.atomic.rmw8.xchg_u (local.get $old) (i32.const 1))
            (then unreachable))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $loop (i32.lt_u (local.get $i) (i32.const 250)))
    )
  )
)

(thread $T1 (shared (module $GrowMem))
  (module instance $G $Grower)
  (assert_return (invoke $G "run"))
)
(thread $T2 (shared (module $GrowMem))
  (module instance $G $Grower)
  (assert_return (invoke $G "run"))
)
(thread $T3 (shared (module $GrowMem))
  (module instance $G $Grower)
  (assert_return (invoke $G "run"))
)
(thread $T4 (shared (module $GrowMem))
  (module instance $G $Grower)
  (assert_return (invoke $G "run"))
)
(wait $T1)
(wait $T2)
(wait $T3)
(wait $T4)

(module $GrowCheck
  (import "GrowMem" "shared" (memory 1 1000 shared))
  (func (export "size") (result i32) (memory.size))
  (func (export "failures") (result i32) (i32.atomic.load (i32.const 2000)))
  ;; Returns how many of the old sizes 1..999 were seen.
  (func (export "seen") (result i32)
    (local $i i32) (local $n i32)
    (local.set $i (i32.const 1))
    (loop $loop
      (local.set $n
        (i32.add (local.get $n) (i32.load8_u (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $loop (i32.lt_u (local.get $i) (i32.const 1000)))
    )
    (local.get $n)
  )
)
(assert_return (invoke $GrowCheck "size") (i32.const 1000))
(assert_return (invoke $GrowCheck "failures") (i32.const 1))
(assert_return (invoke $GrowCheck "seen") (i32.const 999))

;; One thread grows a shared memory while another repeatedly loads from the
;; last four bytes of the current size. Every load must be in bounds and read
;; zero. The grower sets a flag at offset 0 when it is done.
(module $ProbeMem
  (memory (export "shared") 1 1000 shared)
)

(thread $Grow (shared (module $ProbeMem))
  (module
    (import "ProbeMem" "shared" (memory 1 1000 shared))
    (func (export "run")
      (loop $loop
        (br_if $loop (i32.ne (memory.grow (i32.const 1)) (i32.const -1)))
      )
      (i32.atomic.store (i32.const 0) (i32.const 1))
    )
  )
  (assert_return (invoke "run"))
)
(thread $Probe (shared (module $ProbeMem))
  (module
    (import "ProbeMem" "shared" (memory 1 1000 shared))
    (func (export "run")
      (loop $loop
        (if (i32.load
              (i32.sub
                (i32.mul (memory.size) (i32.const 0x10000))
                (i32.const 4)))
          (then unreachable))
        (br_if $loop (i32.eqz (i32.atomic.load (i32.const 0))))
      )
    )
  )
  (assert_return (invoke "run"))
)
(wait $Grow)
(wait $Probe)
