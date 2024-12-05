;;! target = "s390x"
;;! test = "compile"
;;! flags = "-Wwide-arithmetic"

(module
  (func $add128 (param i64 i64 i64 i64) (result i64 i64)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    i64.add128)

  (func $sub128 (param i64 i64 i64 i64) (result i64 i64)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    i64.sub128)

  (func $signed (param i64 i64) (result i64 i64)
    local.get 0
    local.get 1
    i64.mul_wide_s)

  (func $unsigned (param i64 i64) (result i64 i64)
    local.get 0
    local.get 1
    i64.mul_wide_u)

  (func $signed_only_high (param i64 i64) (result i64)
    local.get 0
    local.get 1
    i64.mul_wide_s
    local.set 0
    drop
    local.get 0)

  (func $unsigned_only_high (param i64 i64) (result i64)
    local.get 0
    local.get 1
    i64.mul_wide_u
    local.set 0
    drop
    local.get 0)
)

;; wasm[0]::function[0]::add128:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       vlvgp   %v17, %r5, %r4
;;       vlvgp   %v18, %r7, %r6
;;       vaq     %v17, %v17, %v18
;;       vlgvg   %r3, %v17, 0
;;       vlgvg   %r2, %v17, 1
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; wasm[0]::function[1]::sub128:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       vlvgp   %v17, %r5, %r4
;;       vlvgp   %v18, %r7, %r6
;;       vsq     %v17, %v17, %v18
;;       vlgvg   %r3, %v17, 0
;;       vlgvg   %r2, %v17, 1
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; wasm[0]::function[2]::signed:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       mgrk    %r2, %r4, %r5
;;       vlvgp   %v16, %r2, %r3
;;       vlgvg   %r3, %v16, 0
;;       vlgvg   %r2, %v16, 1
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; wasm[0]::function[3]::unsigned:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       lgr     %r3, %r4
;;       mlgr    %r2, %r5
;;       vlvgp   %v16, %r2, %r3
;;       vlgvg   %r3, %v16, 0
;;       vlgvg   %r2, %v16, 1
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; wasm[0]::function[4]::signed_only_high:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       mgrk    %r2, %r4, %r5
;;       vlvgp   %v16, %r2, %r3
;;       vlgvg   %r2, %v16, 0
;;       vlgvg   %r3, %v16, 1
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; wasm[0]::function[5]::unsigned_only_high:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       lgr     %r3, %r4
;;       mlgr    %r2, %r5
;;       vlvgp   %v16, %r2, %r3
;;       vlgvg   %r2, %v16, 0
;;       vlgvg   %r3, %v16, 1
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
