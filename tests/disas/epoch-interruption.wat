;;! target = "x86_64"
;;! test = "optimize"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx) -> i64 tail
;;     fn0 = colocated u805306368:13 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0016                               v2 = load.i64 notrap aligned v0+24
;; @0016                               v3 = load.i64 notrap aligned v2
;; @0016                               v4 = load.i64 notrap aligned readonly can_move v0+8
;; @0016                               v5 = load.i64 notrap aligned v4+8
;; @0016                               v6 = icmp uge v3, v5
;; @0016                               brif v6, block3, block2(v5)
;;
;;                                 block3 cold:
;; @0016                               v7 = call fn0(v0)
;; @0016                               jump block2(v7)
;;
;;                                 block2(v18: i64):
;; @0017                               jump block4(v18)
;;
;;                                 block4(v10: i64):
;; @0017                               v9 = load.i64 notrap aligned v2
;; @0017                               v11 = icmp uge v9, v10
;; @0017                               brif v11, block7, block6(v10)
;;
;;                                 block7 cold:
;; @0017                               v13 = load.i64 notrap aligned v4+8
;; @0017                               v14 = icmp.i64 uge v9, v13
;; @0017                               brif v14, block8, block6(v13)
;;
;;                                 block8 cold:
;; @0017                               v15 = call fn0(v0)
;; @0017                               jump block6(v15)
;;
;;                                 block6(v19: i64):
;; @0019                               jump block4(v19)
;; }
