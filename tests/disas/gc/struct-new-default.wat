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
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               v9 = iconst.i32 -1342177280
;; @0023                               v11 = load.i64 notrap aligned readonly can_move v0+40
;; @0023                               v12 = load.i32 notrap aligned readonly can_move v11
;; @0023                               v7 = iconst.i32 64
;; @0023                               v13 = iconst.i32 16
;; @0023                               v14 = call fn0(v0, v9, v12, v7, v13)  ; v9 = -1342177280, v7 = 64, v13 = 16
;; @0023                               v3 = f32const 0.0
;; @0023                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0023                               v15 = load.i64 notrap aligned readonly can_move v49+32
;; @0023                               v16 = uextend.i64 v14
;; @0023                               v17 = iadd v15, v16
;;                                     v48 = iconst.i64 24
;; @0023                               v18 = iadd v17, v48  ; v48 = 24
;; @0023                               store notrap aligned little v3, v18  ; v3 = 0.0
;; @0023                               v4 = iconst.i32 0
;;                                     v47 = iconst.i64 28
;; @0023                               v19 = iadd v17, v47  ; v47 = 28
;; @0023                               istore8 notrap aligned little v4, v19  ; v4 = 0
;;                                     v45 = iconst.i32 1
;; @0023                               brif v45, block3, block2  ; v45 = 1
;;
;;                                 block2:
;; @0023                               v28 = iconst.i64 8
;; @0023                               v29 = iadd.i64 v15, v28  ; v28 = 8
;; @0023                               v30 = load.i64 notrap aligned v29
;;                                     v41 = iconst.i64 1
;; @0023                               v31 = iadd v30, v41  ; v41 = 1
;; @0023                               store notrap aligned v31, v29
;; @0023                               jump block3
;;
;;                                 block3:
;;                                     v71 = iconst.i32 0
;;                                     v46 = iconst.i64 32
;; @0023                               v20 = iadd.i64 v17, v46  ; v46 = 32
;; @0023                               store notrap aligned little v71, v20  ; v71 = 0
;; @0023                               v6 = vconst.i8x16 const0
;;                                     v38 = iconst.i64 48
;; @0023                               v37 = iadd.i64 v17, v38  ; v38 = 48
;; @0023                               store notrap aligned little v6, v37  ; v6 = const0
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return v14
;; }
