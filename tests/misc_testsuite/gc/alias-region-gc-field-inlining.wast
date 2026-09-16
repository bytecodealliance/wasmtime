;;! gc = true

;; Field regions must be stable across the whole compilation, since inlining
;; merges a callee's regions into its caller by `user_id`.

(component
  (core module $A
    (type $s (sub (struct (field (mut i32)))))

    (func (export "set") (param (ref $s)) (param i32)
      (struct.set $s 0 (local.get 0) (local.get 1)))
  )
  (core instance $a (instantiate $A))

  (core module $B
    (type $unused0 (struct (field f64)))
    (type $unused1 (array (mut i64)))
    (type $s1 (sub (struct (field (mut i32)))))
    ;; The same type again, at another type index.
    (type $s2 (sub (struct (field (mut i32)))))

    (import "" "set" (func $set-cross-module (param (ref $s1)) (param i32)))

    (func $set (param (ref $s2)) (param i32)
      (struct.set $s2 0 (local.get 0) (local.get 1)))

    (func $op (param $x (ref $s1)) (result i32)
      (struct.set $s1 0 (local.get $x) (i32.const 0x1111))
      (call $set (local.get $x) (i32.const 0x2222))
      (struct.get $s1 0 (local.get $x)))  ;; must be 0x2222

    (func (export "inlined") (result i32)
      (call $op (struct.new $s1 (i32.const 0))))

    (func $op-cross-module (param $x (ref $s1)) (result i32)
      (struct.set $s1 0 (local.get $x) (i32.const 0x1111))
      (call $set-cross-module (local.get $x) (i32.const 0x2222))
      (struct.get $s1 0 (local.get $x)))  ;; must be 0x2222

    (func (export "cross-module") (result i32)
      (call $op-cross-module (struct.new $s2 (i32.const 0))))
  )
  (core instance $b (instantiate $B
    (with "" (instance (export "set" (func $a "set"))))))

  (func (export "inlined") (result u32)
    (canon lift (core func $b "inlined")))
  (func (export "cross-module") (result u32)
    (canon lift (core func $b "cross-module")))
)

(assert_return (invoke "inlined") (u32.const 0x2222))
(assert_return (invoke "cross-module") (u32.const 0x2222))
