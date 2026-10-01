;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W gc-support=y -C collector=copying"

(module
  (import "" "make" (func $make (result externref)))
  (func $nop)

  ;; The second operand is a null represented internally as i32, but the
  ;; selected live reference must appear in the call-site stack map.
  (func (export "select-live") (result externref)
    call $make
    ref.null extern
    i32.const 1
    select (result externref)
    call $nop)
)
;; wasm[0]::function[1]::nop:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x10
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x58
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   58: udf     #0xc11f
;;
;; wasm[0]::function[2]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x118
;;   8c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldur    x3, [x9, #0x48]
;;       ldur    x2, [x9, #0x38]
;;       mov     x0, x3
;;       mov     x1, x9
;;       blr     x2
;;   b4: ldur    x9, [x28, #8]
;;       mov     x1, #1
;;       mov     x2, #0
;;       cmp     w1, #0
;;       csel    x2, x0, x2, ne
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w2, [x28]
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, x9
;;       bl      #0
;;   e8: add     x28, x28, #0xc
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[12]
;;       mov     sp, x28
;;       ldur    x9, [x28, #0xc]
;;       ldur    w0, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  118: udf     #0xc11f
