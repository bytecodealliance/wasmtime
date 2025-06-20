;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:28 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v8 = iconst.i32 -1342177280
;; @0021                               v4 = iconst.i32 0
;; @0021                               v6 = iconst.i32 32
;; @0021                               v10 = iconst.i32 8
;; @0021                               v11 = call fn0(v0, v8, v4, v6, v10)  ; v8 = -1342177280, v4 = 0, v6 = 32, v10 = 8
;; @0021                               v3 = f32const 0.0
;; @0021                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v12 = load.i64 notrap aligned readonly can_move v44+24
;; @0021                               v13 = uextend.i64 v11
;; @0021                               v14 = iadd v12, v13
;;                                     v43 = iconst.i64 16
;; @0021                               v15 = iadd v14, v43  ; v43 = 16
;; @0021                               store notrap aligned little v3, v15  ; v3 = 0.0
;;                                     v42 = iconst.i64 20
;; @0021                               v16 = iadd v14, v42  ; v42 = 20
;; @0021                               istore8 notrap aligned little v4, v16  ; v4 = 0
;;                                     v40 = iconst.i32 1
;; @0021                               brif v40, block3, block2  ; v40 = 1
;;
;;                                 block2:
;; @0021                               v25 = iconst.i64 8
;; @0021                               v26 = iadd.i64 v12, v25  ; v25 = 8
;; @0021                               v27 = load.i64 notrap aligned v26
;;                                     v36 = iconst.i64 1
;; @0021                               v28 = iadd v27, v36  ; v36 = 1
;; @0021                               store notrap aligned v28, v26
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v66 = iconst.i32 0
;;                                     v41 = iconst.i64 24
;; @0021                               v17 = iadd.i64 v14, v41  ; v41 = 24
;; @0021                               store notrap aligned little v66, v17  ; v66 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v11
;; }
