;; Each core result type is passed to `post-return` unchanged, which traps if
;; it sees anything else.
(component
  (core module $m
    (func (export "i32") (result i32) i32.const 1)
    (func (export "i64") (result i64) i64.const 2)
    (func (export "f32") (result f32) f32.const 3)
    (func (export "f64") (result f64) f64.const 4)

    (func (export "post-i32") (param i32)
      (if (i32.ne (local.get 0) (i32.const 1)) (then unreachable)))
    (func (export "post-i64") (param i64)
      (if (i64.ne (local.get 0) (i64.const 2)) (then unreachable)))
    (func (export "post-f32") (param f32)
      (if (f32.ne (local.get 0) (f32.const 3)) (then unreachable)))
    (func (export "post-f64") (param f64)
      (if (f64.ne (local.get 0) (f64.const 4)) (then unreachable)))
  )
  (core instance $i (instantiate $m))

  (func (export "i32") (result u32)
    (canon lift (core func $i "i32") (post-return (core func $i "post-i32"))))
  (func (export "i64") (result u64)
    (canon lift (core func $i "i64") (post-return (core func $i "post-i64"))))
  (func (export "f32") (result f32)
    (canon lift (core func $i "f32") (post-return (core func $i "post-f32"))))
  (func (export "f64") (result f64)
    (canon lift (core func $i "f64") (post-return (core func $i "post-f64"))))
)

(assert_return (invoke "i32") (u32.const 1))
(assert_return (invoke "i64") (u64.const 2))
(assert_return (invoke "f32") (f32.const 3))
(assert_return (invoke "f64") (f64.const 4))

;; `post-return` receives the return pointer of a string result.
(component
  (core module $m
    (memory (export "memory") 1)
    (func (export "get") (result i32)
      (i32.store offset=0 (i32.const 8) (i32.const 100))
      (i32.store offset=4 (i32.const 8) (i32.const 11))
      i32.const 8)
    (func (export "post") (param i32)
      (if (i32.ne (local.get 0) (i32.const 8)) (then unreachable)))
    (data (i32.const 100) "hello world")
  )
  (core instance $i (instantiate $m))

  (func (export "get") (result string)
    (canon lift
      (core func $i "get")
      (post-return (core func $i "post"))
      (memory (core memory $i "memory"))
    )
  )
)

(assert_return (invoke "get") (str.const "hello world"))

;; A trap in `post-return` poisons the instance.
(component
  (core module $m
    (func (export "f"))
    (func (export "post") unreachable)
  )
  (core instance $i (instantiate $m))

  (func (export "f")
    (canon lift (core func $i "f") (post-return (core func $i "post"))))
)

(assert_trap (invoke "f") "unreachable")
(assert_trap (invoke "f") "cannot enter component instance")
