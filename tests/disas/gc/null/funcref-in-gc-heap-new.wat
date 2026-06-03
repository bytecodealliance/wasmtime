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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+40
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:23 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v8 = load.i64 notrap aligned readonly region0 v0+32
;; @0020                               v9 = load.i32 user2 region1 v8
;;                                     v42 = iconst.i32 7
;; @0020                               v12 = uadd_overflow_trap v9, v42, user18  ; v42 = 7
;;                                     v48 = iconst.i32 -8
;; @0020                               v14 = band v12, v48  ; v48 = -8
;; @0020                               v4 = iconst.i32 16
;; @0020                               v15 = uadd_overflow_trap v14, v4, user18  ; v4 = 16
;; @0020                               v34 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v17 = load.i64 notrap aligned v34+40
;; @0020                               v16 = uextend.i64 v15
;; @0020                               v18 = icmp ule v16, v17
;; @0020                               brif v18, block2, block3
;;
;;                                 block2:
;;                                     v49 = iconst.i32 -1342177264
;; @0020                               v21 = load.i64 notrap aligned readonly can_move v34+32
;;                                     v55 = band.i32 v12, v48  ; v48 = -8
;;                                     v56 = uextend.i64 v55
;; @0020                               v23 = iadd v21, v56
;; @0020                               store user2 region1 v49, v23  ; v49 = -1342177264
;; @0020                               v26 = load.i64 notrap aligned readonly can_move v0+40
;; @0020                               v27 = load.i32 notrap aligned readonly can_move v26
;; @0020                               store user2 region1 v27, v23+4
;; @0020                               store.i32 user2 region1 v15, v8
;; @0020                               v30 = call fn1(v0, v2)
;; @0020                               v31 = ireduce.i32 v30
;; @0020                               v28 = iconst.i64 8
;; @0020                               v29 = iadd v23, v28  ; v28 = 8
;; @0020                               store user2 little region1 v31, v29
;; @0023                               jump block1
;;
;;                                 block3 cold:
;; @0020                               v19 = isub.i64 v16, v17
;; @0020                               v20 = call fn0(v0, v19)
;; @0020                               jump block2
;;
;;                                 block1:
;;                                     v57 = band.i32 v12, v48  ; v48 = -8
;; @0023                               return v57
;; }
