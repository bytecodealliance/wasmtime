(module $tables_of_i31ref
  (table $table 3 10 i31ref)
  (elem (table $table) (i32.const 0) i31ref (item (ref.i31 (i32.const 999)))
                                            (item (ref.i31 (i32.const 888)))
                                            (item (ref.i31 (i32.const 777))))

  (func (export "size") (result i32)
    table.size $table
  )

  (func (export "get") (param i32) (result i32)
    (i31.get_u (table.get $table (local.get 0)))
  )

  (func (export "grow") (param i32 i32) (result i32)
    (table.grow $table (ref.i31 (local.get 1)) (local.get 0))
  )

  (func (export "fill") (param i32 i32 i32)
    (table.fill $table (local.get 0) (ref.i31 (local.get 1)) (local.get 2))
  )

  (func (export "copy") (param i32 i32 i32)
    (table.copy $table $table (local.get 0) (local.get 1) (local.get 2))
  )

  (elem $elem i31ref (item (ref.i31 (i32.const 123)))
                     (item (ref.i31 (i32.const 456)))
                     (item (ref.i31 (i32.const 789))))
  (func (export "init") (param i32 i32 i32)
    (table.init $table $elem (local.get 0) (local.get 1) (local.get 2))
  )
)

;; Initial state.
(assert_return (invoke "size") (i32.const 3))
(assert_return (invoke "get" (i32.const 0)) (i32.const 999))
(assert_return (invoke "get" (i32.const 1)) (i32.const 888))
(assert_return (invoke "get" (i32.const 2)) (i32.const 777))

;; Grow from size 3 to size 5.
(assert_return (invoke "grow" (i32.const 2) (i32.const 333)) (i32.const 3))
(assert_return (invoke "size") (i32.const 5))
(assert_return (invoke "get" (i32.const 3)) (i32.const 333))
(assert_return (invoke "get" (i32.const 4)) (i32.const 333))

;; Fill table[2..4] = 111.
(invoke "fill" (i32.const 2) (i32.const 111) (i32.const 2))
(assert_return (invoke "get" (i32.const 2)) (i32.const 111))
(assert_return (invoke "get" (i32.const 3)) (i32.const 111))

;; Copy from table[0..2] to table[3..5].
(invoke "copy" (i32.const 3) (i32.const 0) (i32.const 2))
(assert_return (invoke "get" (i32.const 3)) (i32.const 999))
(assert_return (invoke "get" (i32.const 4)) (i32.const 888))

;; Initialize the passive element at table[1..4].
(invoke "init" (i32.const 1) (i32.const 0) (i32.const 3))
(assert_return (invoke "get" (i32.const 1)) (i32.const 123))
(assert_return (invoke "get" (i32.const 2)) (i32.const 456))
(assert_return (invoke "get" (i32.const 3)) (i32.const 789))
