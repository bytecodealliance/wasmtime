;;! target = "x86_64"
;;! test = "optimize"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     sig0 = (i64 vmctx) -> i64 tail
;;     fn0 = colocated u805306368:13 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0016                               v3 = load.i64 notrap aligned v0+24
;; @0016                               v4 = load.i64 notrap aligned v3
;; @0016                               v5 = load.i64 notrap aligned readonly can_move v0+8
;; @0016                               v6 = load.i64 notrap aligned v5+8
;; @0016                               v7 = icmp uge v4, v6
;; @0016                               brif v7, block3, block2(v6)
;;
;;                                 block3 cold:
;; @0016                               v8 = call fn0(v0)
;; @0016                               jump block2(v8)
;;
;;                                 block2(v19: i64):
;; @0017                               jump block4(v19)
;;
;;                                 block4(v11: i64):
;; @0017                               v10 = load.i64 notrap aligned v3
;; @0017                               v12 = icmp uge v10, v11
;; @0017                               brif v12, block7, block6(v11)
;;
;;                                 block7 cold:
;; @0017                               v14 = load.i64 notrap aligned v5+8
;; @0017                               v15 = icmp.i64 uge v10, v14
;; @0017                               brif v15, block8, block6(v14)
;;
;;                                 block8 cold:
;; @0017                               v16 = call fn0(v0)
;; @0017                               jump block6(v16)
;;
;;                                 block6(v20: i64):
;; @0019                               jump block4(v20)
;; }
