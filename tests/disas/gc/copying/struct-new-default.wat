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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v7 = load.i64 notrap aligned readonly can_move v0+32
;; @0021                               v8 = load.i32 notrap aligned v7
;; @0021                               v9 = load.i32 notrap aligned v7+4
;; @0021                               v15 = uextend.i64 v8
;;                                     v44 = iconst.i64 32
;; @0021                               v16 = iadd v15, v44  ; v44 = 32
;; @0021                               v17 = uextend.i64 v9
;; @0021                               v18 = icmp ule v16, v17
;; @0021                               brif v18, block2, block3
;;
;;                                 block2:
;;                                     v60 = iconst.i32 32
;;                                     v58 = iadd.i32 v8, v60  ; v60 = 32
;; @0021                               store notrap aligned region2 v58, v7
;;                                     v61 = iconst.i32 -1342177246
;;                                     v62 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v63 = load.i64 notrap aligned readonly can_move v62+32
;; @0021                               v32 = iadd v63, v15
;; @0021                               store notrap aligned v61, v32  ; v61 = -1342177246
;;                                     v64 = load.i64 notrap aligned readonly can_move region1 v0+40
;;                                     v65 = load.i32 notrap aligned readonly can_move v64
;; @0021                               store notrap aligned v65, v32+4
;;                                     v66 = iconst.i64 32
;; @0021                               istore32 notrap aligned v66, v32+8  ; v66 = 32
;; @0021                               jump block4(v8, v32)
;;
;;                                 block3 cold:
;; @0021                               v19 = iconst.i32 -1342177246
;; @0021                               v20 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @0021                               v21 = load.i32 notrap aligned readonly can_move v20
;; @0021                               v6 = iconst.i32 32
;; @0021                               v22 = iconst.i32 16
;; @0021                               v23 = call fn0(v0, v19, v21, v6, v22)  ; v19 = -1342177246, v6 = 32, v22 = 16
;; @0021                               v24 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0021                               v25 = load.i64 notrap aligned readonly can_move v24+32
;; @0021                               v26 = uextend.i64 v23
;; @0021                               v27 = iadd v25, v26
;; @0021                               jump block4(v23, v27)
;;
;;                                 block4(v36: i32, v37: i64):
;; @0021                               v3 = f32const 0.0
;; @0021                               v38 = iconst.i64 16
;; @0021                               v39 = iadd v37, v38  ; v38 = 16
;; @0021                               store user2 little region3 v3, v39  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;; @0021                               v40 = iconst.i64 20
;; @0021                               v41 = iadd v37, v40  ; v40 = 20
;; @0021                               istore8 user2 little region3 v4, v41  ; v4 = 0
;; @0021                               v42 = iconst.i64 24
;; @0021                               v43 = iadd v37, v42  ; v42 = 24
;; @0021                               store user2 little region3 v4, v43  ; v4 = 0
;; @0024                               jump block1(v36)
;;
;;                                 block1(v2: i32):
;; @0024                               return v2
;; }
