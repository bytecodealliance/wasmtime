;;! target = "x86_64"
;;! test = "optimize"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx) -> i64 tail
;;     fn0 = colocated u1:16 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0016                               v3 = load.i64 notrap aligned v0+8
;; @0016                               v5 = load.i64 notrap aligned v0+32
;; @0016                               v6 = load.i64 notrap aligned v5
;; @0016                               v7 = load.i64 notrap aligned v3+8
;; @0016                               v8 = icmp uge v6, v7
;; @0016                               brif v8, block3, block2(v7)
;;
;;                                 block3 cold:
;; @0016                               v10 = call fn0(v0)
;; @0016                               jump block2(v10)
;;
;;                                 block2(v21: i64):
;; @0017                               jump block4(v21)
;;
;;                                 block4(v13: i64):
;; @0017                               v12 = load.i64 notrap aligned v5
;; @0017                               v14 = icmp uge v12, v13
;; @0017                               brif v14, block7, block6(v13)
;;
;;                                 block7 cold:
;; @0017                               v15 = load.i64 notrap aligned v3+8
;; @0017                               v16 = icmp.i64 uge v12, v15
;; @0017                               brif v16, block8, block6(v15)
;;
;;                                 block8 cold:
;; @0017                               v18 = call fn0(v0)
;; @0017                               jump block6(v18)
;;
;;                                 block6(v22: i64):
;; @0019                               jump block4(v22)
;; }
