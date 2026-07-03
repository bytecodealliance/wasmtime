;;! target = "x86_64"

;; Reachable `if` head and consequent and unreachable alternative means that the
;; following block is also reachable.

(module
  (func (param i32) (result i32)
    local.get 0
    if
      nop
    else
      unreachable
    end
    i32.const 0))

;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001b                               brif v2, block2, block4
;;
;;                                 block2:
;; @001e                               jump block3
;;
;;                                 block4:
;; @001f                               trap user12
;;
;;                                 block3:
;; @0021                               v3 = iconst.i32 0
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v3  ; v3 = 0
;; }
