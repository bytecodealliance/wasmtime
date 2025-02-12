;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     fn0 = colocated u1:35 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v35 = stack_addr.i64 ss0
;;                                     store notrap v2, v35
;;                                     v37 = iconst.i32 0
;; @001e                               v4 = icmp eq v2, v37  ; v37 = 0
;; @001e                               v5 = uextend.i32 v4
;; @001e                               brif v5, block4(v37), block2  ; v37 = 0
;;
;;                                 block2:
;; @001e                               v7 = iconst.i32 1
;; @001e                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v41 = iconst.i32 0
;; @001e                               brif v8, block4(v41), block3  ; v41 = 0
;;
;;                                 block3:
;; @001e                               v17 = uextend.i64 v2
;; @001e                               v18 = iconst.i64 4
;; @001e                               v19 = uadd_overflow_trap v17, v18, user1  ; v18 = 4
;; @001e                               v21 = uadd_overflow_trap v19, v18, user1  ; v18 = 4
;; @001e                               v16 = load.i64 notrap aligned readonly v0+48
;; @001e                               v22 = icmp ule v21, v16
;; @001e                               trapz v22, user1
;; @001e                               v14 = load.i64 notrap aligned readonly v0+40
;; @001e                               v23 = iadd v14, v19
;; @001e                               v24 = load.i32 notrap aligned readonly v23
;; @001e                               v11 = load.i64 notrap aligned readonly v0+64
;; @001e                               v12 = load.i32 notrap aligned readonly v11
;; @001e                               v25 = icmp eq v24, v12
;; @001e                               v26 = uextend.i32 v25
;; @001e                               brif v26, block6(v26), block5
;;
;;                                 block5:
;; @001e                               v28 = call fn0(v0, v24, v12), stack_map=[i32 @ ss0+0]
;; @001e                               jump block6(v28)
;;
;;                                 block6(v29: i32):
;; @001e                               jump block4(v29)
;;
;;                                 block4(v30: i32):
;; @001e                               trapz v30, user19
;;                                     v31 = load.i32 notrap v35
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v31
;; }
