;;! target = "x86_64"
;;! flags = ["-Wepoch-interruption-via-mmu=y"]

(module
  (memory 0)
  (func)
)
;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001b                               v2 = load.i64 notrap aligned readonly can_move v0+8
;; @001b                               v3 = load.i64 notrap aligned v2+16
;; @001b                               dead_load_with_context v3, v0
;; @001c                               jump block1
;;
;;                                 block1:
;; @001c                               return
;; }
