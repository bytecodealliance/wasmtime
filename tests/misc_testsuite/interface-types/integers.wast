(@interface)

(module
 (@interface func (export "i32-to-s8") (param i32) (result s8)
   arg.get 0
   i32-to-s8)
 (@interface func (export "i32-to-s8x") (param i32) (result s8)
   arg.get 0
   i32-to-s8x)
 (@interface func (export "i32-to-u8") (param i32) (result u8)
   arg.get 0
   i32-to-u8)
)

(assert_return (invoke "i32-to-s8" (i32.const 0)) (s8.const 0))
(assert_return (invoke "i32-to-s8" (i32.const 8)) (s8.const 8))
(assert_return (invoke "i32-to-s8" (i32.const 0x100)) (s8.const 0))
(assert_return (invoke "i32-to-s8" (i32.const 0x100)) (s8.const 0))
(assert_return (invoke "i32-to-s8" (i32.const 0x10021)) (s8.const 0x21))
(assert_return (invoke "i32-to-s8" (i32.const -1)) (s8.const -1))
(assert_return (invoke "i32-to-s8" (i32.const 0xffffff00)) (s8.const 0))

(assert_return (invoke "i32-to-s8x" (i32.const 0)) (s8.const 0))
(assert_return (invoke "i32-to-s8x" (i32.const 8)) (s8.const 8))
(assert_return (invoke "i32-to-s8x" (i32.const -1)) (s8.const -1))
(assert_trap (invoke "i32-to-s8x" (i32.const 0x100)) "overflow")
(assert_trap (invoke "i32-to-s8x" (i32.const -129)) "overflow")

(assert_return (invoke "i32-to-u8" (i32.const 0)) (u8.const 0))
(assert_return (invoke "i32-to-u8" (i32.const 8)) (u8.const 8))
(assert_return (invoke "i32-to-u8" (i32.const 0x100)) (u8.const 0))
(assert_return (invoke "i32-to-u8" (i32.const 0x100)) (u8.const 0))
(assert_return (invoke "i32-to-u8" (i32.const 0x10021)) (u8.const 0x21))
(assert_return (invoke "i32-to-u8" (i32.const -1)) (u8.const 255))
(assert_return (invoke "i32-to-u8" (i32.const 0xffffff00)) (u8.const 0))
