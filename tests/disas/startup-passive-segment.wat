;;! target = 'x86_64'
;;! test = 'optimize'
;;! filter = 'module_start'
;;! flags = '-Wgc'

(module
  (elem (ref i31) (item (ref.i31 (i32.const 0))) (item (ref.i31 (i32.const 1))))
)
;; function u2415919104:1(i64 vmctx, i64, i64, i64) -> i8 system_v {
;;     sig0 = (i64 vmctx, i64) tail
;;     fn0 = colocated u2415919104:0 sig0
;;
;; block0(v0: i64, v1: i64, v2: i64, v3: i64):
;;     jump block1
;;
;; block1:
;;     v4 = load.i64 notrap aligned v0+8
;;     v5 = get_frame_pointer.i64 
;;     store notrap aligned v5, v4+72
;;     v6 = get_stack_pointer.i64 
;;     store notrap aligned v6, v4+64
;;     v7 = get_exception_handler_address.i64 block1, 0
;;     store notrap aligned v7, v4+80
;;     try_call fn0(v0, v1), sig0, block2, [ default: block3 ]
;;
;; block2:
;;     v8 = iconst.i8 1
;;     return v8  ; v8 = 1
;;
;; block3:
;;     v9 = iconst.i8 0
;;     return v9  ; v9 = 0
;; }
;;
;; function u2415919104:0(i64 vmctx, i64) tail {
;;     region0 = 2147483648 "GcHeap"
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u805306368:4 sig0
;;
;; block0(v0: i64, v1: i64):
;;     v2 = iconst.i32 0
;;     v3 = call fn0(v0, v2)  ; v2 = 0
;;     v5 = iconst.i32 1
;;     store user2 little region0 v5, v3  ; v5 = 1
;;     v24 = iconst.i32 3
;;     v16 = iconst.i64 16
;;     v17 = iadd v3, v16  ; v16 = 16
;;     store user2 little region0 v24, v17  ; v24 = 3
;;     return
;; }
