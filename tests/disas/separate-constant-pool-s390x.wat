;;! target = "s390x"
;;! test = "compile"
;;! flags = "-Ccranelift-separate-constant-pool"

(module
  (func (export "one") (result v128)
    v128.const i8x16 0x11 0x22 0x33 0x44 0x55 0x66 0x77 0x88 0x99 0xaa 0xbb 0xcc 0xdd 0xee 0xf0 0x0f)
  (func (export "two_uses") (param v128) (result v128)
    (v128.xor
      (i32x4.add (local.get 0) (v128.const i32x4 1 2 3 4))
      (v128.const i32x4 1 2 3 4)))
  (func (export "two_constants") (param v128) (result v128)
    (v128.xor
      (i32x4.add (local.get 0) (v128.const i32x4 1 2 3 4))
      (v128.const i32x4 5 6 7 8)))
)
;; wasm[0]::function[0]:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       larl    %r1, 0x1000
;;       vl      %v24, 0(%r1)
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; wasm[0]::function[1]:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       larl    %r1, 0x1010
;;       vl      %v5, 0(%r1)
;;       vaf     %v6, %v24, %v5
;;       vx      %v24, %v6, %v5
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; wasm[0]::function[2]:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       larl    %r1, 0x1020
;;       vl      %v6, 0(%r1)
;;       vaf     %v6, %v24, %v6
;;       larl    %r1, 0x1030
;;       vl      %v7, 0(%r1)
;;       vx      %v24, %v6, %v7
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; .wasmtime.constants:
;; 1000: 0f f0 ee dd cc bb aa 99 88 77 66 55 44 33 22 11
;; 1010: 00 00 00 04 00 00 00 03 00 00 00 02 00 00 00 01
;; 1020: 00 00 00 04 00 00 00 03 00 00 00 02 00 00 00 01
;; 1030: 00 00 00 08 00 00 00 07 00 00 00 06 00 00 00 05
