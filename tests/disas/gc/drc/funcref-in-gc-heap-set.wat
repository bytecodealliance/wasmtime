;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param (ref $ty) funcref)
    (struct.set $ty 0 (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0022                               trapz v2, user16
;; @0022                               v10 = call fn0(v0, v3)
;; @0022                               v11 = ireduce.i32 v10
;; @0022                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @0022                               v4 = uextend.i64 v2
;; @0022                               v7 = iadd v6, v4
;; @0022                               v8 = iconst.i64 24
;; @0022                               v9 = iadd v7, v8  ; v8 = 24
;; @0022                               store user2 little region4 v11, v9
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return
;; }
