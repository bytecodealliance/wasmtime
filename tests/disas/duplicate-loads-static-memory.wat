;;! target = "x86_64"
;;! test = "optimize"

(module
  (memory (export "memory") 1)
  (func (export "load-without-offset") (param i32) (result i32 i32)
    local.get 0
    i32.load
    local.get 0
    i32.load
  )
  (func (export "load-with-offset") (param i32) (result i32 i32)
    local.get 0
    i32.load offset=1234
    local.get 0
    i32.load offset=1234
  )
)

;; function u0:0(i64 vmctx, i64, i32) -> i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0057                               v6 = load.i64 notrap aligned readonly checked v0+96
;; @0057                               v5 = uextend.i64 v2
;; @0057                               v7 = iadd v6, v5
;; @0057                               v8 = load.i32 little heap v7
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v8, v8
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0064                               v6 = load.i64 notrap aligned readonly checked v0+96
;; @0064                               v5 = uextend.i64 v2
;; @0064                               v7 = iadd v6, v5
;; @0064                               v8 = iconst.i64 1234
;; @0064                               v9 = iadd v7, v8  ; v8 = 1234
;; @0064                               v10 = load.i32 little heap v9
;; @006e                               jump block1
;;
;;                                 block1:
;; @006e                               return v10, v10
;; }
