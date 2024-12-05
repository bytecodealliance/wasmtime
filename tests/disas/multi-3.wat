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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64, v4: i64):
;; @0036                               brif v2, block2, block4(v4, v3)
;;
;;                                 block2:
;; @0038                               return v4, v3
;;
;;                                 block4(v9: i64, v10: i64):
;; @003c                               v11 = iconst.i64 0
;; @003e                               v12 = iconst.i64 0
;; @0040                               jump block3(v11, v12)  ; v11 = 0, v12 = 0
;;
;;                                 block3(v7: i64, v8: i64):
;; @0041                               jump block1(v7, v8)
;;
;;                                 block1(v5: i64, v6: i64):
;; @0041                               return v5, v6
;; }
