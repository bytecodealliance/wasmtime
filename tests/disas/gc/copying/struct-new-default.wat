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
;; @0021                               v8 = iconst.i32 -1342177280
;; @0021                               v10 = load.i64 notrap aligned readonly can_move v0+40
;; @0021                               v11 = load.i32 notrap aligned readonly can_move v10
;; @0021                               v6 = iconst.i32 32
;; @0021                               v12 = iconst.i32 16
;; @0021                               v13 = call fn0(v0, v8, v11, v6, v12)  ; v8 = -1342177280, v6 = 32, v12 = 16
;; @0021                               v3 = f32const 0.0
;; @0021                               v23 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v14 = load.i64 notrap aligned readonly can_move v23+32
;; @0021                               v15 = uextend.i64 v13
;; @0021                               v16 = iadd v14, v15
;;                                     v22 = iconst.i64 16
;; @0021                               v17 = iadd v16, v22  ; v22 = 16
;; @0021                               store notrap aligned little v3, v17  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;;                                     v21 = iconst.i64 20
;; @0021                               v18 = iadd v16, v21  ; v21 = 20
;; @0021                               istore8 notrap aligned little v4, v18  ; v4 = 0
;;                                     v20 = iconst.i64 24
;; @0021                               v19 = iadd v16, v20  ; v20 = 24
;; @0021                               store notrap aligned little v4, v19  ; v4 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v13
;; }
