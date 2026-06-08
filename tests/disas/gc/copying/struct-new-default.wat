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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v7 = load.i64 notrap aligned readonly can_move v0+32
;; @0021                               v8 = load.i32 notrap aligned v7
;; @0021                               v9 = load.i32 notrap aligned v7+4
;; @0021                               v15 = uextend.i64 v8
;;                                     v46 = iconst.i64 32
;; @0021                               v16 = iadd v15, v46  ; v46 = 32
;; @0021                               v17 = uextend.i64 v9
;; @0021                               v18 = icmp ule v16, v17
;; @0021                               brif v18, block2, block3
;;
;;                                 block2:
;;                                     v62 = iconst.i32 32
;;                                     v60 = iadd.i32 v8, v62  ; v62 = 32
;; @0021                               store notrap aligned region0 v60, v7
;;                                     v63 = iconst.i32 -1342177246
;;                                     v64 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v65 = load.i64 notrap aligned readonly can_move v64+32
;; @0021                               v30 = iadd v65, v15
;; @0021                               store notrap aligned v63, v30  ; v63 = -1342177246
;;                                     v66 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v67 = load.i32 notrap aligned readonly can_move v66
;; @0021                               store notrap aligned v67, v30+4
;;                                     v68 = iconst.i64 32
;; @0021                               istore32 notrap aligned v68, v30+8  ; v68 = 32
;; @0021                               jump block4(v8, v30)
;;
;;                                 block3 cold:
;; @0021                               v19 = iconst.i32 -1342177246
;; @0021                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @0021                               v21 = load.i32 notrap aligned readonly can_move v20
;; @0021                               v6 = iconst.i32 32
;; @0021                               v22 = iconst.i32 16
;; @0021                               v23 = call fn0(v0, v19, v21, v6, v22)  ; v19 = -1342177246, v6 = 32, v22 = 16
;; @0021                               v42 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v24 = load.i64 notrap aligned readonly can_move v42+32
;; @0021                               v25 = uextend.i64 v23
;; @0021                               v26 = iadd v24, v25
;; @0021                               jump block4(v23, v26)
;;
;;                                 block4(v34: i32, v35: i64):
;; @0021                               v3 = f32const 0.0
;; @0021                               v36 = iconst.i64 16
;; @0021                               v37 = iadd v35, v36  ; v36 = 16
;; @0021                               store user2 little region1 v3, v37  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;; @0021                               v38 = iconst.i64 20
;; @0021                               v39 = iadd v35, v38  ; v38 = 20
;; @0021                               istore8 user2 little region1 v4, v39  ; v4 = 0
;; @0021                               v40 = iconst.i64 24
;; @0021                               v41 = iadd v35, v40  ; v40 = 24
;; @0021                               store user2 little region1 v4, v41  ; v4 = 0
;; @0024                               jump block1(v34)
;;
;;                                 block1(v2: i32):
;; @0024                               return v2
;; }
