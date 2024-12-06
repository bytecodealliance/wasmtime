;;! target = "aarch64"
;;! test = "compile"

(module
  (func $main (result i32)
    (local $var i32)
    (call $add (i32.const 20) (i32.const 80))
    (local.set $var (i32.const 2))
    (local.get $var)
    (i32.add))

  (func $add (param i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.add))
)

;; wasm[0]::function[0]::main:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldur    x16, [x2, #8]
;;       ldur    x16, [x16]
;;       add     x16, x16, #0x10
;;       cmp     sp, x16
;;       b.lo    #0x38
;;   1c: mov     w4, #0x14
;;       mov     w5, #0x50
;;       mov     x3, x2
;;       bl      #0x40
;;   2c: add     w2, w2, #2
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   38: .byte   0x1f, 0xc1, 0x00, 0x00
;; 
;; wasm[0]::function[1]::add:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       add     w2, w4, w5
;;       ldp     x29, x30, [sp], #0x10
;;       ret
