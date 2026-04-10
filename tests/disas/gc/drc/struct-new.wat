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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v58 = stack_addr.i64 ss0
;;                                     store notrap v4, v58
;; @002a                               v8 = iconst.i32 -1342177280
;; @002a                               v10 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v11 = load.i32 notrap aligned readonly can_move v10
;; @002a                               v6 = iconst.i32 40
;; @002a                               v12 = iconst.i32 8
;; @002a                               v13 = call fn0(v0, v8, v11, v6, v12), stack_map=[i32 @ ss0+0]  ; v8 = -1342177280, v6 = 40, v12 = 8
;; @002a                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v14 = load.i64 notrap aligned readonly can_move v56+32
;; @002a                               v15 = uextend.i64 v13
;; @002a                               v16 = iadd v14, v15
;;                                     v55 = iconst.i64 24
;; @002a                               v17 = iadd v16, v55  ; v55 = 24
;; @002a                               store notrap aligned little v2, v17
;;                                     v54 = iconst.i64 28
;; @002a                               v18 = iadd v16, v54  ; v54 = 28
;; @002a                               istore8 notrap aligned little v3, v18
;;                                     v40 = load.i32 notrap v58
;;                                     v51 = iconst.i32 1
;; @002a                               v20 = band v40, v51  ; v51 = 1
;;                                     v49 = iconst.i32 0
;; @002a                               v21 = icmp eq v40, v49  ; v49 = 0
;; @002a                               v22 = uextend.i32 v21
;; @002a                               v23 = bor v20, v22
;; @002a                               brif v23, block3, block2
;;
;;                                 block2:
;; @002a                               v24 = uextend.i64 v40
;; @002a                               v26 = iadd.i64 v14, v24
;; @002a                               v27 = iconst.i64 8
;; @002a                               v28 = iadd v26, v27  ; v27 = 8
;; @002a                               v29 = load.i64 notrap aligned v28
;;                                     v45 = iconst.i64 1
;; @002a                               v30 = iadd v29, v45  ; v45 = 1
;; @002a                               store notrap aligned v30, v28
;; @002a                               jump block3
;;
;;                                 block3:
;;                                     v36 = load.i32 notrap v58
;;                                     v53 = iconst.i64 32
;; @002a                               v19 = iadd.i64 v16, v53  ; v53 = 32
;; @002a                               store notrap aligned little v36, v19
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v13
;; }
