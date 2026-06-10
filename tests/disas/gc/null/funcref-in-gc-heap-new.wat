;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param funcref) (result (ref $ty))
    (struct.new $ty (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 32 "VMContext+0x20"
;;     region2 = 2147483648 "GcHeap"
;;     region3 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:23 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v8 = load.i64 notrap aligned readonly region1 v0+32
;; @0020                               v9 = load.i32 user2 region2 v8
;;                                     v40 = iconst.i32 7
;; @0020                               v12 = uadd_overflow_trap v9, v40, user18  ; v40 = 7
;;                                     v46 = iconst.i32 -8
;; @0020                               v14 = band v12, v46  ; v46 = -8
;; @0020                               v4 = iconst.i32 16
;; @0020                               v15 = uadd_overflow_trap v14, v4, user18  ; v4 = 16
;; @0020                               v17 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v18 = load.i64 notrap aligned v17+40
;; @0020                               v16 = uextend.i64 v15
;; @0020                               v19 = icmp ule v16, v18
;; @0020                               brif v19, block2, block3
;;
;;                                 block2:
;;                                     v47 = iconst.i32 -1342177264
;; @0020                               v23 = load.i64 notrap aligned readonly can_move v17+32
;;                                     v53 = band.i32 v12, v46  ; v46 = -8
;;                                     v54 = uextend.i64 v53
;; @0020                               v25 = iadd v23, v54
;; @0020                               store user2 region2 v47, v25  ; v47 = -1342177264
;; @0020                               v28 = load.i64 notrap aligned readonly can_move region3 v0+40
;; @0020                               v29 = load.i32 notrap aligned readonly can_move v28
;; @0020                               store user2 region2 v29, v25+4
;; @0020                               store.i32 user2 region2 v15, v8
;; @0020                               v32 = call fn1(v0, v2)
;; @0020                               v33 = ireduce.i32 v32
;; @0020                               v30 = iconst.i64 8
;; @0020                               v31 = iadd v25, v30  ; v30 = 8
;; @0020                               store user2 little region2 v33, v31
;; @0023                               jump block1
;;
;;                                 block3 cold:
;; @0020                               v20 = isub.i64 v16, v18
;; @0020                               v21 = call fn0(v0, v20)
;; @0020                               jump block2
;;
;;                                 block1:
;;                                     v55 = band.i32 v12, v46  ; v46 = -8
;; @0023                               return v55
;; }
