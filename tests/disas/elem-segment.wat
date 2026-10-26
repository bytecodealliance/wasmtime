;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "module_start"

;; Passive funcref element segments store their `ref.func` values into the
;; runtime `ValRaw` storage of the segment during instantiation. Those stores
;; should carry the `ElementSegment` alias region.

(module
  (func $f)
  (elem func $f $f $f)
)
;; function u2415919104:1(i64 vmctx, i64, i64, i64) -> i8 system_v {
;;     region0 = 123 ""
;;     region1 = 231 ""
;;     region2 = 243 ""
;;     region3 = 209 ""
;;     region4 = 68 ""
;;     sig0 = (i64 vmctx, i64) tail
;;     fn0 = colocated u2415919104:0 sig0
;;
;; block0(v0: i64, v1: i64, v2: i64, v3: i64):
;;     jump block1
;;
;; block1:
;;     v7 = get_frame_pointer.i64 
;;     v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;;     store notrap aligned region1 v7, v6+72
;;     v8 = get_stack_pointer.i64 
;;     store notrap aligned region2 v8, v6+64
;;     v9 = get_exception_handler_address.i64 block1, 0
;;     store notrap aligned region3 v9, v6+80
;;     try_call fn0(v0, v1), sig0, block2, [ default: block3 ]
;;
;; block2:
;;     v13 = iconst.i8 1
;;     return v13  ; v13 = 1
;;
;; block3:
;;     v11 = iconst.i64 1
;;     store notrap aligned region4 v11, v6+136  ; v11 = 1
;;     v12 = iconst.i8 0
;;     return v12  ; v12 = 0
;; }
;;
;; function u2415919104:0(i64 vmctx, i64) tail {
;;     region0 = 130 ""
;;     region1 = 134 ""
;;     region2 = 44 ""
;;     region3 = 120 ""
;;     region4 = 100 ""
;;     region5 = 160 ""
;;     region6 = 116 ""
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64, i64, i64) -> i8 system_v
;;     sig3 = (i64 vmctx, i64, i64, i64) -> i8 system_v
;;     sig4 = (i64 vmctx, i64, i64, i64) -> i8 system_v
;;     fn0 = colocated u805306368:4 sig0
;;     fn1 = colocated u0:0 sig1
;;     fn2 = colocated u268435456:0 sig2
;;     fn3 = colocated u268435456:0 sig3
;;     fn4 = colocated u268435456:0 sig4
;;
;; block0(v0: i64, v1: i64):
;;     v2 = iconst.i32 0
;;     v3 = call fn0(v0, v2)  ; v2 = 0
;;     v8 = iconst.i64 48
;;     v9 = iadd v0, v8  ; v8 = 48
;;     store notrap aligned little region6 v9, v3
;;     store notrap aligned little region6 v9, v3+16
;;     v17 = func_addr.i64 fn4
;;     store notrap aligned region2 v17, v0+48
;;     v4 = func_addr.i64 fn1
;;     store notrap aligned region3 v4, v0+56
;;     store notrap aligned region4 v0, v0+72
;;     v6 = load.i64 notrap aligned readonly can_move region0 v0+40
;;     v7 = load.i32 notrap aligned readonly can_move region1 v6
;;     store notrap aligned region5 v7, v0+64
;;     store notrap aligned little region6 v9, v3+32
;;     return
;; }
