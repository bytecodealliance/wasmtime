;;! target = 'x86_64'
;;! test = 'optimize'
;;! filter = 'module_start'

(module
  (global i64 i64.const 0 i64.const 0 i64.add)
)
;; function u2415919104:1(i64 vmctx, i64, i64, i64) -> i8 system_v {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435528 "VMStoreContext+0x48"
;;     region2 = 268435520 "VMStoreContext+0x40"
;;     region3 = 268435536 "VMStoreContext+0x50"
;;     region4 = 268435592 "VMStoreContext+0x88"
;;     sig0 = (i64 vmctx, i64) tail
;;     fn0 = colocated u2415919104:0 sig0
;;
;; block0(v0: i64, v1: i64, v2: i64, v3: i64):
;;     jump block1
;;
;; block1:
;;     v5 = get_frame_pointer.i64 
;;     v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;;     store notrap aligned region1 v5, v4+72
;;     v6 = get_stack_pointer.i64 
;;     store notrap aligned region2 v6, v4+64
;;     v7 = get_exception_handler_address.i64 block1, 0
;;     store notrap aligned region3 v7, v4+80
;;     try_call fn0(v0, v1), sig0, block2, [ default: block3 ]
;;
;; block2:
;;     v8 = iconst.i8 1
;;     return v8  ; v8 = 1
;;
;; block3:
;;     v9 = iconst.i64 1
;;     store notrap aligned region4 v9, v4+136  ; v9 = 1
;;     v10 = iconst.i8 0
;;     return v10  ; v10 = 0
;; }
;;
;; function u2415919104:0(i64 vmctx, i64) tail {
;;     region0 = 1879048192 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;
;; block0(v0: i64, v1: i64):
;;     v2 = iconst.i64 0
;;     store notrap aligned region0 v2, v0+48  ; v2 = 0
;;     return
;; }
