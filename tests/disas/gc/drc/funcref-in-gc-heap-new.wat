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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     fn1 = colocated u1:28 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v7 = iconst.i32 -1342177280
;; @0020                               v5 = iconst.i32 0
;; @0020                               v4 = iconst.i32 32
;; @0020                               v10 = iconst.i32 8
;; @0020                               v11 = call fn0(v0, v7, v5, v4, v10)  ; v7 = -1342177280, v5 = 0, v4 = 32, v10 = 8
;;                                     v22 = stack_addr.i64 ss0
;;                                     store notrap v11, v22
;; @0020                               v18 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v19 = ireduce.i32 v18
;; @0020                               v13 = load.i64 notrap aligned readonly can_move v0+40
;; @0020                               v14 = uextend.i64 v11
;; @0020                               v15 = iadd v13, v14
;;                                     v24 = iconst.i64 24
;; @0020                               v16 = iadd v15, v24  ; v24 = 24
;; @0020                               store notrap aligned little v19, v16
;;                                     v20 = load.i32 notrap v22
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v20
;; }
