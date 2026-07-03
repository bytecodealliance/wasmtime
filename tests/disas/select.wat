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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               v2 = iconst.i32 42
;; @0025                               v3 = iconst.i32 24
;; @0027                               v4 = iconst.i32 1
;; @0029                               v5 = select v4, v2, v3  ; v4 = 1, v2 = 42, v3 = 24
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return v5
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @002d                               v2 = iconst.i32 0
;; @002f                               v3 = iconst.i32 0
;; @0031                               v4 = iconst.i32 1
;; @0033                               v5 = select v4, v2, v3  ; v4 = 1, v2 = 0, v3 = 0
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v5
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0039                               v3 = iconst.i32 0
;; @003d                               v4 = iconst.i32 1
;; @003f                               v5 = select v4, v3, v2  ; v4 = 1, v3 = 0
;; @0042                               jump block1
;;
;;                                 block1:
;; @0042                               return v5
;; }
