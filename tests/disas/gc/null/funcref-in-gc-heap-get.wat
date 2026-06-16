;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param (ref $ty)) (result funcref)
    (struct.get $ty 0 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0020                               trapz v2, user16
;; @0020                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v6 = load.i64 notrap aligned readonly can_move v5+32
;; @0020                               v4 = uextend.i64 v2
;; @0020                               v7 = iadd v6, v4
;; @0020                               v8 = iconst.i64 8
;; @0020                               v9 = iadd v7, v8  ; v8 = 8
;; @0020                               v11 = load.i32 user2 little region1 v9
;; @0020                               v10 = iconst.i32 -1
;; @0020                               v12 = call fn0(v0, v11, v10)  ; v10 = -1
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v12
;; }
