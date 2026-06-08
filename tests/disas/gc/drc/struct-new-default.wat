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
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v7 = iconst.i32 -1342177280
;; @0021                               v8 = load.i64 notrap aligned readonly can_move v0+40
;; @0021                               v9 = load.i32 notrap aligned readonly can_move v8
;; @0021                               v6 = iconst.i32 40
;; @0021                               v10 = iconst.i32 8
;; @0021                               v11 = call fn0(v0, v7, v9, v6, v10)  ; v7 = -1342177280, v6 = 40, v10 = 8
;; @0021                               v3 = f32const 0.0
;; @0021                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v12 = load.i64 notrap aligned readonly can_move v44+32
;; @0021                               v13 = uextend.i64 v11
;; @0021                               v14 = iadd v12, v13
;; @0021                               v15 = iconst.i64 24
;; @0021                               v16 = iadd v14, v15  ; v15 = 24
;; @0021                               store user2 little region0 v3, v16  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;; @0021                               v17 = iconst.i64 28
;; @0021                               v18 = iadd v14, v17  ; v17 = 28
;; @0021                               istore8 user2 little region0 v4, v18  ; v4 = 0
;;                                     jump block3
;;
;;                                 block3:
;;                                     v63 = iconst.i32 0
;; @0021                               v19 = iconst.i64 32
;; @0021                               v20 = iadd.i64 v14, v19  ; v19 = 32
;; @0021                               store user2 little region0 v63, v20  ; v63 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v11
;; }
