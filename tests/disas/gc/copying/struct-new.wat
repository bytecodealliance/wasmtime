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
;;                                     v27 = stack_addr.i64 ss0
;;                                     store notrap v4, v27
;; @002a                               v8 = iconst.i32 -1342177280
;; @002a                               v10 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v11 = load.i32 notrap aligned readonly can_move v10
;; @002a                               v6 = iconst.i32 32
;; @002a                               v12 = iconst.i32 16
;; @002a                               v13 = call fn0(v0, v8, v11, v6, v12), stack_map=[i32 @ ss0+0]  ; v8 = -1342177280, v6 = 32, v12 = 16
;; @002a                               v25 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v14 = load.i64 notrap aligned readonly can_move v25+32
;; @002a                               v15 = uextend.i64 v13
;; @002a                               v16 = iadd v14, v15
;;                                     v24 = iconst.i64 16
;; @002a                               v17 = iadd v16, v24  ; v24 = 16
;; @002a                               store notrap aligned little v2, v17
;;                                     v23 = iconst.i64 20
;; @002a                               v18 = iadd v16, v23  ; v23 = 20
;; @002a                               istore8 notrap aligned little v3, v18
;;                                     v20 = load.i32 notrap v27
;;                                     v22 = iconst.i64 24
;; @002a                               v19 = iadd v16, v22  ; v22 = 24
;; @002a                               store notrap aligned little v20, v19
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v13
;; }
