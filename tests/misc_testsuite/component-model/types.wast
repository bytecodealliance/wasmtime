(component
  (type string)
  (type (func (param "a" string)))
  (type $r (record (field "x" (result)) (field "y" string)))
  (type $u (union $r string))
  (type $e (result $u (error u32)))
  (type (result $u))
  (type (result (error $u)))
  (type (result))

  (type (func (param "a" $e) (result (option $r))))

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
    (param "a" bool)
    (param "b" u8)
    (param "c" s8)
    (param "d" u16)
    (param "e" s16)
    (param "f" u32)
    (param "g" s32)
    (param "h" u64)
    (param "i" s64)
    (param "j" char)
    (param "k" string)
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
  (type (func (param "a" string) (result u32)))
  (type (component))
  (core type (module))
  (core type (func))
  (type (instance))

  (type (component
    (import "x" (func (type $empty)))
    (import "y" (func))
    (import "z" (component))

    (type $t (instance))

    (export "a" (core module))
    (export "b" (instance (type $t)))
  ))

  (type (instance
    (export "x" (func (type $empty)))
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
    (import "a" (core module (type $m)))
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
  (func (export "i-to-b") (param "a" u32) (result bool) (canon lift (core func $m "")))
  (func (export "i-to-u8") (param "a" u32) (result u8) (canon lift (core func $m "")))
  (func (export "i-to-s8") (param "a" u32) (result s8) (canon lift (core func $m "")))
  (func (export "i-to-u16") (param "a" u32) (result u16) (canon lift (core func $m "")))
  (func (export "i-to-s16") (param "a" u32) (result s16) (canon lift (core func $m "")))
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

(assert_invalid
  (component
    (type $t1 string)
    (type $t2 (list $t1))
    (type $t3 (list $t2))
    (type $t4 (list $t3))
    (type $t5 (list $t4))
    (type $t6 (list $t5))
    (type $t7 (list $t6))
    (type $t8 (list $t7))
    (type $t9 (list $t8))
    (type $t10 (list $t9))
    (type $t11 (list $t10))
    (type $t12 (list $t11))
    (type $t13 (list $t12))
    (type $t14 (list $t13))
    (type $t15 (list $t14))
    (type $t16 (list $t15))
    (type $t17 (list $t16))
    (type $t18 (list $t17))
    (type $t19 (list $t18))
    (type $t20 (list $t19))
    (type $t21 (list $t20))
    (type $t22 (list $t21))
    (type $t23 (list $t22))
    (type $t24 (list $t23))
    (type $t25 (list $t24))
    (type $t26 (list $t25))
    (type $t27 (list $t26))
    (type $t28 (list $t27))
    (type $t29 (list $t28))
    (type $t30 (list $t29))
    (type $t31 (list $t30))
    (type $t32 (list $t31))
    (type $t33 (list $t32))
    (type $t34 (list $t33))
    (type $t35 (list $t34))
    (type $t36 (list $t35))
    (type $t37 (list $t36))
    (type $t38 (list $t37))
    (type $t39 (list $t38))
    (type $t40 (list $t39))
    (type $t41 (list $t40))
    (type $t42 (list $t41))
    (type $t43 (list $t42))
    (type $t44 (list $t43))
    (type $t45 (list $t44))
    (type $t46 (list $t45))
    (type $t47 (list $t46))
    (type $t48 (list $t47))
    (type $t49 (list $t48))
    (type $t50 (list $t49))
    (type $t51 (list $t50))
    (type $t52 (list $t51))
    (type $t53 (list $t52))
    (type $t54 (list $t53))
    (type $t55 (list $t54))
    (type $t56 (list $t55))
    (type $t57 (list $t56))
    (type $t58 (list $t57))
    (type $t59 (list $t58))
    (type $t60 (list $t59))
    (type $t61 (list $t60))
    (type $t62 (list $t61))
    (type $t63 (list $t62))
    (type $t64 (list $t63))
    (type $t65 (list $t64))
    (type $t66 (list $t65))
    (type $t67 (list $t66))
    (type $t68 (list $t67))
    (type $t69 (list $t68))
    (type $t70 (list $t69))
    (type $t71 (list $t70))
    (type $t72 (list $t71))
    (type $t73 (list $t72))
    (type $t74 (list $t73))
    (type $t75 (list $t74))
    (type $t76 (list $t75))
    (type $t77 (list $t76))
    (type $t78 (list $t77))
    (type $t79 (list $t78))
    (type $t80 (list $t79))
    (type $t81 (list $t80))
    (type $t82 (list $t81))
    (type $t83 (list $t82))
    (type $t84 (list $t83))
    (type $t85 (list $t84))
    (type $t86 (list $t85))
    (type $t87 (list $t86))
    (type $t88 (list $t87))
    (type $t89 (list $t88))
    (type $t90 (list $t89))
    (type $t91 (list $t90))
    (type $t92 (list $t91))
    (type $t93 (list $t92))
    (type $t94 (list $t93))
    (type $t95 (list $t94))
    (type $t96 (list $t95))
    (type $t97 (list $t96))
    (type $t98 (list $t97))
    (type $t99 (list $t98))
    (type $t100 (list $t99))
    (type $t101 (list $t100))
    (export "t" (type $t101))
  )
  "type nesting is too deep")

(component
  (type (instance
    (export $x "x" (instance
      (type $t u32)
      (export "y" (type (eq $t)))
    ))
    (alias export $x "y" (type $t))
    (export "my-y" (type (eq $t)))
  ))

  (type (component
    (import "x" (instance $x
      (type $t u32)
      (export "y" (type (eq $t)))
    ))
    (alias export $x "y" (type $t))
    (export "my-y" (type (eq $t)))
  ))
)

(component
  (type $t u32)
  (export $t2 "t" (type $t))
  (type $r (record (field "x" $t2)))
  (export "r" (type $r))
)

(component
  (component
    (import "x" (instance $i
      (type $i u32)
      (export "i" (type (eq $i)))
    ))
    (alias export $i "i" (type $i))
    (export "i" (type $i))
  )
)

(component
  (type $u u32)
  (instance $i
    (export "i" (type $u))
  )
  (alias export $i "i" (type $i))
  (export "i" (type $i))
)

(component
  (component $c
    (type $t u32)
    (export "t" (type $t))
  )
  (instance $c (instantiate $c))
  (export "i" (type $c "t"))
)

(component
  (component $c
    (import "x" (component $c
      (type $t u32)
      (export "t" (type (eq $t)))
    ))
    (instance $c (instantiate $c))
    (export "i" (type $c "t"))
  )

  (component $x
    (type $t u32)
    (export "t" (type $t))
  )

  (instance $c (instantiate $c (with "x" (component $x))))
)

(component
  (type $t1 u64)
  (import "a" (type $t2 (eq $t1)))
  (import "b" (type $t3 (eq $t2)))
)

(component
  (import "a" (instance
    (type $t1 u64)
    (export $t2 "a" (type (eq $t1)))
    (export "b" (type (eq $t2)))
  ))
)
