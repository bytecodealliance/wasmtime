;;! component_model_gc = true
;;! gc = true

;; Tests for the component model `tuple` type under the GC data model, where a
;; tuple lowers to a `(ref null (struct ...))` whose fields are the lowerings
;; of the tuple's element types.
;;
;; Note that the tuples here never cross the host boundary, because lifting a
;; GC-lowered type into a host value is not implemented yet. Each test instead
;; builds its tuple inside a core module and reads a scalar back out.
;;
;; See https://github.com/WebAssembly/component-model/issues/525.

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (type $t (struct (field (mut i32)) (field (mut i32))))
      (func (export "f") (param (ref null $t)) (result (ref null $t))
        local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" (tuple u32 u32)) (result (tuple u32 u32))
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $t (struct (field (mut i32)) (field (mut i32))))
  (core type $sig (func (param (ref null $t)) (result (ref null $t))))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (type $t (struct (field (mut i32)) (field (mut i32))))
    (import "" "f" (func $f (param (ref null $t)) (result (ref null $t))))
    (func (export "g") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      struct.new $t
      call $f
      struct.get $t 0)
    (func (export "h") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      struct.new $t
      call $f
      struct.get $t 1)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (param "b" u32) (result u32)
    (canon lift (core func $i "g") gc))
  (func (export "h") (param "a" u32) (param "b" u32) (result u32)
    (canon lift (core func $i "h") gc))
)

(assert_return (invoke "g" (u32.const 1) (u32.const 2)) (u32.const 1))
(assert_return (invoke "h" (u32.const 1) (u32.const 2)) (u32.const 2))

;; A single-element tuple.
(component
  (component $A
    (core module $m
      (type $t (struct (field (mut i32))))
      (func (export "f") (param (ref null $t)) (result (ref null $t))
        local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" (tuple u32)) (result (tuple u32))
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $t (struct (field (mut i32))))
  (core type $sig (func (param (ref null $t)) (result (ref null $t))))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (type $t (struct (field (mut i32))))
    (import "" "f" (func $f (param (ref null $t)) (result (ref null $t))))
    (func (export "g") (param i32) (result i32)
      local.get 0
      struct.new $t
      call $f
      struct.get $t 0)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (result u32)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u32.const 42)) (u32.const 42))

;; A tuple of mixed widths, including immutable fields.
(component
  (component $A
    (core module $m
      (type $t (struct (field i8) (field (mut i64)) (field f64)))
      (func (export "f") (param (ref null $t)) (result (ref null $t))
        local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" (tuple u8 u64 f64)) (result (tuple u8 u64 f64))
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $t (struct (field i8) (field (mut i64)) (field f64)))
  (core type $sig (func (param (ref null $t)) (result (ref null $t))))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (type $t (struct (field i8) (field (mut i64)) (field f64)))
    (import "" "f" (func $f (param (ref null $t)) (result (ref null $t))))
    (func $make (param i32 i64 f64) (result (ref null $t))
      local.get 0
      local.get 1
      local.get 2
      struct.new $t
      call $f)
    (func (export "g") (param i32 i64 f64) (result i32)
      local.get 0
      local.get 1
      local.get 2
      call $make
      struct.get_u $t 0)
    (func (export "h") (param i32 i64 f64) (result i64)
      local.get 0
      local.get 1
      local.get 2
      call $make
      struct.get $t 1)
    (func (export "i") (param i32 i64 f64) (result f64)
      local.get 0
      local.get 1
      local.get 2
      call $make
      struct.get $t 2)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (param "b" u64) (param "c" f64) (result u32)
    (canon lift (core func $i "g") gc))
  (func (export "h") (param "a" u32) (param "b" u64) (param "c" f64) (result u64)
    (canon lift (core func $i "h") gc))
  (func (export "i") (param "a" u32) (param "b" u64) (param "c" f64) (result f64)
    (canon lift (core func $i "i") gc))
)

(assert_return (invoke "g" (u32.const 0xff) (u64.const 0) (f64.const 0)) (u32.const 0xff))
(assert_return
  (invoke "h" (u32.const 0) (u64.const 0xfedcba9876543210) (f64.const 0))
  (u64.const 0xfedcba9876543210))
(assert_return
  (invoke "i" (u32.const 0) (u64.const 0) (f64.const 1.5))
  (f64.const 1.5))
(assert_return
  (invoke "i" (u32.const 0) (u64.const 0) (f64.const nan:0x4000000000000))
  (f64.const nan:0x4000000000000))

;; A GC-lowered caller calling a linear-memory-lifted callee. The tuple's
;; fields use packed storage types, and the callee widens each one to an
;; `i32`, which is where the choice between `struct.get_s` and `struct.get_u`
;; becomes observable.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32 i32) (result i32) local.get 0)
      (func (export "f2") (param i32 i32) (result i32) local.get 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" (tuple s8 s16)) (result s32)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
    (func (export "f2") (param "a" (tuple s8 s16)) (result s32)
      (canon lift (core func $i "f2") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))
  (alias export $a "f2" (func $af2))

  (core type $t (struct (field (mut i8)) (field (mut i16))))
  (core type $sig (func (param (ref null $t)) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))
  (core func $l2 (canon lower (func $af2) gc (core-type $sig)))

  (core module $m
    (type $t (struct (field (mut i8)) (field (mut i16))))
    (import "" "f" (func $f (param (ref null $t)) (result i32)))
    (import "" "f2" (func $f2 (param (ref null $t)) (result i32)))
    (func (export "g") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      struct.new $t
      call $f)
    (func (export "h") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      struct.new $t
      call $f2)
  )
  (core instance $i (instantiate $m
    (with "" (instance (export "f" (func $l)) (export "f2" (func $l2))))))

  (func (export "g") (param "a" s32) (param "b" s32) (result s32)
    (canon lift (core func $i "g") gc))
  (func (export "h") (param "a" s32) (param "b" s32) (result s32)
    (canon lift (core func $i "h") gc))
)

