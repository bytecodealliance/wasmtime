;;! gc = true

(module
  (type $f0 (func (result i32)))

  ;; Test both typed and untyped function references, as well as `nofunc`
  ;; references.
  (type $s0 (struct (field (mut funcref))))
  (type $s1 (struct (field (mut (ref $f0)))))
  (type $s2 (struct (field (mut (ref null nofunc)))))

  (table 1 1 funcref)

  (func $f (result i32) (i32.const 0x11111111))
  (func $g (result i32) (i32.const 0x22222222))

  (elem declare func $f $g)

  (func $alloc-s0 (export "alloc-s0") (result (ref $s0))
    (struct.new $s0 (ref.func $f))
  )

  (func $alloc-s1 (export "alloc-s1") (result (ref $s1))
    (struct.new $s1 (ref.func $f))
  )

  (func $alloc-s2 (export "alloc-s2") (result (ref $s2))
    (struct.new $s2 (ref.null nofunc))
  )

  (func (export "get-s0") (result i32)
    (table.set (i32.const 0) (struct.get $s0 0 (call $alloc-s0)))
    (call_indirect (type $f0) (i32.const 0))
  )

  (func (export "get-s1") (result i32)
    (table.set (i32.const 0) (struct.get $s1 0 (call $alloc-s1)))
    (call_indirect (type $f0) (i32.const 0))
  )

  (func (export "get-s2") (result i32)
    (table.set (i32.const 0) (struct.get $s2 0 (call $alloc-s2)))
    (call_indirect (type $f0) (i32.const 0))
  )

  (func (export "set-s0") (result i32)
    (local $s (ref $s0))
    (local.set $s (call $alloc-s0))
    (struct.set $s0 0 (local.get $s) (ref.func $g))
    (table.set (i32.const 0) (struct.get $s0 0 (local.get $s)))
    (call_indirect (type $f0) (i32.const 0))
  )

  (func (export "set-s1") (result i32)
    (local $s (ref $s1))
    (local.set $s (call $alloc-s1))
    (struct.set $s1 0 (local.get $s) (ref.func $g))
    (table.set (i32.const 0) (struct.get $s1 0 (local.get $s)))
    (call_indirect (type $f0) (i32.const 0))
  )

  (func (export "set-s2") (result i32)
    (local $s (ref $s2))
    (local.set $s (call $alloc-s2))
    (struct.set $s2 0 (local.get $s) (ref.null nofunc))
    (table.set (i32.const 0) (struct.get $s2 0 (local.get $s)))
    (call_indirect (type $f0) (i32.const 0))
  )
)

(assert_return (invoke "alloc-s0") (ref.struct))
(assert_return (invoke "alloc-s1") (ref.struct))
(assert_return (invoke "alloc-s2") (ref.struct))

(assert_return (invoke "get-s0") (i32.const 0x11111111))
(assert_return (invoke "get-s1") (i32.const 0x11111111))
(assert_trap (invoke "get-s2") "uninitialized element")

(assert_return (invoke "set-s0") (i32.const 0x22222222))
(assert_return (invoke "set-s1") (i32.const 0x22222222))
(assert_trap (invoke "set-s2") "uninitialized element")
