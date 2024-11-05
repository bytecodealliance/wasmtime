;;! threads = true

(module $Mem
  (memory (export "shared") 1 1 shared)
)

(thread $T1 (shared (module $Mem))
  (register "mem" $Mem)
  (module
    (memory (import "mem" "shared") 1 10 shared)
    (func (export "run")
      (local i32)
      (i32.atomic.load (i32.const 4))
      (local.set 0)
      (i32.atomic.store (i32.const 0) (i32.const 1))

      ;; store results for checking
      (i32.store (i32.const 24) (local.get 0))
    )
  )
  (invoke "run")
)

(thread $T2 (shared (module $Mem))
  (register "mem" $Mem)
  (module
    (memory (import "mem" "shared") 1 1 shared)
    (func (export "run")
      (local i32)
      (i32.atomic.load (i32.const 0))
      (local.set 0)
      (i32.atomic.store (i32.const 4) (i32.const 1))

      ;; store results for checking
      (i32.store (i32.const 32) (local.get 0))
    )
  )

  (invoke "run")
)

(wait $T1)
(wait $T2)

(module $Check
  (memory (import "Mem" "shared") 1 1 shared)

  (func (export "check") (result i32)
    (local i32 i32)
    (i32.load (i32.const 24))
    (local.set 0)
    (i32.load (i32.const 32))
    (local.set 1)

    ;; allowed results: (L_0 = 0 && L_1 = 0) || (L_0 = 0 && L_1 = 1) || (L_0 = 1 && L_1 = 0)

    (i32.and (i32.eq (local.get 0) (i32.const 0)) (i32.eq (local.get 1) (i32.const 0)))
    (i32.and (i32.eq (local.get 0) (i32.const 0)) (i32.eq (local.get 1) (i32.const 1)))
    (i32.and (i32.eq (local.get 0) (i32.const 1)) (i32.eq (local.get 1) (i32.const 0)))
    (i32.or)
    (i32.or)
    (return)
  )
)

(assert_return (invoke $Check "check") (i32.const 1))
