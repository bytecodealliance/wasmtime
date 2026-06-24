;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param (ref $ty)) (result funcref)
    (struct.get $ty 0 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0020                               trapz v2, user16
;; @0020                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @0020                               v3 = uextend.i64 v2
;; @0020                               v6 = iadd v5, v3
;; @0020                               v7 = iconst.i64 24
;; @0020                               v8 = iadd v6, v7  ; v7 = 24
;; @0020                               v10 = load.i32 user2 little region4 v8
;; @0020                               v9 = iconst.i32 -1
;; @0020                               v11 = call fn0(v0, v10, v9)  ; v9 = -1
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v11
;; }
