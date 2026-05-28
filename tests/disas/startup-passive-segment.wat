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
;;     gv0 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u805306368:4 sig0
;;
;; block0(v0: i64, v1: i64):
;;     v3 = iconst.i32 0
;;     v4 = call fn0(v0, v3)  ; v3 = 0
;;     v18 = iconst.i32 1
;;     store user2 little v18, v4  ; v18 = 1
;;     v25 = iconst.i32 3
;;     v13 = iconst.i64 16
;;     v12 = iadd v4, v13  ; v13 = 16
;;     store user2 little v25, v12  ; v25 = 3
;;     return
;; }
