;;! component_model_gc = true
;;! gc = true

;; Tests for the component model integer types under the GC data model.
;; `{s,u}{8,16,32}` lower to a core `i32` value and `{s,u}64` lower to a
;; core `i64` value.
;;
;; See https://github.com/WebAssembly/component-model/issues/525.

;; `u8`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u8) (result u8)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u8) (result u8)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u8.const 0)) (u8.const 0))
(assert_return (invoke "g" (u8.const 1)) (u8.const 1))
(assert_return (invoke "g" (u8.const 127)) (u8.const 127))
(assert_return (invoke "g" (u8.const 128)) (u8.const 128))
(assert_return (invoke "g" (u8.const 255)) (u8.const 255))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u8) (result u8)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u8) (result u8)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u8.const 0)) (u8.const 0))
(assert_return (invoke "g" (u8.const 1)) (u8.const 1))
(assert_return (invoke "g" (u8.const 127)) (u8.const 127))
(assert_return (invoke "g" (u8.const 128)) (u8.const 128))
(assert_return (invoke "g" (u8.const 255)) (u8.const 255))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u8) (result u8)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u8) (result u8)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (u8.const 0)) (u8.const 0))
(assert_return (invoke "g" (u8.const 1)) (u8.const 1))
(assert_return (invoke "g" (u8.const 127)) (u8.const 127))
(assert_return (invoke "g" (u8.const 128)) (u8.const 128))
(assert_return (invoke "g" (u8.const 255)) (u8.const 255))

;; `s8`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s8) (result s8)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s8) (result s8)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s8.const 0)) (s8.const 0))
(assert_return (invoke "g" (s8.const 1)) (s8.const 1))
(assert_return (invoke "g" (s8.const -1)) (s8.const -1))
(assert_return (invoke "g" (s8.const 127)) (s8.const 127))
(assert_return (invoke "g" (s8.const -128)) (s8.const -128))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s8) (result s8)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s8) (result s8)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s8.const 0)) (s8.const 0))
(assert_return (invoke "g" (s8.const 1)) (s8.const 1))
(assert_return (invoke "g" (s8.const -1)) (s8.const -1))
(assert_return (invoke "g" (s8.const 127)) (s8.const 127))
(assert_return (invoke "g" (s8.const -128)) (s8.const -128))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s8) (result s8)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s8) (result s8)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (s8.const 0)) (s8.const 0))
(assert_return (invoke "g" (s8.const 1)) (s8.const 1))
(assert_return (invoke "g" (s8.const -1)) (s8.const -1))
(assert_return (invoke "g" (s8.const 127)) (s8.const 127))
(assert_return (invoke "g" (s8.const -128)) (s8.const -128))

;; `u16`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u16) (result u16)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u16) (result u16)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u16.const 0)) (u16.const 0))
(assert_return (invoke "g" (u16.const 1)) (u16.const 1))
(assert_return (invoke "g" (u16.const 32767)) (u16.const 32767))
(assert_return (invoke "g" (u16.const 32768)) (u16.const 32768))
(assert_return (invoke "g" (u16.const 65535)) (u16.const 65535))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u16) (result u16)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u16) (result u16)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u16.const 0)) (u16.const 0))
(assert_return (invoke "g" (u16.const 1)) (u16.const 1))
(assert_return (invoke "g" (u16.const 32767)) (u16.const 32767))
(assert_return (invoke "g" (u16.const 32768)) (u16.const 32768))
(assert_return (invoke "g" (u16.const 65535)) (u16.const 65535))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u16) (result u16)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u16) (result u16)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (u16.const 0)) (u16.const 0))
(assert_return (invoke "g" (u16.const 1)) (u16.const 1))
(assert_return (invoke "g" (u16.const 32767)) (u16.const 32767))
(assert_return (invoke "g" (u16.const 32768)) (u16.const 32768))
(assert_return (invoke "g" (u16.const 65535)) (u16.const 65535))

