;;! target = "s390x-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Osignals-based-traps=n"

;; Without signals-based traps, the stack-limit check is done at the very
;; start of each function, before frame setup, and on overflow tail-calls the
;; `stack_overflow` builtin's trampoline.

(module
  (func $callee)
  (func (param i64 i64) (result i64)
    call $callee
    (i64.add (local.get 0) (local.get 1))
  )
)
;; wasm[0]::function[0]::callee:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgr    %r15, %r1
;;       jgle    0x20c
;;       stmg    %r14, %r15, 0x70(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       lmg     %r14, %r15, 0x110(%r15)
;;       br      %r14
;;
;; wasm[0]::function[1]:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0x10(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgr    %r15, %r1
;;       jgle    0x20c
;;       stmg    %r8, %r15, 0x40(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       lgr     %r3, %r2
;;       lgr     %r8, %r5
;;       lgr     %r9, %r4
;;       brasl   %r14, 0
;;       lgr     %r4, %r9
;;       lgr     %r5, %r8
;;       agrk    %r2, %r4, %r5
;;       lmg     %r8, %r15, 0xe0(%r15)
;;       br      %r14
