;;! target = "x86_64"

(module
  ;; Iterative factorial without locals.
  (func $pick0 (param i64) (result i64 i64)
    (local.get 0) (local.get 0)
  )
  (func $pick1 (param i64 i64) (result i64 i64 i64)
    (local.get 0) (local.get 1) (local.get 0)
  )
  (func (export "fac-ssa") (param i64) (result i64)
    (i64.const 1) (local.get 0)
    (loop $l (param i64 i64) (result i64)
      (call $pick1) (call $pick1) (i64.mul)
      (call $pick1) (i64.const 1) (i64.sub)
      (call $pick0) (i64.const 0) (i64.gt_u)
      (br_if $l)
      (drop) (return)
    )
  )
)

;; function u0:0(i64 vmctx, i64, i64) -> i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0040                               jump block1(v2, v2)
;;
;;                                 block1(v3: i64, v4: i64):
;; @0040                               return v3, v4
;; }
;;
;; function u0:1(i64 vmctx, i64, i64, i64) -> i64, i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64):
;; @0049                               jump block1(v2, v3, v2)
;;
;;                                 block1(v4: i64, v5: i64, v6: i64):
;; @0049                               return v4, v5, v6
;; }
;;
;; function u0:2(i64 vmctx, i64, i64) -> i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     sig0 = (i64 vmctx, i64, i64, i64) -> i64, i64, i64 tail
;;     sig1 = (i64 vmctx, i64, i64) -> i64, i64 tail
;;     fn0 = colocated u0:1 sig0
;;     fn1 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @004c                               v4 = iconst.i64 1
;; @0050                               jump block2(v4, v2)  ; v4 = 1
;;
;;                                 block2(v5: i64, v6: i64):
;; @0052                               v8, v9, v10 = call fn0(v0, v0, v5, v6)
;; @0054                               v11, v12, v13 = call fn0(v0, v0, v9, v10)
;; @0056                               v14 = imul v12, v13
;; @0057                               v15, v16, v17 = call fn0(v0, v0, v11, v14)
;; @0059                               v18 = iconst.i64 1
;; @005b                               v19 = isub v17, v18  ; v18 = 1
;; @005c                               v20, v21 = call fn1(v0, v0, v19)
;; @005e                               v22 = iconst.i64 0
;; @0060                               v23 = icmp ugt v21, v22  ; v22 = 0
;; @0060                               v24 = uextend.i32 v23
;; @0061                               brif v24, block2(v16, v20), block4
;;
;;                                 block4:
;; @0064                               return v16
;; }
