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
;;     region0 = 21 ""
;;     region1 = 123 ""
;;     region2 = 231 ""
;;     region3 = 243 ""
;;     region4 = 209 ""
;;     region5 = 68 ""
;;     sig0 = (i64 vmctx, i64, i32, i64) -> i32, i64 tail
;;     fn0 = colocated u0:0 sig0
;;
;; block0(v0: i64, v1: i64, v2: i64, v3: i64):
;;     jump block1
;;
;; block1:
;;     v4 = iconst.i64 2
;;     v5 = icmp.i64 uge v3, v4  ; v4 = 2
;;     trapz v5, user1
;;     v6 = load.i32 notrap little region0 v2
;;     v7 = load.i64 notrap little region0 v2+16
;;     v9 = get_frame_pointer.i64 
;;     v8 = load.i64 notrap aligned readonly can_move region1 v0+8
;;     store notrap aligned region2 v9, v8+72
;;     v10 = get_stack_pointer.i64 
;;     store notrap aligned region3 v10, v8+64
;;     v11 = get_exception_handler_address.i64 block1, 0
;;     store notrap aligned region4 v11, v8+80
;;     try_call fn0(v0, v1, v6, v7), sig0, block2(ret0, ret1), [ default: block3 ]
;;
;; block2(v12: i32, v13: i64):
;;     store notrap little region0 v12, v2
;;     store notrap little region0 v13, v2+16
;;     v14 = iconst.i8 1
;;     return v14  ; v14 = 1
;;
;; block3:
;;     v15 = iconst.i64 1
;;     store notrap aligned region5 v15, v8+136  ; v15 = 1
;;     v16 = iconst.i8 0
;;     return v16  ; v16 = 0
;; }
