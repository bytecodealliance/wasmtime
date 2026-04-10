;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param funcref) (result (ref $ty))
    (struct.new $ty (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:27 sig0
;;     fn1 = colocated u805306368:28 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v6 = iconst.i32 -1342177280
;; @0020                               v8 = load.i64 notrap aligned readonly can_move v0+40
;; @0020                               v9 = load.i32 notrap aligned readonly can_move v8
;; @0020                               v4 = iconst.i32 32
;; @0020                               v10 = iconst.i32 8
;; @0020                               v11 = call fn0(v0, v6, v9, v4, v10)  ; v6 = -1342177280, v4 = 32, v10 = 8
;;                                     v26 = stack_addr.i64 ss0
;;                                     store notrap v11, v26
;; @0020                               v17 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v18 = ireduce.i32 v17
;; @0020                               v24 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v12 = load.i64 notrap aligned readonly can_move v24+32
;; @0020                               v13 = uextend.i64 v11
;; @0020                               v14 = iadd v12, v13
;;                                     v22 = iconst.i64 24
;; @0020                               v15 = iadd v14, v22  ; v22 = 24
;; @0020                               store notrap aligned little v18, v15
;;                                     v19 = load.i32 notrap v26
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v19
;; }
