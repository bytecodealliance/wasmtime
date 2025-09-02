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
;;       l       %r4, 0(%r2)
;;       clfi    %r4, 0x65726f63
;;       jgne    0x70
;;       lg      %r4, 8(%r2)
;;       lg      %r5, 0(%r15)
;;       stg     %r5, 0x38(%r4)
;;       brasl   %r14, 0
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
