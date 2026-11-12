;;! target = "x86_64"
;;! test = "clif"
;;! filter = "wasm[0]--function"

;; A lowered component import cannot unwind a Wasm exception into its caller
(component
  (import "f" (func $f))
  (core func $lower (canon lower (func $f)))
  (core module $m
    (import "" "f" (func $f))
    (func (export "run")
      (block
        (try_table (catch_all 0)
          (call $f)))))
  (core instance (instantiate $m
    (with "" (instance (export "f" (func $lower)))))))

;; function u0:0(i64 vmctx, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 42 ""
;;     region3 = 174 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004a                               jump block3
;;
;;                                 block3:
;; @004f                               v4 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @004f                               v5 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @004f                               call_indirect sig0, v5(v4, v0)
;; @0051                               jump block4
;;
;;                                 block4:
;; @0052                               jump block2
;;
;;                                 block2:
;; @0053                               jump block1
;;
;;                                 block1:
;; @0053                               return
;; }
