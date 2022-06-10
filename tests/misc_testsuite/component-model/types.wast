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
