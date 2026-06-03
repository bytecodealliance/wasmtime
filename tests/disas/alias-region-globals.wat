;;! target = "x86_64"
;;! test = "optimize"

(module
  (import "env" "g" (global $imported (mut i32)))
  (global $defined (mut i32) (i32.const 0))

  (func (export "test") (param i32) (result i32)
    ;; Set imported global
    (global.set $imported (local.get 0))
    ;; Set defined global
    (global.set $defined (local.get 0))
    ;; Get imported global (should alias with first set)
    (global.get $imported)
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 1610612736 "PublicGlobal"
;;     region1 = 1879048192 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0039                               v4 = load.i64 notrap aligned readonly can_move v0+48
;; @0039                               store notrap aligned region0 v2, v4
;; @003d                               store notrap aligned region1 v2, v0+80
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return v2
;; }
