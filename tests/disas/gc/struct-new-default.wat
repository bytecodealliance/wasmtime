;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))
                    (field (mut v128))))

  (func (result (ref $ty))
    (struct.new_default $ty)
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 2 "vmctx"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               v9 = load.i64 notrap aligned readonly can_move v0+32
;; @0023                               v10 = load.i32 notrap aligned v9
;; @0023                               v11 = load.i32 notrap aligned v9+4
;; @0023                               v17 = uextend.i64 v10
;;                                     v53 = iconst.i64 48
;; @0023                               v18 = iadd v17, v53  ; v53 = 48
;; @0023                               v19 = uextend.i64 v11
;; @0023                               v20 = icmp ule v18, v19
;; @0023                               brif v20, block2, block3
;;
;;                                 block2:
;;                                     v69 = iconst.i32 48
;;                                     v67 = iadd.i32 v10, v69  ; v69 = 48
;; @0023                               store notrap aligned region0 v67, v9
;;                                     v70 = iconst.i32 -1342177246
;;                                     v71 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v72 = load.i64 notrap aligned readonly can_move v71+32
;; @0023                               v34 = iadd v72, v17
;; @0023                               store notrap aligned v70, v34  ; v70 = -1342177246
;;                                     v73 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v74 = load.i32 notrap aligned readonly can_move v73
;; @0023                               store notrap aligned v74, v34+4
;;                                     v75 = iconst.i64 48
;; @0023                               istore32 notrap aligned v75, v34+8  ; v75 = 48
;; @0023                               jump block4(v10, v34)
;;
;;                                 block3 cold:
;; @0023                               v22 = iconst.i32 -1342177246
;; @0023                               v24 = load.i64 notrap aligned readonly can_move v0+40
;; @0023                               v25 = load.i32 notrap aligned readonly can_move v24
;; @0023                               v7 = iconst.i32 48
;; @0023                               v26 = iconst.i32 16
;; @0023                               v27 = call fn0(v0, v22, v25, v7, v26)  ; v22 = -1342177246, v7 = 48, v26 = 16
;; @0023                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0023                               v28 = load.i64 notrap aligned readonly can_move v49+32
;; @0023                               v29 = uextend.i64 v27
;; @0023                               v30 = iadd v28, v29
;; @0023                               jump block4(v27, v30)
;;
;;                                 block4(v39: i32, v40: i64):
;; @0023                               v3 = f32const 0.0
;;                                     v48 = iconst.i64 16
;; @0023                               v41 = iadd v40, v48  ; v48 = 16
;; @0023                               store user2 little v3, v41  ; v3 = 0.0
;; @0023                               v4 = iconst.i32 0
;;                                     v47 = iconst.i64 20
;; @0023                               v42 = iadd v40, v47  ; v47 = 20
;; @0023                               istore8 user2 little v4, v42  ; v4 = 0
;;                                     v46 = iconst.i64 24
;; @0023                               v43 = iadd v40, v46  ; v46 = 24
;; @0023                               store user2 little v4, v43  ; v4 = 0
;; @0023                               v6 = vconst.i8x16 const0
;;                                     v45 = iconst.i64 32
;; @0023                               v44 = iadd v40, v45  ; v45 = 32
;; @0023                               store user2 little v6, v44  ; v6 = const0
;; @0026                               jump block1(v39)
;;
;;                                 block1(v2: i32):
;; @0026                               return v2
;; }
