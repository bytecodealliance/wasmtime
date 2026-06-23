;;! target = "x86_64"

(module
  (func (export "multiIf") (param i32 i64 i64) (result i64 i64)
    (local.get 2)
    (local.get 1)
    (local.get 0)
    (if (param i64 i64) (result i64 i64)
      (then return)
      ;; Hits the code path for an `else` after a block that ends unreachable.
      (else
        (drop)
        (drop)
        (i64.const 0)
        (i64.const 0)))))

;; function u0:0(i64 vmctx, i64, i32, i64, i64) -> i64, i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64, v4: i64):
;; @0036                               brif v2, block2, block4
;;
;;                                 block2:
;; @0038                               return v4, v3
;;
;;                                 block4:
;; @003c                               v5 = iconst.i64 0
;; @003e                               v6 = iconst.i64 0
;; @0040                               jump block3
;;
;;                                 block3:
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return v5, v6  ; v5 = 0, v6 = 0
;; }
