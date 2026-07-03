;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "array_to_wasm_trampoline"

(module
  (func (export "f") (param i32 i64) (result i32 i64)
    (local.get 0)
    (local.get 1)
  )
)
;; function u268435456:0(i64 vmctx, i64, i64, i64) -> i8 system_v {
;;     region0 = 2013265920 "HostValRaw+0x0"
;;     region1 = 8 "VMContext+0x8"
;;     region2 = 67108936 "VMStoreContext+0x48"
;;     region3 = 67108928 "VMStoreContext+0x40"
;;     region4 = 67108944 "VMStoreContext+0x50"
;;     region5 = 67109000 "VMStoreContext+0x88"
;;     sig0 = (i64 vmctx, i64, i32, i64) -> i32, i64 tail
;;     fn0 = colocated u0:0 sig0
;;
;; block0(v0: i64, v1: i64, v2: i64, v3: i64):
;;     jump block1
;;
;; block1:
;;     v4 = load.i32 notrap little region0 v2
;;     v5 = load.i64 notrap little region0 v2+16
;;     v7 = get_frame_pointer.i64 
;;     v6 = load.i64 notrap aligned readonly can_move region1 v0+8
;;     store notrap aligned region2 v7, v6+72
;;     v8 = get_stack_pointer.i64 
;;     store notrap aligned region3 v8, v6+64
;;     v9 = get_exception_handler_address.i64 block1, 0
;;     store notrap aligned region4 v9, v6+80
;;     try_call fn0(v0, v1, v4, v5), sig0, block2(ret0, ret1), [ default: block3 ]
;;
;; block2(v10: i32, v11: i64):
;;     store notrap little region0 v10, v2
;;     store notrap little region0 v11, v2+16
;;     v12 = iconst.i8 1
;;     return v12  ; v12 = 1
;;
;; block3:
;;     v13 = iconst.i64 1
;;     store notrap aligned region5 v13, v6+136  ; v13 = 1
;;     v14 = iconst.i8 0
;;     return v14  ; v14 = 0
;; }
