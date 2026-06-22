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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0040                               jump block1
;;
;;                                 block1:
;; @0040                               return v2, v2
;; }
;;
;; function u0:1(i64 vmctx, i64, i64, i64) -> i64, i64, i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64):
;; @0049                               jump block1
;;
;;                                 block1:
;; @0049                               return v2, v3, v2
;; }
;;
;; function u0:2(i64 vmctx, i64, i64) -> i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
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
;; @0052                               v7, v8, v9 = call fn0(v0, v0, v5, v6)
;; @0054                               v10, v11, v12 = call fn0(v0, v0, v8, v9)
;; @0056                               v13 = imul v11, v12
;; @0057                               v14, v15, v16 = call fn0(v0, v0, v10, v13)
;; @0059                               v17 = iconst.i64 1
;; @005b                               v18 = isub v16, v17  ; v17 = 1
;; @005c                               v19, v20 = call fn1(v0, v0, v18)
;; @005e                               v21 = iconst.i64 0
;; @0060                               v22 = icmp ugt v20, v21  ; v21 = 0
;; @0060                               v23 = uextend.i32 v22
;; @0061                               brif v23, block2(v15, v19), block4
;;
;;                                 block4:
;; @0064                               return v15
;; }