;; `s16`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s16) (result s16)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s16) (result s16)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s16.const 0)) (s16.const 0))
(assert_return (invoke "g" (s16.const 1)) (s16.const 1))
(assert_return (invoke "g" (s16.const -1)) (s16.const -1))
(assert_return (invoke "g" (s16.const 32767)) (s16.const 32767))
(assert_return (invoke "g" (s16.const -32768)) (s16.const -32768))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s16) (result s16)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s16) (result s16)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s16.const 0)) (s16.const 0))
(assert_return (invoke "g" (s16.const 1)) (s16.const 1))
(assert_return (invoke "g" (s16.const -1)) (s16.const -1))
(assert_return (invoke "g" (s16.const 32767)) (s16.const 32767))
(assert_return (invoke "g" (s16.const -32768)) (s16.const -32768))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s16) (result s16)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s16) (result s16)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (s16.const 0)) (s16.const 0))
(assert_return (invoke "g" (s16.const 1)) (s16.const 1))
(assert_return (invoke "g" (s16.const -1)) (s16.const -1))
(assert_return (invoke "g" (s16.const 32767)) (s16.const 32767))
(assert_return (invoke "g" (s16.const -32768)) (s16.const -32768))

;; `u32`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u32) (result u32)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (result u32)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u32.const 0)) (u32.const 0))
(assert_return (invoke "g" (u32.const 1)) (u32.const 1))
(assert_return (invoke "g" (u32.const 2147483648)) (u32.const 2147483648))
(assert_return (invoke "g" (u32.const 4294967295)) (u32.const 4294967295))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u32) (result u32)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (result u32)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u32.const 0)) (u32.const 0))
(assert_return (invoke "g" (u32.const 1)) (u32.const 1))
(assert_return (invoke "g" (u32.const 2147483648)) (u32.const 2147483648))
(assert_return (invoke "g" (u32.const 4294967295)) (u32.const 4294967295))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u32) (result u32)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (result u32)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (u32.const 0)) (u32.const 0))
(assert_return (invoke "g" (u32.const 1)) (u32.const 1))
(assert_return (invoke "g" (u32.const 2147483648)) (u32.const 2147483648))
(assert_return (invoke "g" (u32.const 4294967295)) (u32.const 4294967295))

;; `s32`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s32) (result s32)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s32) (result s32)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s32.const 0)) (s32.const 0))
(assert_return (invoke "g" (s32.const 1)) (s32.const 1))
(assert_return (invoke "g" (s32.const -1)) (s32.const -1))
(assert_return (invoke "g" (s32.const 2147483647)) (s32.const 2147483647))
(assert_return (invoke "g" (s32.const -2147483648)) (s32.const -2147483648))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s32) (result s32)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s32) (result s32)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s32.const 0)) (s32.const 0))
(assert_return (invoke "g" (s32.const 1)) (s32.const 1))
(assert_return (invoke "g" (s32.const -1)) (s32.const -1))
(assert_return (invoke "g" (s32.const 2147483647)) (s32.const 2147483647))
(assert_return (invoke "g" (s32.const -2147483648)) (s32.const -2147483648))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s32) (result s32)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s32) (result s32)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (s32.const 0)) (s32.const 0))
(assert_return (invoke "g" (s32.const 1)) (s32.const 1))
(assert_return (invoke "g" (s32.const -1)) (s32.const -1))
(assert_return (invoke "g" (s32.const 2147483647)) (s32.const 2147483647))
(assert_return (invoke "g" (s32.const -2147483648)) (s32.const -2147483648))

;; `u64`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i64) (result i64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u64) (result u64)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i64) (result i64)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i64) (result i64)))
    (func (export "g") (param i64) (result i64) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u64) (result u64)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u64.const 0)) (u64.const 0))
