;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param funcref) (result (ref $ty))
    (struct.new $ty (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v4 = iconst.i32 -1342177280
;; @0020                               v5 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0020                               v6 = load.i32 notrap aligned readonly can_move v5
;; @0020                               v3 = iconst.i32 32
;; @0020                               v7 = iconst.i32 8
;; @0020                               v8 = call fn0(v0, v4, v6, v3, v7)  ; v4 = -1342177280, v3 = 32, v7 = 8
;;                                     v21 = stack_addr.i64 ss0
;;                                     store notrap v8, v21
;; @0020                               v15 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v16 = ireduce.i32 v15
;; @0020                               v9 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v10 = load.i64 notrap aligned readonly can_move region3 v9+32
;; @0020                               v11 = uextend.i64 v8
;; @0020                               v12 = iadd v10, v11
;; @0020                               v13 = iconst.i64 24
;; @0020                               v14 = iadd v12, v13  ; v13 = 24
;; @0020                               store user2 little region4 v16, v14
;; @0023                               jump block1
;;
;;                                 block1:
;;                                     v18 = load.i32 notrap v21
;; @0023                               return v18
;; }
