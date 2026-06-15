;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (result (ref $ty))
    (struct.new_default $ty)
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 3758096384 "VMNullHeapData+0x0"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 268435488 "VMStoreContext+0x20"
;;     region6 = 40 "VMContext+0x28"
;;     region7 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v9 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0021                               v10 = load.i32 notrap aligned region3 v9
;;                                     v42 = iconst.i32 7
;; @0021                               v13 = uadd_overflow_trap v10, v42, user18  ; v42 = 7
;;                                     v48 = iconst.i32 -8
;; @0021                               v15 = band v13, v48  ; v48 = -8
;; @0021                               v5 = iconst.i32 24
;; @0021                               v16 = uadd_overflow_trap v15, v5, user18  ; v5 = 24
;; @0021                               v18 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0021                               v19 = load.i64 notrap aligned region4 v18+40
;; @0021                               v17 = uextend.i64 v16
;; @0021                               v20 = icmp ule v17, v19
;; @0021                               brif v20, block2, block3
;;
;;                                 block2:
;;                                     v49 = iconst.i32 -1342177256
;; @0021                               v24 = load.i64 notrap aligned readonly can_move region5 v18+32
;;                                     v55 = band.i32 v13, v48  ; v48 = -8
;;                                     v56 = uextend.i64 v55
;; @0021                               v26 = iadd v24, v56
;; @0021                               store user2 region7 v49, v26  ; v49 = -1342177256
;; @0021                               v29 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0021                               v30 = load.i32 notrap aligned readonly can_move v29
;; @0021                               store user2 region7 v30, v26+4
;; @0021                               store.i32 notrap aligned region3 v16, v9
;; @0021                               v2 = f32const 0.0
;; @0021                               v31 = iconst.i64 8
;; @0021                               v32 = iadd v26, v31  ; v31 = 8
;; @0021                               store user2 little region7 v2, v32  ; v2 = 0.0
;; @0021                               v3 = iconst.i32 0
;; @0021                               v33 = iconst.i64 12
;; @0021                               v34 = iadd v26, v33  ; v33 = 12
;; @0021                               istore8 user2 little region7 v3, v34  ; v3 = 0
;; @0021                               v35 = iconst.i64 16
;; @0021                               v36 = iadd v26, v35  ; v35 = 16
;; @0021                               store user2 little region7 v3, v36  ; v3 = 0
;; @0024                               jump block1
;;
;;                                 block3 cold:
;; @0021                               v21 = isub.i64 v17, v19
;; @0021                               v22 = call fn0(v0, v21)
;; @0021                               jump block2
;;
;;                                 block1:
;;                                     v57 = band.i32 v13, v48  ; v48 = -8
;; @0024                               return v57
;; }
