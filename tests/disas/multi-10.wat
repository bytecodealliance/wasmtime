;;! target = "x86_64"

(module
  (func (export "f") (param i64 i32) (result i64 i64)
    (local.get 0)
    (local.get 1)
    ;; If with else. Fewer params than results.
    (if (param i64) (result i64 i64)
      (then
        (i64.const -1))
      (else
        (i64.const -2)))))

;; function u0:0(i64 vmctx, i64, i64, i32) -> i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @002c                               brif v3, block2, block4(v2)
;;
;;                                 block2:
;; @002e                               v9 = iconst.i64 -1
;; @0030                               jump block3(v2, v9)  ; v9 = -1
;;
;;                                 block4(v8: i64):
;; @0031                               v10 = iconst.i64 -2
;; @0033                               jump block3(v2, v10)  ; v10 = -2
;;
;;                                 block3(v6: i64, v7: i64):
;; @0034                               jump block1(v6, v7)
;;
;;                                 block1(v4: i64, v5: i64):
;; @0034                               return v4, v5
;; }
