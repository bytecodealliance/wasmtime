;;! target = "x86_64"

;; Reachable `if` head and unreachable consequent and alternative, but with a
;; branch out of the alternative, means that the following block is reachable.

(module
  (func (param i32 i32) (result i32)
    local.get 0
    if
      unreachable
    else
      local.get 1
      br_if 0
      unreachable
    end
    i32.const 0))

;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;;                                     trapnz v2, user11
;; @001c                               jump block4
;;
;;                                 block4:
;;                                     trapz.i32 v3, user11
;; @0022                               jump block3
;;
;;                                 block3:
;; @0026                               v5 = iconst.i32 0
;; @0028                               jump block1
;;
;;                                 block1:
;; @0028                               return v5  ; v5 = 0
;; }