(assert_return (invoke "g" (u64.const 1)) (u64.const 1))
(assert_return (invoke "g" (u64.const 9223372036854775807)) (u64.const 9223372036854775807))
(assert_return (invoke "g" (u64.const 9223372036854775808)) (u64.const 9223372036854775808))
(assert_return (invoke "g" (u64.const 18446744073709551615)) (u64.const 18446744073709551615))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i64) (result i64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u64) (result u64)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i64) (result i64)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i64) (result i64)))
    (func (export "g") (param i64) (result i64) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u64) (result u64)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u64.const 0)) (u64.const 0))
(assert_return (invoke "g" (u64.const 1)) (u64.const 1))
(assert_return (invoke "g" (u64.const 9223372036854775807)) (u64.const 9223372036854775807))
(assert_return (invoke "g" (u64.const 9223372036854775808)) (u64.const 9223372036854775808))
(assert_return (invoke "g" (u64.const 18446744073709551615)) (u64.const 18446744073709551615))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i64) (result i64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u64) (result u64)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i64) (result i64)))
    (memory (export "m") 1)
    (func (export "g") (param i64) (result i64) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u64) (result u64)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (u64.const 0)) (u64.const 0))
(assert_return (invoke "g" (u64.const 1)) (u64.const 1))
(assert_return (invoke "g" (u64.const 9223372036854775807)) (u64.const 9223372036854775807))
(assert_return (invoke "g" (u64.const 9223372036854775808)) (u64.const 9223372036854775808))
(assert_return (invoke "g" (u64.const 18446744073709551615)) (u64.const 18446744073709551615))

;; `s64`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i64) (result i64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s64) (result s64)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i64) (result i64)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i64) (result i64)))
    (func (export "g") (param i64) (result i64) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s64) (result s64)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s64.const 0)) (s64.const 0))
(assert_return (invoke "g" (s64.const 1)) (s64.const 1))
(assert_return (invoke "g" (s64.const -1)) (s64.const -1))
(assert_return (invoke "g" (s64.const 9223372036854775807)) (s64.const 9223372036854775807))
(assert_return (invoke "g" (s64.const -9223372036854775808)) (s64.const -9223372036854775808))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i64) (result i64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s64) (result s64)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i64) (result i64)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i64) (result i64)))
    (func (export "g") (param i64) (result i64) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s64) (result s64)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s64.const 0)) (s64.const 0))
(assert_return (invoke "g" (s64.const 1)) (s64.const 1))
(assert_return (invoke "g" (s64.const -1)) (s64.const -1))
(assert_return (invoke "g" (s64.const 9223372036854775807)) (s64.const 9223372036854775807))
(assert_return (invoke "g" (s64.const -9223372036854775808)) (s64.const -9223372036854775808))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i64) (result i64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s64) (result s64)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i64) (result i64)))
    (memory (export "m") 1)
    (func (export "g") (param i64) (result i64) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s64) (result s64)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (s64.const 0)) (s64.const 0))
(assert_return (invoke "g" (s64.const 1)) (s64.const 1))
(assert_return (invoke "g" (s64.const -1)) (s64.const -1))
(assert_return (invoke "g" (s64.const 9223372036854775807)) (s64.const 9223372036854775807))
(assert_return (invoke "g" (s64.const -9223372036854775808)) (s64.const -9223372036854775808))

;; `u8` masks off everything above bit 7.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0xffffff01)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u8) (result u8)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u8) (result u8)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u8.const 0)) (u8.const 1))

;; `s8` sign-extends from bit 7.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0xffffff80)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s8) (result s8)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s8) (result s8)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s8.const 0)) (s8.const -128))

;; A positive `s8` keeps its value.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0x0000007f)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s8) (result s8)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s8) (result s8)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s8.const 0)) (s8.const 127))

;; `u16` masks off everything above bit 15.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0xffff0001)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" u16) (result u16)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u16) (result u16)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (u16.const 0)) (u16.const 1))

;; `s16` sign-extends from bit 15.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0xffff8000)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s16) (result s16)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s16) (result s16)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s16.const 0)) (s16.const -32768))

;; A positive `s16` keeps its value.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0x00007fff)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" s16) (result s16)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s16) (result s16)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (s16.const 0)) (s16.const 32767))
