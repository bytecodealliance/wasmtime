;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (func (param anyref) (result i32)
    (ref.test (ref any) (local.get 0))
  )
  (func (param (ref any)) (result i32)
    (ref.test (ref any) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0025                               jump block1
;;
;;                                 block1:
;; @0022                               v7 = iconst.i32 1
;; @0022                               v3 = iconst.i32 0
;;                                     v13 = select v2, v7, v3  ; v7 = 1, v3 = 0
;; @0025                               return v13
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002d                               jump block1
;;
;;                                 block1:
;; @002a                               v5 = iconst.i32 1
;; @002d                               return v5  ; v5 = 1
;; }
