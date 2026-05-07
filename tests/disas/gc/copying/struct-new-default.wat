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
;;                                     v55 = iconst.i64 32
;; @0021                               v17 = iadd v16, v55  ; v55 = 32
;; @0021                               v10 = load.i32 notrap aligned readonly can_move v8+4
;; @0021                               v18 = uextend.i64 v10
;; @0021                               v19 = icmp ule v17, v18
;; @0021                               brif v19, block2, block3
;;
;;                                 block2:
;;                                     v73 = iconst.i32 32
;;                                     v69 = iadd.i32 v9, v73  ; v73 = 32
;; @0021                               store notrap aligned vmctx v69, v8
;;                                     v74 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v75 = load.i32 notrap aligned readonly can_move v74
;; @0021                               v37 = uextend.i64 v75
;;                                     v76 = iconst.i64 32
;;                                     v77 = ishl v37, v76  ; v76 = 32
;; @0021                               v39 = iconst.i64 0xb000_0000
;;                                     v71 = bor v77, v39  ; v39 = 0xb000_0000
;;                                     v78 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v79 = load.i64 notrap aligned readonly can_move v78+32
;; @0021                               v33 = iadd v79, v16
;; @0021                               store notrap aligned vmctx v71, v33
;; @0021                               store notrap aligned v73, v33+8  ; v73 = 32
;; @0021                               jump block4(v9, v33)
;;
;;                                 block3 cold:
;; @0021                               v21 = iconst.i32 -1342177280
;; @0021                               v35 = load.i64 notrap aligned readonly can_move v0+40
;; @0021                               v36 = load.i32 notrap aligned readonly can_move v35
;; @0021                               v6 = iconst.i32 32
;; @0021                               v25 = iconst.i32 16
;; @0021                               v26 = call fn0(v0, v21, v36, v6, v25)  ; v21 = -1342177280, v6 = 32, v25 = 16
;; @0021                               v53 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v31 = load.i64 notrap aligned readonly can_move v53+32
;; @0021                               v28 = uextend.i64 v26
;; @0021                               v29 = iadd v31, v28
;; @0021                               jump block4(v26, v29)
;;
;;                                 block4(v42: i32, v43: i64):
;; @0021                               v3 = f32const 0.0
;;                                     v49 = iconst.i64 16
;; @0021                               v44 = iadd v43, v49  ; v49 = 16
;; @0021                               store notrap aligned little v3, v44  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;;                                     v48 = iconst.i64 20
;; @0021                               v45 = iadd v43, v48  ; v48 = 20
;; @0021                               istore8 notrap aligned little v4, v45  ; v4 = 0
;;                                     v47 = iconst.i64 24
;; @0021                               v46 = iadd v43, v47  ; v47 = 24
;; @0021                               store notrap aligned little v4, v46  ; v4 = 0
;; @0024                               jump block1(v42)
;;
;;                                 block1(v2: i32):
;; @0024                               return v2
;; }
