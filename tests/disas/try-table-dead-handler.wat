;;! target = "x86_64"
;;! test = "optimize"

(module
  (tag $t)
  (func $f)
  (func $a
    (block
      (try_table (catch $t 0) (catch_all 0)
        (call $f))
    )
  )
  (func $b
    (block
      (try_table (catch_all 0) (catch $t 0)
        (call $f))
    )
  )
)
;; function u0:0(i64 vmctx, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001e                               jump block1
;;
;;                                 block1:
;; @001e                               return
;; }
;;
;; function u0:1(i64 vmctx, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               jump block3
;;
;;                                 block5(v2: i64):
;; @0023                               jump block2
;;
;;                                 block6(v4: i64):
;; @0023                               jump block2
;;
;;                                 block3:
;; @002b                               try_call fn0(v0, v0), sig0, block7, [ context v0, tag0: block6(exn0), default: block5(exn0) ]
;;
;;                                 block7:
;; @002d                               jump block4
;;
;;                                 block4:
;; @002e                               jump block2
;;
;;                                 block2:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
;;
;; function u0:2(i64 vmctx, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0034                               jump block3
;;
;;                                 block6(v4: i64):
;; @0034                               jump block2
;;
;;                                 block3:
;; @003c                               try_call fn0(v0, v0), sig0, block7, [ context v0, default: block6(exn0) ]
;;
;;                                 block7:
;; @003e                               jump block4
;;
;;                                 block4:
;; @003f                               jump block2
;;
;;                                 block2:
;; @0040                               jump block1
;;
;;                                 block1:
;; @0040                               return
;; }
