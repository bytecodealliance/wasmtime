;;! target = "x86_64"

(module
  (func (export "multiIf2") (param i32 i64 i64) (result i64 i64)
    (local.get 2)
    (local.get 1)
    (local.get 0)
    (if (param i64 i64) (result i64 i64)
      (then
        i64.add
        i64.const 1)
      ;; Hits the code path for an `else` after a block that does not end unreachable.
      (else
        i64.sub
        i64.const 2))))

;; function u0:0(i64 vmctx, i64, i32, i64, i64) -> i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64, v4: i64):
;; @0037                               brif v2, block2, block4(v4, v3)
;;
;;                                 block2:
;; @0039                               v9 = iadd.i64 v4, v3
;; @003a                               v10 = iconst.i64 1
;; @003c                               jump block3(v9, v10)  ; v10 = 1
;;
;;                                 block4(v11: i64, v12: i64):
;; @003d                               v13 = isub.i64 v4, v3
;; @003e                               v14 = iconst.i64 2
;; @0040                               jump block3(v13, v14)  ; v14 = 2
;;
;;                                 block3(v7: i64, v8: i64):
;; @0041                               jump block1(v7, v8)
;;
;;                                 block1(v5: i64, v6: i64):
;; @0041                               return v5, v6
;; }
