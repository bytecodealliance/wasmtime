(component
  (type string)
  (type (func (param string)))
  (type $r (record (field "x" unit) (field "y" string)))
  (type $u (union $r string))
  (type $e (expected $u u32))

  (type (func (param $e) (result (option $r))))

  (type (variant
    (case "a" string)
    (case "b" u32)
    (case "c" float32)
    (case "d" float64)
  ))

  (type $errno (enum "a" "b" "e"))
  (type (list $errno))
  (type $oflags (flags "read" "write" "exclusive"))
  (type (tuple $oflags $errno $r))

  ;; primitives in functions
  (type (func
    (param bool)
    (param u8)
    (param s8)
    (param u16)
    (param s16)
    (param u32)
    (param s32)
    (param u64)
    (param s64)
    (param char)
    (param string)
  ))

  ;; primitives in types
  (type bool)
  (type u8)
  (type s8)
  (type u16)
  (type s16)
  (type u32)
  (type s32)
  (type u64)
  (type s64)
  (type char)
  (type string)
)

(component
  (type $empty (func))
  (type (func (param string) (result u32)))
  (type (component))
  (core type (module))
  (core type (func))
  (type (instance))

  (type (component
    (import "" (func (type $empty)))
    (import "y" (func))
    (import "z" (component))

    (type $t (instance))

    (export "a" (core module))
    (export "b" (instance (type $t)))
  ))

  (type (instance
    (export "" (func (type $empty)))
    (export "y" (func))
    (export "z" (component))

    (type $t (instance))

    (export "a" (core module))
    (export "b" (instance (type $t)))
  ))

  (core type (module
    (import "" "" (func (param i32)))
    (import "" "1" (func (result i32)))
    (export "1" (global i32))
    (export "2" (memory 1))
    (export "3" (table 1 funcref))
  ))
)

;; outer core aliases work
(component $C
  (core type $f (func))
  (core type $m (module))

  (component $C2
    (alias outer $C $f (core type $my_f))
    (import "" (core module (type $m)))
    (import "x" (core module
      (alias outer $C2 $my_f (type $my_f))
      (import "" "1" (func (type $my_f)))
    ))
  )
)

;; type exports work
(component $C
  (component $C2
    (type string)
    (export "x" (type 0))
  )
  (instance (instantiate 0))
  (alias export 0 "x" (type))
  (export "x" (type 0))
)

(component
  (core module $m (func (export "") (param i32) (result i32) local.get 0))
  (core instance $m (instantiate $m))
  (func (export "i-to-b") (param u32) (result bool) (canon lift (core func $m "")))
  (func (export "i-to-u8") (param u32) (result u8) (canon lift (core func $m "")))
  (func (export "i-to-s8") (param u32) (result s8) (canon lift (core func $m "")))
  (func (export "i-to-u16") (param u32) (result u16) (canon lift (core func $m "")))
  (func (export "i-to-s16") (param u32) (result s16) (canon lift (core func $m "")))
)
(assert_return (invoke "i-to-b" (u32.const 0)) (bool.const false))
(assert_return (invoke "i-to-b" (u32.const 1)) (bool.const true))
(assert_return (invoke "i-to-b" (u32.const 2)) (bool.const true))
(assert_return (invoke "i-to-u8" (u32.const 0x00)) (u8.const 0))
(assert_return (invoke "i-to-u8" (u32.const 0x01)) (u8.const 1))
(assert_return (invoke "i-to-u8" (u32.const 0xf01)) (u8.const 1))
(assert_return (invoke "i-to-u8" (u32.const 0xf00)) (u8.const 0))
(assert_return (invoke "i-to-s8" (u32.const 0xffffffff)) (s8.const -1))
(assert_return (invoke "i-to-s8" (u32.const 127)) (s8.const 127))
(assert_return (invoke "i-to-u16" (u32.const 0)) (u16.const 0))
(assert_return (invoke "i-to-u16" (u32.const 1)) (u16.const 1))
(assert_return (invoke "i-to-u16" (u32.const 0xffffffff)) (u16.const 0xffff))
(assert_return (invoke "i-to-s16" (u32.const 0)) (s16.const 0))
(assert_return (invoke "i-to-s16" (u32.const 1)) (s16.const 1))
(assert_return (invoke "i-to-s16" (u32.const 0xffffffff)) (s16.const -1))
