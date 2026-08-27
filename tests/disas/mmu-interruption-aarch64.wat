;;! target = "aarch64"
;;! flags = ["-Wmmu-interruption=y"]

(module
  (memory 0)
  (func)
)
;; function u0:0(i64 vmctx, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108896 "VMStoreContext+0x20"
;;     region2 = 67108880 "VMStoreContext+0x10"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001b                               v2 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001b                               v3 = load.i64 notrap aligned region2 v2+16
;; @001b                               dead_load_with_context v3, v0
;; @001c                               jump block1
;;
;;                                 block1:
;; @001c                               return
;; }
