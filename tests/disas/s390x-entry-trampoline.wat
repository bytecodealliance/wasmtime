;;! target = "s390x"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       stmg    %r6, %r15, 0x30(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xe0
;;       stg     %r1, 0(%r15)
;;       std     %f8, 0xa0(%r15)
;;       std     %f9, 0xa8(%r15)
;;       std     %f10, 0xb0(%r15)
;;       std     %f11, 0xb8(%r15)
;;       std     %f12, 0xc0(%r15)
;;       std     %f13, 0xc8(%r15)
;;       std     %f14, 0xd0(%r15)
;;       std     %f15, 0xd8(%r15)
;;       lg      %r4, 8(%r2)
;;       lg      %r5, 0(%r15)
;;       stg     %r5, 0x48(%r4)
;;       lgr     %r5, %r15
;;       stg     %r5, 0x40(%r4)
;;       larl    %r5, 0xbe
;;       stg     %r5, 0x50(%r4)
;;       brasl   %r14, 0
;;       ├─╼ exception frame offset: SP = FP - 0xe0
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xbe
;;       lhi     %r2, 1
;;       ld      %f8, 0xa0(%r15)
;;       ld      %f9, 0xa8(%r15)
;;       ld      %f10, 0xb0(%r15)
;;       ld      %f11, 0xb8(%r15)
;;       ld      %f12, 0xc0(%r15)
;;       ld      %f13, 0xc8(%r15)
;;       ld      %f14, 0xd0(%r15)
;;       ld      %f15, 0xd8(%r15)
;;       lmg     %r6, %r15, 0x110(%r15)
;;       br      %r14
;;       lhi     %r2, 0
;;       ld      %f8, 0xa0(%r15)
;;       ld      %f9, 0xa8(%r15)
;;       ld      %f10, 0xb0(%r15)
;;       ld      %f11, 0xb8(%r15)
;;       ld      %f12, 0xc0(%r15)
;;       ld      %f13, 0xc8(%r15)
;;       ld      %f14, 0xd0(%r15)
;;       ld      %f15, 0xd8(%r15)
;;       lmg     %r6, %r15, 0x110(%r15)
;;       br      %r14
