;;! target = "x86_64"

(module
  (func $untyped-select (result i32)
  	i32.const 42
  	i32.const 24
  	i32.const 1
  	select)

  (func $typed-select-1 (result externref)
  	ref.null extern
  	ref.null extern
  	i32.const 1
  	select (result externref))

  (func $typed-select-2 (param externref) (result externref)
    ref.null extern
    local.get 0
    i32.const 1
    select (result externref))
)

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               v3 = iconst.i32 42
;; @0025                               v4 = iconst.i32 24
;; @0027                               v5 = iconst.i32 1
;; @0029                               v6 = select v5, v3, v4  ; v5 = 1, v3 = 42, v4 = 24
;; @002a                               jump block1(v6)
;;
;;                                 block1(v2: i32):
;; @002a                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @002d                               v3 = iconst.i32 0
;; @002f                               v4 = iconst.i32 0
;; @0031                               v5 = iconst.i32 1
;; @0033                               v6 = select v5, v3, v4  ; v5 = 1, v3 = 0, v4 = 0
;; @0036                               jump block1(v6)
;;
;;                                 block1(v2: i32):
;; @0036                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0039                               v4 = iconst.i32 0
;; @003d                               v5 = iconst.i32 1
;; @003f                               v6 = select v5, v4, v2  ; v5 = 1, v4 = 0
;; @0042                               jump block1(v6)
;;
;;                                 block1(v3: i32):
;; @0042                               return v3
;; }
