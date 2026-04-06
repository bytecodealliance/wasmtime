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
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v8 = iconst.i32 -1342177280
;; @0021                               v10 = load.i64 notrap aligned readonly can_move v0+40
;; @0021                               v11 = load.i32 notrap aligned readonly can_move v10
;; @0021                               v6 = iconst.i32 40
;; @0021                               v12 = iconst.i32 8
;; @0021                               v13 = call fn0(v0, v8, v11, v6, v12)  ; v8 = -1342177280, v6 = 40, v12 = 8
;; @0021                               v3 = f32const 0.0
;; @0021                               v46 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v14 = load.i64 notrap aligned readonly can_move v46+32
;; @0021                               v15 = uextend.i64 v13
;; @0021                               v16 = iadd v14, v15
;;                                     v45 = iconst.i64 24
;; @0021                               v17 = iadd v16, v45  ; v45 = 24
;; @0021                               store notrap aligned little v3, v17  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;;                                     v44 = iconst.i64 28
;; @0021                               v18 = iadd v16, v44  ; v44 = 28
;; @0021                               istore8 notrap aligned little v4, v18  ; v4 = 0
;;                                     v42 = iconst.i32 1
;; @0021                               brif v42, block3, block2  ; v42 = 1
;;
;;                                 block2:
;; @0021                               v27 = iconst.i64 8
;; @0021                               v28 = iadd.i64 v14, v27  ; v27 = 8
;; @0021                               v29 = load.i64 notrap aligned v28
;;                                     v38 = iconst.i64 1
;; @0021                               v30 = iadd v29, v38  ; v38 = 1
;; @0021                               store notrap aligned v30, v28
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v68 = iconst.i32 0
;;                                     v43 = iconst.i64 32
;; @0021                               v19 = iadd.i64 v16, v43  ; v43 = 32
;; @0021                               store notrap aligned little v68, v19  ; v68 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v13
;; }
