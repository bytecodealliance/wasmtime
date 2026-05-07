;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
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
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v8 = load.i64 notrap aligned readonly can_move v0+32
;; @0021                               v9 = load.i32 notrap aligned can_move v8
;; @0021                               v16 = uextend.i64 v9
;;                                     v50 = iconst.i64 32
;; @0021                               v17 = iadd v16, v50  ; v50 = 32
;; @0021                               v10 = load.i32 notrap aligned readonly can_move v8+4
;; @0021                               v18 = uextend.i64 v10
;; @0021                               v19 = icmp ule v17, v18
;; @0021                               brif v19, block2, block3
;;
;;                                 block2:
;;                                     v66 = iconst.i32 32
;;                                     v64 = iadd.i32 v9, v66  ; v66 = 32
;; @0021                               store notrap aligned vmctx v64, v8
;;                                     v67 = iconst.i32 -1342177280
;;                                     v68 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v69 = load.i64 notrap aligned readonly can_move v68+32
;; @0021                               v33 = iadd v69, v16
;; @0021                               store notrap aligned v67, v33  ; v67 = -1342177280
;;                                     v70 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v71 = load.i32 notrap aligned readonly can_move v70
;; @0021                               store notrap aligned v71, v33+4
;;                                     v72 = iconst.i64 32
;; @0021                               istore32 notrap aligned v72, v33+8  ; v72 = 32
;; @0021                               jump block4(v9, v33)
;;
;;                                 block3 cold:
;; @0021                               v21 = iconst.i32 -1342177280
;; @0021                               v23 = load.i64 notrap aligned readonly can_move v0+40
;; @0021                               v24 = load.i32 notrap aligned readonly can_move v23
;; @0021                               v6 = iconst.i32 32
;; @0021                               v25 = iconst.i32 16
;; @0021                               v26 = call fn0(v0, v21, v24, v6, v25)  ; v21 = -1342177280, v6 = 32, v25 = 16
;; @0021                               v46 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v27 = load.i64 notrap aligned readonly can_move v46+32
;; @0021                               v28 = uextend.i64 v26
;; @0021                               v29 = iadd v27, v28
;; @0021                               jump block4(v26, v29)
;;
;;                                 block4(v38: i32, v39: i64):
;; @0021                               v3 = f32const 0.0
;;                                     v45 = iconst.i64 16
;; @0021                               v40 = iadd v39, v45  ; v45 = 16
;; @0021                               store user2 little v3, v40  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;;                                     v44 = iconst.i64 20
;; @0021                               v41 = iadd v39, v44  ; v44 = 20
;; @0021                               istore8 user2 little v4, v41  ; v4 = 0
;;                                     v43 = iconst.i64 24
;; @0021                               v42 = iadd v39, v43  ; v43 = 24
;; @0021                               store user2 little v4, v42  ; v4 = 0
;; @0024                               jump block1(v38)
;;
;;                                 block1(v2: i32):
;; @0024                               return v2
;; }