(assert_return (invoke "g" (s32.const -1) (s32.const -2)) (s32.const -1))
(assert_return (invoke "h" (s32.const -1) (s32.const -2)) (s32.const -2))
(assert_return (invoke "g" (s32.const -128) (s32.const -32768)) (s32.const -128))
(assert_return (invoke "h" (s32.const -128) (s32.const -32768)) (s32.const -32768))

;; Unsigned packed fields are zero-extended rather than sign-extended.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32 i32) (result i32) local.get 0)
      (func (export "f2") (param i32 i32) (result i32) local.get 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" (tuple u8 u16)) (result u32)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
    (func (export "f2") (param "a" (tuple u8 u16)) (result u32)
      (canon lift (core func $i "f2") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))
  (alias export $a "f2" (func $af2))

  (core type $t (struct (field (mut i8)) (field (mut i16))))
  (core type $sig (func (param (ref null $t)) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))
  (core func $l2 (canon lower (func $af2) gc (core-type $sig)))

  (core module $m
    (type $t (struct (field (mut i8)) (field (mut i16))))
    (import "" "f" (func $f (param (ref null $t)) (result i32)))
    (import "" "f2" (func $f2 (param (ref null $t)) (result i32)))
    (func (export "g") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      struct.new $t
      call $f)
    (func (export "h") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      struct.new $t
      call $f2)
  )
  (core instance $i (instantiate $m
    (with "" (instance (export "f" (func $l)) (export "f2" (func $l2))))))

  (func (export "g") (param "a" u32) (param "b" u32) (result u32)
    (canon lift (core func $i "g") gc))
  (func (export "h") (param "a" u32) (param "b" u32) (result u32)
    (canon lift (core func $i "h") gc))
)

(assert_return (invoke "g" (u32.const 0xff) (u32.const 0xffff)) (u32.const 0xff))
(assert_return (invoke "h" (u32.const 0xff) (u32.const 0xffff)) (u32.const 0xffff))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (type $t (struct (field (mut i32)) (field (mut i32))))
      (func (export "f") (param (ref null $t)) (result i32)
        local.get 0
        struct.get $t 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" (tuple u32 u32)) (result u32)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32 i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (param "b" u32) (result u32)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (u32.const 3) (u32.const 4)) (u32.const 4))

;; The tuple's struct type sits in a multi-member rec group, alongside members
;; that are unrelated to it and that refer to each other. Core Wasm
;; canonicalizes types at whole-rec-group granularity, so the adapter must
;; clone the entire group, preserving member order, for its struct type to be
;; the same type as the caller's and the callee's.
(component
  (component $A
    (core module $m
      (rec
        (type $a (struct (field (ref null $b))))
        (type $t (struct (field (mut i32)) (field (mut i32))))
        (type $b (struct (field (ref null $a))))
      )
      (func (export "f") (param (ref null $t)) (result (ref null $t))
        local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" (tuple u32 u32)) (result (tuple u32 u32))
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core rec
    (type $a (struct (field (ref null $b))))
    (type $t (struct (field (mut i32)) (field (mut i32))))
    (type $b (struct (field (ref null $a))))
  )
  (core type $sig (func (param (ref null $t)) (result (ref null $t))))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (rec
      (type $a (struct (field (ref null $b))))
      (type $t (struct (field (mut i32)) (field (mut i32))))
      (type $b (struct (field (ref null $a))))
    )
    (import "" "f" (func $f (param (ref null $t)) (result (ref null $t))))
    (func (export "g") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      struct.new $t
      call $f
      struct.get $t 1)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (param "b" u32) (result u32)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u32.const 7) (u32.const 9)) (u32.const 9))

;; A GC adapter with no params and no results, and one with a tuple param and
;; no result. Validation requires the declared `core-type` to have exactly as
;; many parameters as the component function, and at most one result, so these
;; exercise the empty ends of that 1:1 mapping.
(component
  (component $A
    (core module $m
      (type $t (struct (field (mut i32))))
      (global $g (export "g") (mut i32) (i32.const 0))
      (func (export "f"))
      (func (export "f2") (param (ref null $t))
        local.get 0
        struct.get $t 0
        global.set $g)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (canon lift (core func $i "f") gc))
    (func (export "f2") (param "a" (tuple u32))
      (canon lift (core func $i "f2") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))
  (alias export $a "f2" (func $af2))

  (core type $t (struct (field (mut i32))))
  (core type $sig (func))
  (core type $sig2 (func (param (ref null $t))))
  (core func $l (canon lower (func $af) gc (core-type $sig)))
  (core func $l2 (canon lower (func $af2) gc (core-type $sig2)))

  (core module $m
    (type $t (struct (field (mut i32))))
    (import "" "f" (func $f))
    (import "" "f2" (func $f2 (param (ref null $t))))
    (func (export "g") (result i32)
      call $f
      i32.const 11)
    (func (export "h") (param i32) (result i32)
      local.get 0
      struct.new $t
      call $f2
      i32.const 22)
  )
  (core instance $i (instantiate $m
    (with "" (instance (export "f" (func $l)) (export "f2" (func $l2))))))

  (func (export "g") (result u32) (canon lift (core func $i "g") gc))
  (func (export "h") (param "a" u32) (result u32)
    (canon lift (core func $i "h") gc))
)

(assert_return (invoke "g") (u32.const 11))
(assert_return (invoke "h" (u32.const 3)) (u32.const 22))
