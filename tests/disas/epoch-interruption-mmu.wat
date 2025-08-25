;;! target = "x86_64"
;;! flags = ["-Wepoch-interruption-via-mmu=y"]

(module
  (memory 0)
  (func)
)
;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001b                               v2 = iconst.i32 33
;; @001c                               jump block1
;;
;;                                 block1:
;; @001c                               return
;; }
