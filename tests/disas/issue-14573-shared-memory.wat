;;! target = "x86_64"
;;! test = "optimize"
;;! flags = ["-Wthreads=yes"]

;; Other threads can observe the first store while this one spins in the loop,
;; so it must not be eliminated.

(module
  (memory 1 1 shared)
  (func (param i32)
    (i32.store (i32.const 0) (i32.const 1))
    (if (local.get 0) (then (loop (br 0))))
    (i32.store (i32.const 0) (i32.const 2))
  )
)

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 136 ""
;;     region3 = 215 ""
;;     region4 = 105 ""
;;     region5 = 171 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0020                               v4 = iconst.i32 1
;; @0022                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0022                               v7 = load.i64 notrap aligned readonly can_move region3 v6
;; @0022                               store little region5 v4, v7  ; v4 = 1
;; @0027                               brif v2, block2, block3
;;
;;                                 block2:
;; @0029                               jump block4
;;
;;                                 block4:
;; @002b                               jump block4
;;
;;                                 block3:
;; @0031                               v10 = iconst.i32 2
;; @0033                               store little region5 v10, v7  ; v10 = 2
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return
;; }
