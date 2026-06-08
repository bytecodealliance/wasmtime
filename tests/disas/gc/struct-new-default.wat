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
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               v8 = load.i64 notrap aligned readonly can_move v0+32
;; @0023                               v9 = load.i32 notrap aligned v8
;; @0023                               v10 = load.i32 notrap aligned v8+4
;; @0023                               v16 = uextend.i64 v9
;;                                     v49 = iconst.i64 48
;; @0023                               v17 = iadd v16, v49  ; v49 = 48
;; @0023                               v18 = uextend.i64 v10
;; @0023                               v19 = icmp ule v17, v18
;; @0023                               brif v19, block2, block3
;;
;;                                 block2:
;;                                     v65 = iconst.i32 48
;;                                     v63 = iadd.i32 v9, v65  ; v65 = 48
;; @0023                               store notrap aligned region0 v63, v8
;;                                     v66 = iconst.i32 -1342177246
;;                                     v67 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v68 = load.i64 notrap aligned readonly can_move v67+32
;; @0023                               v31 = iadd v68, v16
;; @0023                               store notrap aligned v66, v31  ; v66 = -1342177246
;;                                     v69 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v70 = load.i32 notrap aligned readonly can_move v69
;; @0023                               store notrap aligned v70, v31+4
;;                                     v71 = iconst.i64 48
;; @0023                               istore32 notrap aligned v71, v31+8  ; v71 = 48
;; @0023                               jump block4(v9, v31)
;;
;;                                 block3 cold:
;; @0023                               v20 = iconst.i32 -1342177246
;; @0023                               v21 = load.i64 notrap aligned readonly can_move v0+40
;; @0023                               v22 = load.i32 notrap aligned readonly can_move v21
;; @0023                               v7 = iconst.i32 48
;; @0023                               v23 = iconst.i32 16
;; @0023                               v24 = call fn0(v0, v20, v22, v7, v23)  ; v20 = -1342177246, v7 = 48, v23 = 16
;; @0023                               v45 = load.i64 notrap aligned readonly can_move v0+8
;; @0023                               v25 = load.i64 notrap aligned readonly can_move v45+32
;; @0023                               v26 = uextend.i64 v24
;; @0023                               v27 = iadd v25, v26
;; @0023                               jump block4(v24, v27)
;;
;;                                 block4(v35: i32, v36: i64):
;; @0023                               v3 = f32const 0.0
;; @0023                               v37 = iconst.i64 16
;; @0023                               v38 = iadd v36, v37  ; v37 = 16
;; @0023                               store user2 little region1 v3, v38  ; v3 = 0.0
;; @0023                               v4 = iconst.i32 0
;; @0023                               v39 = iconst.i64 20
;; @0023                               v40 = iadd v36, v39  ; v39 = 20
;; @0023                               istore8 user2 little region1 v4, v40  ; v4 = 0
;; @0023                               v41 = iconst.i64 24
;; @0023                               v42 = iadd v36, v41  ; v41 = 24
;; @0023                               store user2 little region1 v4, v42  ; v4 = 0
;; @0023                               v6 = vconst.i8x16 const0
;; @0023                               v43 = iconst.i64 32
;; @0023                               v44 = iadd v36, v43  ; v43 = 32
;; @0023                               store user2 little region1 v6, v44  ; v6 = const0
;; @0026                               jump block1(v35)
;;
;;                                 block1(v2: i32):
;; @0026                               return v2
;; }
