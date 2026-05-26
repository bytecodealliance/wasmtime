;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut funcref)))

  (func $fill (param $len i32) (result (ref $a))
    (array.new_default $a (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 2 "vmctx"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v5 = uextend.i64 v2
;;                                     v110 = iconst.i64 2
;;                                     v111 = ishl v5, v110  ; v110 = 2
;;                                     v108 = iconst.i64 32
;; @001f                               v7 = ushr v111, v108  ; v108 = 32
;; @001f                               trapnz v7, user18
;; @001f                               v4 = iconst.i32 20
;;                                     v117 = iconst.i32 2
;;                                     v118 = ishl v2, v117  ; v117 = 2
;; @001f                               v9 = uadd_overflow_trap v4, v118, user18  ; v4 = 20
;; @001f                               v11 = load.i64 notrap aligned readonly can_move v0+32
;; @001f                               v12 = load.i32 notrap aligned v11
;; @001f                               v13 = load.i32 notrap aligned v11+4
;; @001f                               v19 = uextend.i64 v12
;; @001f                               v14 = uextend.i64 v9
