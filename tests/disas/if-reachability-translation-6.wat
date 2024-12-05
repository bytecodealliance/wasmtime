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
;; @001c                               brif v2, block2, block4
;;
;;                                 block2:
;; @001e                               trap user11
;;
;;                                 block4:
;; @0022                               brif.i32 v3, block3, block5
;;
;;                                 block5:
;; @0024                               trap user11
;;
;;                                 block3:
;; @0026                               v5 = iconst.i32 0
;; @0028                               jump block1(v5)  ; v5 = 0
;;
;;                                 block1(v4: i32):
;; @0028                               return v4
;; }
