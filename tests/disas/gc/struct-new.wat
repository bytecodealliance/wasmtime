;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v56 = stack_addr.i64 ss0
;;                                     store notrap v4, v56
;; @002a                               v8 = iconst.i32 -1342177280
;; @002a                               v9 = iconst.i32 0
;; @002a                               v6 = iconst.i32 40
;; @002a                               v10 = iconst.i32 8
;; @002a                               v11 = call fn0(v0, v8, v9, v6, v10), stack_map=[i32 @ ss0+0]  ; v8 = -1342177280, v9 = 0, v6 = 40, v10 = 8
;; @002a                               v54 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v12 = load.i64 notrap aligned readonly can_move v54+24
;; @002a                               v13 = uextend.i64 v11
;; @002a                               v14 = iadd v12, v13
;;                                     v53 = iconst.i64 24
;; @002a                               v15 = iadd v14, v53  ; v53 = 24
;; @002a                               store notrap aligned little v2, v15
;;                                     v52 = iconst.i64 28
;; @002a                               v16 = iadd v14, v52  ; v52 = 28
;; @002a                               istore8 notrap aligned little v3, v16
;;                                     v38 = load.i32 notrap v56
;;                                     v49 = iconst.i32 1
;; @002a                               v18 = band v38, v49  ; v49 = 1
;; @002a                               v19 = icmp eq v38, v9  ; v9 = 0
;; @002a                               v20 = uextend.i32 v19
;; @002a                               v21 = bor v18, v20
;; @002a                               brif v21, block3, block2
;;
;;                                 block2:
;; @002a                               v22 = uextend.i64 v38
;; @002a                               v24 = iadd.i64 v12, v22
;; @002a                               v25 = iconst.i64 8
;; @002a                               v26 = iadd v24, v25  ; v25 = 8
;; @002a                               v27 = load.i64 notrap aligned v26
;;                                     v43 = iconst.i64 1
;; @002a                               v28 = iadd v27, v43  ; v43 = 1
;; @002a                               store notrap aligned v28, v26
;; @002a                               jump block3
;;
;;                                 block3:
;;                                     v34 = load.i32 notrap v56
;;                                     v51 = iconst.i64 32
;; @002a                               v17 = iadd.i64 v14, v51  ; v51 = 32
;; @002a                               store notrap aligned little v34, v17
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v11
;; }
