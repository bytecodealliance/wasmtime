;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut f64)))

  (func $fill (param $len i32) (result (ref $a))
    (array.new_default $a (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:2 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v5 = uextend.i64 v2
;;                                     v79 = iconst.i64 3
;;                                     v80 = ishl v5, v79  ; v79 = 3
;;                                     v77 = iconst.i64 32
;; @001f                               v7 = ushr v80, v77  ; v77 = 32
;; @001f                               trapnz v7, user18
;; @001f                               v4 = iconst.i32 32
;;                                     v86 = iconst.i32 3
;;                                     v87 = ishl v2, v86  ; v86 = 3
;; @001f                               v9 = uadd_overflow_trap v4, v87, user18  ; v4 = 32
;; @001f                               v11 = iconst.i32 -1476395008
;; @001f                               v13 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v14 = load.i32 notrap aligned readonly can_move v13
;;                                     v84 = iconst.i32 8
;; @001f                               v16 = call fn0(v0, v11, v14, v9, v84)  ; v11 = -1476395008, v84 = 8
;;                                     v76 = stack_addr.i64 ss0
;;                                     store notrap v16, v76
;; @001f                               v74 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v17 = load.i64 notrap aligned readonly can_move v74+32
;; @001f                               v18 = uextend.i64 v16
;; @001f                               v19 = iadd v17, v18
;;                                     v72 = iconst.i64 24
;; @001f                               v20 = iadd v19, v72  ; v72 = 24
;; @001f                               store user2 v2, v20
;;                                     v54 = load.i32 notrap v76
;; @001f                               trapz v54, user16
;; @001f                               v23 = uextend.i64 v54
;; @001f                               v25 = iadd v17, v23
;; @001f                               v27 = iadd v25, v72  ; v72 = 24
;; @001f                               v28 = load.i32 user2 readonly v27
;; @001f                               v29 = uextend.i64 v28
;; @001f                               v34 = icmp ugt v5, v29
;; @001f                               trapnz v34, user17
;; @001f                               v45 = load.i64 notrap aligned v74+40
;; @001f                               v38 = iadd v25, v77  ; v77 = 32
;; @001f                               v47 = uadd_overflow_trap v38, v80, user2
;; @001f                               v46 = iadd v17, v45
;; @001f                               v48 = icmp ugt v47, v46
;; @001f                               trapnz v48, user2
;; @001f                               v22 = iconst.i32 0
;; @001f                               call fn1(v0, v38, v22, v80), stack_map=[i32 @ ss0+0]  ; v22 = 0
;;                                     v51 = load.i32 notrap v76
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v51
;; }
