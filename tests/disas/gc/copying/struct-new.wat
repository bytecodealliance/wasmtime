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
;;                                     v57 = stack_addr.i64 ss0
;;                                     store notrap v4, v57
;; @002a                               v8 = load.i64 notrap aligned readonly can_move v0+32
;; @002a                               v9 = load.i32 notrap aligned can_move v8
;; @002a                               v16 = uextend.i64 v9
;;                                     v58 = iconst.i64 32
;; @002a                               v17 = iadd v16, v58  ; v58 = 32
;; @002a                               v10 = load.i32 notrap aligned readonly can_move v8+4
;; @002a                               v18 = uextend.i64 v10
;; @002a                               v19 = icmp ule v17, v18
;; @002a                               brif v19, block2, block3
;;
;;                                 block2:
;;                                     v76 = iconst.i32 32
;;                                     v72 = iadd.i32 v9, v76  ; v76 = 32
;; @002a                               store notrap aligned vmctx v72, v8
;;                                     v77 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v78 = load.i32 notrap aligned readonly can_move v77
;; @002a                               v37 = uextend.i64 v78
;;                                     v79 = iconst.i64 32
;;                                     v80 = ishl v37, v79  ; v79 = 32
;; @002a                               v39 = iconst.i64 0xb000_0000
;;                                     v74 = bor v80, v39  ; v39 = 0xb000_0000
;;                                     v81 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v82 = load.i64 notrap aligned readonly can_move v81+32
;; @002a                               v33 = iadd v82, v16
;; @002a                               store notrap aligned vmctx v74, v33
;; @002a                               store notrap aligned v76, v33+8  ; v76 = 32
;; @002a                               jump block4(v9, v33)
;;
;;                                 block3 cold:
;; @002a                               v21 = iconst.i32 -1342177280
;; @002a                               v35 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v36 = load.i32 notrap aligned readonly can_move v35
;; @002a                               v6 = iconst.i32 32
;; @002a                               v25 = iconst.i32 16
;; @002a                               v26 = call fn0(v0, v21, v36, v6, v25), stack_map=[i32 @ ss0+0]  ; v21 = -1342177280, v6 = 32, v25 = 16
;; @002a                               v55 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v31 = load.i64 notrap aligned readonly can_move v55+32
;; @002a                               v28 = uextend.i64 v26
;; @002a                               v29 = iadd v31, v28
;; @002a                               jump block4(v26, v29)
;;
;;                                 block4(v42: i32, v43: i64):
;;                                     v51 = iconst.i64 16
;; @002a                               v44 = iadd v43, v51  ; v51 = 16
;; @002a                               store.f32 notrap aligned little v2, v44
;;                                     v50 = iconst.i64 20
;; @002a                               v45 = iadd v43, v50  ; v50 = 20
;; @002a                               istore8.i32 notrap aligned little v3, v45
;;                                     v47 = load.i32 notrap v57
;;                                     v49 = iconst.i64 24
;; @002a                               v46 = iadd v43, v49  ; v49 = 24
;; @002a                               store notrap aligned little v47, v46
;; @002d                               jump block1(v42)
;;
;;                                 block1(v5: i32):
;; @002d                               return v5
;; }
