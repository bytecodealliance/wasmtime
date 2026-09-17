;;! gc = true

;; A field's region is keyed on the type that introduced it, and every wasm
;; spec-level type index naming the same interned type must agree on that
;; answer.

(component
  (core module $A
    (type $base (sub (struct (field (mut i32)))))
    (type $derived (sub $base (struct (field (mut i32))
                                      (field (mut i32)))))

    (func (export "set-via-base") (param (ref $base)) (param i32)
      (struct.set $base 0 (local.get 0) (local.get 1)))
  )

  (core instance $a (instantiate $A))

  (core module $B
    (type $unused (struct (field f64)))

    (type $base (sub (struct (field (mut i32)))))
    (type $derived (sub $base (struct (field (mut i32))
                                      (field (mut i32)))))

    (type $base2 (sub (struct (field (mut i32)))))
    (type $derived2 (sub $base2 (struct (field (mut i32))
                                        (field (mut i32)))))

    (import "" "set-via-base" (func $set-via-base (param (ref $base)) (param i32)))

    (func $introducer (param $d (ref $derived)) (result i32)
      (struct.set $derived 0 (local.get $d) (i32.const 0x1111))
      (struct.set $base 0 (local.get $d) (i32.const 0x2222))
      ;; A different field, in its own region, which must not satisfy the load.
      (struct.set $derived 1 (local.get $d) (i32.const 0x3333))
      (struct.get $derived 0 (local.get $d)))  ;; must be 0x2222

    (func (export "introducer") (result i32)
      (call $introducer (struct.new $derived (i32.const 0) (i32.const 0))))

    (func $dup-type-index (param $d (ref $derived)) (result i32)
      (struct.set $derived 0 (local.get $d) (i32.const 0x1111))
      (struct.set $base2 0 (local.get $d) (i32.const 0x2222))
      (struct.set $derived2 1 (local.get $d) (i32.const 0x3333))
      (struct.get $derived 0 (local.get $d)))  ;; must be 0x2222

    (func (export "dup-type-index") (result i32)
      (call $dup-type-index (struct.new $derived2 (i32.const 0) (i32.const 0))))

    (func $cross-module (param $d (ref $derived)) (result i32)
      (call $set-via-base (local.get $d) (i32.const 0x2222))
      (struct.get $derived 0 (local.get $d)))  ;; must be 0x2222

    (func (export "cross-module") (result i32)
      (call $cross-module (struct.new $derived (i32.const 0) (i32.const 0))))
  )

  (core instance $b (instantiate $B
    (with "" (instance (export "set-via-base" (func $a "set-via-base"))))))

  (func (export "introducer") (result u32)
    (canon lift (core func $b "introducer")))
  (func (export "dup-type-index") (result u32)
    (canon lift (core func $b "dup-type-index")))
  (func (export "cross-module") (result u32)
    (canon lift (core func $b "cross-module")))
)

(assert_return (invoke "introducer") (u32.const 0x2222))
(assert_return (invoke "dup-type-index") (u32.const 0x2222))
(assert_return (invoke "cross-module") (u32.const 0x2222))
