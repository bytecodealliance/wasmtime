;;! target = "x86_64"

(module
  (global $x (mut i32) (i32.const 4))
  (memory 1)
  (func $main (local i32)
    (i32.store (i32.const 0) (global.get $x))
  )
  (start $main)
)

;; function u0:0(i64 vmctx, i64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0027                               v2 = iconst.i32 0
;; @0029                               v3 = iconst.i32 0
;; @002b                               v4 = global_value.i64 gv3
;; @002b                               v5 = load.i32 notrap aligned table v4+112
;; @002d                               v6 = uextend.i64 v3  ; v3 = 0
;; @002d                               v7 = global_value.i64 gv4
;; @002d                               v8 = iadd v7, v6
;; @002d                               store little heap v5, v8
;; @0030                               jump block1
;;
;;                                 block1:
;; @0030                               return
;; }
