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
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v51 = stack_addr.i64 ss0
;;                                     store notrap v4, v51
;; @002a                               v7 = iconst.i32 -1342177280
;; @002a                               v8 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @002a                               v9 = load.i32 notrap aligned readonly can_move v8
;; @002a                               v6 = iconst.i32 40
;; @002a                               v10 = iconst.i32 8
;; @002a                               v11 = call fn0(v0, v7, v9, v6, v10), stack_map=[i32 @ ss0+0]  ; v7 = -1342177280, v6 = 40, v10 = 8
;; @002a                               v12 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002a                               v13 = load.i64 notrap aligned readonly can_move v12+32
;; @002a                               v14 = uextend.i64 v11
;; @002a                               v15 = iadd v13, v14
;; @002a                               v16 = iconst.i64 24
;; @002a                               v17 = iadd v15, v16  ; v16 = 24
;; @002a                               store user2 little region2 v2, v17
;; @002a                               v18 = iconst.i64 28
;; @002a                               v19 = iadd v15, v18  ; v18 = 28
;; @002a                               istore8 user2 little region2 v3, v19
;;                                     v50 = load.i32 notrap v51
;; @002a                               v22 = iconst.i32 1
;; @002a                               v23 = band v50, v22  ; v22 = 1
;; @002a                               v24 = iconst.i32 0
;; @002a                               v25 = icmp eq v50, v24  ; v24 = 0
;; @002a                               v26 = uextend.i32 v25
;; @002a                               v27 = bor v23, v26
;; @002a                               brif v27, block3, block2
;;
;;                                 block2:
;; @002a                               v28 = uextend.i64 v50
;; @002a                               v30 = iadd.i64 v13, v28
;; @002a                               v31 = iconst.i64 8
;; @002a                               v32 = iadd v30, v31  ; v31 = 8
;; @002a                               v33 = load.i64 user2 region2 v32
;; @002a                               v34 = iconst.i64 1
;; @002a                               v35 = iadd v33, v34  ; v34 = 1
;; @002a                               store user2 region2 v35, v32
;; @002a                               jump block3
;;
;;                                 block3:
;; @002a                               v20 = iconst.i64 32
;; @002a                               v21 = iadd.i64 v15, v20  ; v20 = 32
;; @002a                               store.i32 user2 little region2 v50, v21
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v11
;; }
