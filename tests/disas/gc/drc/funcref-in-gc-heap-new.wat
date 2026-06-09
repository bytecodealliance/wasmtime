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
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v5 = iconst.i32 -1342177280
;; @0020                               v6 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @0020                               v7 = load.i32 notrap aligned readonly can_move v6
;; @0020                               v4 = iconst.i32 32
;; @0020                               v8 = iconst.i32 8
;; @0020                               v9 = call fn0(v0, v5, v7, v4, v8)  ; v5 = -1342177280, v4 = 32, v8 = 8
;;                                     v22 = stack_addr.i64 ss0
;;                                     store notrap v9, v22
;; @0020                               v16 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v17 = ireduce.i32 v16
;; @0020                               v10 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v11 = load.i64 notrap aligned readonly can_move v10+32
;; @0020                               v12 = uextend.i64 v9
;; @0020                               v13 = iadd v11, v12
;; @0020                               v14 = iconst.i64 24
;; @0020                               v15 = iadd v13, v14  ; v14 = 24
;; @0020                               store user2 little region2 v17, v15
;;                                     v19 = load.i32 notrap v22
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v19
;; }
