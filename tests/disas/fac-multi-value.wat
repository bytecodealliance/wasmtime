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
;;     region1 = 67108888 "VMStoreContext+0x18"
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
;;     region1 = 67108888 "VMStoreContext+0x18"
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
;;     region1 = 67108888 "VMStoreContext+0x18"
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
;; @004c                               v3 = iconst.i64 1
;; @0050                               jump block2(v3, v2)  ; v3 = 1
;;
;;                                 block2(v4: i64, v5: i64):
;; @0052                               v6, v7, v8 = call fn0(v0, v0, v4, v5)
;; @0054                               v9, v10, v11 = call fn0(v0, v0, v7, v8)
;; @0056                               v12 = imul v10, v11
;; @0057                               v13, v14, v15 = call fn0(v0, v0, v9, v12)
;; @0059                               v16 = iconst.i64 1
;; @005b                               v17 = isub v15, v16  ; v16 = 1
;; @005c                               v18, v19 = call fn1(v0, v0, v17)
;; @005e                               v20 = iconst.i64 0
;; @0060                               v21 = icmp ugt v19, v20  ; v20 = 0
;; @0060                               v22 = uextend.i32 v21
;; @0061                               brif v22, block2(v14, v18), block4
;;
;;                                 block4:
;; @0064                               return v14
;; }
