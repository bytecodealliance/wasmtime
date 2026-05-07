;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
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
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v52 = stack_addr.i64 ss0
;;                                     store notrap v4, v52
;; @002a                               v8 = load.i64 notrap aligned readonly can_move v0+32
;; @002a                               v9 = load.i32 notrap aligned can_move v8
;; @002a                               v16 = uextend.i64 v9
;;                                     v53 = iconst.i64 32
;; @002a                               v17 = iadd v16, v53  ; v53 = 32
;; @002a                               v10 = load.i32 notrap aligned readonly can_move v8+4
;; @002a                               v18 = uextend.i64 v10
;; @002a                               v19 = icmp ule v17, v18
;; @002a                               brif v19, block2, block3
;;
;;                                 block2:
;;                                     v69 = iconst.i32 32
;;                                     v67 = iadd.i32 v9, v69  ; v69 = 32
;; @002a                               store notrap aligned vmctx v67, v8
;;                                     v70 = iconst.i32 -1342177280
;;                                     v71 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v72 = load.i64 notrap aligned readonly can_move v71+32
;; @002a                               v33 = iadd v72, v16
;; @002a                               store notrap aligned v70, v33  ; v70 = -1342177280
;;                                     v73 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v74 = load.i32 notrap aligned readonly can_move v73
;; @002a                               store notrap aligned v74, v33+4
;;                                     v75 = iconst.i64 32
;; @002a                               istore32 notrap aligned v75, v33+8  ; v75 = 32
;; @002a                               jump block4(v9, v33)
;;
;;                                 block3 cold:
;; @002a                               v21 = iconst.i32 -1342177280
;; @002a                               v23 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v24 = load.i32 notrap aligned readonly can_move v23
;; @002a                               v6 = iconst.i32 32
;; @002a                               v25 = iconst.i32 16
;; @002a                               v26 = call fn0(v0, v21, v24, v6, v25), stack_map=[i32 @ ss0+0]  ; v21 = -1342177280, v6 = 32, v25 = 16
;; @002a                               v48 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v27 = load.i64 notrap aligned readonly can_move v48+32
;; @002a                               v28 = uextend.i64 v26
;; @002a                               v29 = iadd v27, v28
;; @002a                               jump block4(v26, v29)
;;
;;                                 block4(v38: i32, v39: i64):
;;                                     v47 = iconst.i64 16
;; @002a                               v40 = iadd v39, v47  ; v47 = 16
;; @002a                               store.f32 user2 little v2, v40
;;                                     v46 = iconst.i64 20
;; @002a                               v41 = iadd v39, v46  ; v46 = 20
;; @002a                               istore8.i32 user2 little v3, v41
;;                                     v43 = load.i32 notrap v52
;;                                     v45 = iconst.i64 24
;; @002a                               v42 = iadd v39, v45  ; v45 = 24
;; @002a                               store user2 little v43, v42
;; @002d                               jump block1(v38)
;;
;;                                 block1(v5: i32):
;; @002d                               return v5
;; }
