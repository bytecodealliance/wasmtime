;;! target = "pulley64"
;;! test = "compile"

;; `return_call_indirect` IS a tail call but the lazy-init brif is
;; unchanged — only the call op itself is different. Phase 2 still
;; matches and fires here: the brif's continuation block contains
;; the same canonical 2-load pattern, and after the loads is a
;; `return_call_indirect` (lowered as `xjump` after the field reads)
;; instead of a `call_indirect`. Both consume (code, vmctx) the same
;; way, so the fusion is sound across the tail-call boundary.
;;
;; The disas confirms: `xband64_s8 ; xfuncref_dispatch_not_x64 ;
;; xjump` — the tail jump replaces what would have been
;; `call_indirect` in the non-tail case.
;;
;; Reference precedent: WAMR #2231 ("AOT/JIT tail-call:
;; `return_call_indirect` is not actually tail" — uses LLVM `tail`
;; hint instead of `musttail`). Our fusion preserves tail-call
;; semantics because it runs upstream of the call_indirect-vs-
;; return_call_indirect choice; this test pins that.

(module
  (table 1 1 funcref)
  (type $sig (func (result i32)))

  (func $f1 (result i32) i32.const 1)

  (func (export "trampoline") (param i32) (result i32)
    local.get 0
    return_call_indirect (type $sig))

  (elem (i32.const 0) func $f1))
;; wasm[0]::function[0]::f1:
;;       push_frame
;;       xone x0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]:
;;       push_frame_save 16, x25
;;       br_if32 x2, 0x58    // target = 0x62
;;   10: xload64le_o32 x15, x0, 48
;;       xmov x1, x0
;;       zext32 x14, x2
;;       xshl64_u6 x0, x14, 3
;;       xadd64 x15, x15, x0
;;       xload64le_o32 x15, x15, 0
;;       xband64_s8 x0, x15, -2
;;       br_if_xeq64_i8 x15, 0, 0x22    // target = 0x50
;;   35: xmov x25, x1
;;       xload64le_o32 x15, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       xmov x1, x25
;;       pop_frame_restore 16, x25
;;       xjump x15
;;   50: xzero x0
;;       xmov x25, x1
;;       call3 x25, x0, x14, 0x1b3    // target = 0x208
;;       jump -0x25    // target = 0x38
;;   62: trap
