;;! target = "x86_64"
;;! test = "optimize"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     sig0 = (i64 vmctx) -> i64 tail
;;     fn0 = colocated u1:16 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0016                               v4 = load.i64 notrap aligned v0+32
;; @0016                               v5 = load.i64 notrap aligned v4
;; @0016                               v2 = load.i64 notrap aligned readonly can_move v0+8
;; @0016                               v7 = load.i64 notrap aligned v2+8
;; @0016                               v8 = icmp uge v5, v7
;; @0016                               brif v8, block3, block2(v7)
;;
;;                                 block3 cold:
;; @0016                               v10 = call fn0(v0)
;; @0016                               jump block2(v10)
;;
;;                                 block2(v22: i64):
;; @0017                               jump block4(v22)
;;
;;                                 block4(v13: i64):
;; @0017                               v12 = load.i64 notrap aligned v4
;; @0017                               v14 = icmp uge v12, v13
;; @0017                               brif v14, block7, block6(v13)
;;
;;                                 block7 cold:
;; @0017                               v16 = load.i64 notrap aligned v2+8
;; @0017                               v17 = icmp.i64 uge v12, v16
;; @0017                               brif v17, block8, block6(v16)
;;
;;                                 block8 cold:
;; @0017                               v19 = call fn0(v0)
;; @0017                               jump block6(v19)
;;
;;                                 block6(v23: i64):
;; @0019                               jump block4(v23)
;; }
