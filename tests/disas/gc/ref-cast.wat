;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result (ref $s))
    (ref.cast (ref $s) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     fn0 = colocated u1:35 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v38 = stack_addr.i64 ss0
;;                                     store notrap v2, v38
;;                                     v40 = iconst.i32 0
;; @001e                               v4 = icmp eq v2, v40  ; v40 = 0
;; @001e                               v5 = uextend.i32 v4
;; @001e                               v7 = iconst.i32 1
;;                                     v48 = select v2, v7, v40  ; v7 = 1, v40 = 0
;; @001e                               brif v5, block4(v48), block2
;;
;;                                 block2:
;;                                     v55 = iconst.i32 1
;;                                     v56 = band.i32 v2, v55  ; v55 = 1
;;                                     v57 = iconst.i32 0
;;                                     v58 = select v56, v57, v55  ; v57 = 0, v55 = 1
;; @001e                               brif v56, block4(v58), block3
;;
;;                                 block3:
;; @001e                               v20 = uextend.i64 v2
;; @001e                               v21 = iconst.i64 4
;; @001e                               v22 = uadd_overflow_trap v20, v21, user1  ; v21 = 4
;; @001e                               v23 = iconst.i64 8
;; @001e                               v24 = uadd_overflow_trap v22, v23, user1  ; v23 = 8
;; @001e                               v19 = load.i64 notrap aligned readonly v0+48
;; @001e                               v25 = icmp ule v24, v19
;; @001e                               trapz v25, user1
;; @001e                               v18 = load.i64 notrap aligned readonly v0+40
;; @001e                               v26 = iadd v18, v22
;; @001e                               v27 = load.i32 notrap aligned readonly v26
;; @001e                               v15 = load.i64 notrap aligned readonly v0+80
;; @001e                               v16 = load.i32 notrap aligned readonly v15
;; @001e                               v28 = icmp eq v27, v16
;; @001e                               v29 = uextend.i32 v28
;; @001e                               brif v29, block6(v29), block5
;;
;;                                 block5:
;; @001e                               v31 = call fn0(v0, v27, v16), stack_map=[i32 @ ss0+0]
;; @001e                               jump block6(v31)
;;
;;                                 block6(v32: i32):
;; @001e                               jump block4(v32)
;;
;;                                 block4(v33: i32):
;; @001e                               trapz v33, user19
;;                                     v34 = load.i32 notrap v38
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v34
;; }
