;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param f32 i32 anyref) (result (ref $ty))
    (struct.new $ty (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, f32, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
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
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v39 = stack_addr.i64 ss0
;;                                     store notrap v4, v39
;; @002a                               v9 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @002a                               v10 = load.i32 notrap aligned region3 v9
;;                                     v46 = iconst.i32 7
;; @002a                               v13 = uadd_overflow_trap v10, v46, user18  ; v46 = 7
;;                                     v52 = iconst.i32 -8
;; @002a                               v15 = band v13, v52  ; v52 = -8
;; @002a                               v5 = iconst.i32 24
;; @002a                               v16 = uadd_overflow_trap v15, v5, user18  ; v5 = 24
;; @002a                               v18 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002a                               v19 = load.i64 notrap aligned region4 v18+40
;; @002a                               v17 = uextend.i64 v16
;; @002a                               v20 = icmp ule v17, v19
;; @002a                               brif v20, block2, block3
;;
;;                                 block2:
;;                                     v53 = iconst.i32 -1342177256
;; @002a                               v24 = load.i64 notrap aligned readonly can_move region5 v18+32
;;                                     v59 = band.i32 v13, v52  ; v52 = -8
;;                                     v60 = uextend.i64 v59
;; @002a                               v26 = iadd v24, v60
;; @002a                               store user2 region7 v53, v26  ; v53 = -1342177256
;; @002a                               v29 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @002a                               v30 = load.i32 notrap aligned readonly can_move v29
;; @002a                               store user2 region7 v30, v26+4
;; @002a                               store.i32 notrap aligned region3 v16, v9
;; @002a                               v31 = iconst.i64 8
;; @002a                               v32 = iadd v26, v31  ; v31 = 8
;; @002a                               store.f32 user2 little region7 v2, v32
;; @002a                               v33 = iconst.i64 12
;; @002a                               v34 = iadd v26, v33  ; v33 = 12
;; @002a                               istore8.i32 user2 little region7 v3, v34
;;                                     v38 = load.i32 notrap v39
;; @002a                               v35 = iconst.i64 16
;; @002a                               v36 = iadd v26, v35  ; v35 = 16
;; @002a                               store user2 little region7 v38, v36
;; @002d                               jump block1
;;
;;                                 block3 cold:
;; @002a                               v21 = isub.i64 v17, v19
;; @002a                               v22 = call fn0(v0, v21), stack_map=[i32 @ ss0+0]
;; @002a                               jump block2
;;
;;                                 block1:
;;                                     v61 = band.i32 v13, v52  ; v52 = -8
;; @002d                               return v61
;; }
