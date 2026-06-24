;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 2147483648 "GcHeap"
;;     region5 = 268435496 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v52 = stack_addr.i64 ss0
;;                                     store notrap v4, v52
;; @002a                               v6 = iconst.i32 -1342177280
;; @002a                               v7 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @002a                               v8 = load.i32 notrap aligned readonly can_move v7
;; @002a                               v5 = iconst.i32 40
;; @002a                               v9 = iconst.i32 8
;; @002a                               v10 = call fn0(v0, v6, v8, v5, v9), stack_map=[i32 @ ss0+0]  ; v6 = -1342177280, v5 = 40, v9 = 8
;; @002a                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002a                               v12 = load.i64 notrap aligned readonly can_move region3 v11+32
;; @002a                               v13 = uextend.i64 v10
;; @002a                               v14 = iadd v12, v13
;; @002a                               v15 = iconst.i64 24
;; @002a                               v16 = iadd v14, v15  ; v15 = 24
;; @002a                               store user2 little region4 v2, v16
;; @002a                               v17 = iconst.i64 28
;; @002a                               v18 = iadd v14, v17  ; v17 = 28
;; @002a                               istore8 user2 little region4 v3, v18
;;                                     v51 = load.i32 notrap v52
;; @002a                               v21 = iconst.i32 1
;; @002a                               v22 = band v51, v21  ; v21 = 1
;; @002a                               v23 = iconst.i32 0
;; @002a                               v24 = icmp eq v51, v23  ; v23 = 0
;; @002a                               v25 = uextend.i32 v24
;; @002a                               v26 = bor v22, v25
;; @002a                               brif v26, block3, block2
;;
;;                                 block2:
;; @002a                               v27 = uextend.i64 v51
;; @002a                               v30 = iadd.i64 v12, v27
;; @002a                               v31 = iconst.i64 8
;; @002a                               v32 = iadd v30, v31  ; v31 = 8
;; @002a                               v33 = load.i64 user2 region4 v32
;; @002a                               v34 = iconst.i64 1
;; @002a                               v35 = iadd v33, v34  ; v34 = 1
;; @002a                               store user2 region4 v35, v32
;; @002a                               jump block3
;;
;;                                 block3:
;; @002a                               v19 = iconst.i64 32
;; @002a                               v20 = iadd.i64 v14, v19  ; v19 = 32
;; @002a                               store.i32 user2 little region4 v51, v20
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v10
;; }
