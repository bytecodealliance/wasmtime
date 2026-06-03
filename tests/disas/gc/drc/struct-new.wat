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
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v50 = stack_addr.i64 ss0
;;                                     store notrap v4, v50
;; @002a                               v7 = iconst.i32 -1342177280
;; @002a                               v8 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v9 = load.i32 notrap aligned readonly can_move v8
;; @002a                               v6 = iconst.i32 40
;; @002a                               v10 = iconst.i32 8
;; @002a                               v11 = call fn0(v0, v7, v9, v6, v10), stack_map=[i32 @ ss0+0]  ; v7 = -1342177280, v6 = 40, v10 = 8
;; @002a                               v55 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v12 = load.i64 notrap aligned readonly can_move v55+32
;; @002a                               v13 = uextend.i64 v11
;; @002a                               v14 = iadd v12, v13
;; @002a                               v15 = iconst.i64 24
;; @002a                               v16 = iadd v14, v15  ; v15 = 24
;; @002a                               store user2 little region0 v2, v16
;; @002a                               v17 = iconst.i64 28
;; @002a                               v18 = iadd v14, v17  ; v17 = 28
;; @002a                               istore8 user2 little region0 v3, v18
;;                                     v49 = load.i32 notrap v50
;; @002a                               v21 = iconst.i32 1
;; @002a                               v22 = band v49, v21  ; v21 = 1
;; @002a                               v23 = iconst.i32 0
;; @002a                               v24 = icmp eq v49, v23  ; v23 = 0
;; @002a                               v25 = uextend.i32 v24
;; @002a                               v26 = bor v22, v25
;; @002a                               brif v26, block3, block2
;;
;;                                 block2:
;; @002a                               v27 = uextend.i64 v49
;; @002a                               v29 = iadd.i64 v12, v27
;; @002a                               v30 = iconst.i64 8
;; @002a                               v31 = iadd v29, v30  ; v30 = 8
;; @002a                               v32 = load.i64 user2 region0 v31
;; @002a                               v33 = iconst.i64 1
;; @002a                               v34 = iadd v32, v33  ; v33 = 1
;; @002a                               store user2 region0 v34, v31
;; @002a                               jump block3
;;
;;                                 block3:
;; @002a                               v19 = iconst.i64 32
;; @002a                               v20 = iadd.i64 v14, v19  ; v19 = 32
;; @002a                               store.i32 user2 little region0 v49, v20
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v11
;; }
