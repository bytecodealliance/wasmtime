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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:28 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               v9 = iconst.i32 -1342177280
;; @0023                               v4 = iconst.i32 0
;; @0023                               v7 = iconst.i32 48
;; @0023                               v11 = iconst.i32 16
;; @0023                               v12 = call fn0(v0, v9, v4, v7, v11)  ; v9 = -1342177280, v4 = 0, v7 = 48, v11 = 16
;; @0023                               v3 = f32const 0.0
;; @0023                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @0023                               v13 = load.i64 notrap aligned readonly can_move v47+24
;; @0023                               v14 = uextend.i64 v12
;; @0023                               v15 = iadd v13, v14
;;                                     v46 = iconst.i64 16
;; @0023                               v16 = iadd v15, v46  ; v46 = 16
;; @0023                               store notrap aligned little v3, v16  ; v3 = 0.0
;;                                     v45 = iconst.i64 20
;; @0023                               v17 = iadd v15, v45  ; v45 = 20
;; @0023                               istore8 notrap aligned little v4, v17  ; v4 = 0
;;                                     v43 = iconst.i32 1
;; @0023                               brif v43, block3, block2  ; v43 = 1
;;
;;                                 block2:
;; @0023                               v26 = iconst.i64 8
;; @0023                               v27 = iadd.i64 v13, v26  ; v26 = 8
;; @0023                               v28 = load.i64 notrap aligned v27
;;                                     v39 = iconst.i64 1
;; @0023                               v29 = iadd v28, v39  ; v39 = 1
;; @0023                               store notrap aligned v29, v27
;; @0023                               jump block3
;;
;;                                 block3:
;;                                     v69 = iconst.i32 0
;;                                     v44 = iconst.i64 24
;; @0023                               v18 = iadd.i64 v15, v44  ; v44 = 24
;; @0023                               store notrap aligned little v69, v18  ; v69 = 0
;; @0023                               v6 = vconst.i8x16 const0
;;                                     v36 = iconst.i64 32
;; @0023                               v35 = iadd.i64 v15, v36  ; v36 = 32
;; @0023                               store notrap aligned little v6, v35  ; v6 = const0
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return v12
;; }
