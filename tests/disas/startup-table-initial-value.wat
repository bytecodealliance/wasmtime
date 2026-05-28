;;! target = 'x86_64'
;;! test = 'optimize'
;;! filter = 'module_start'
;;! flags = '-Wgc -Wfunction-references'

(module
  (table 10 (ref i31) (ref.i31 (i32.const 0)))
)
;; function u2415919104:1(i64 vmctx, i64, i64, i64) -> i8 system_v {
;;     region0 = 8 "VMContext+0x8"
;;     sig0 = (i64 vmctx, i64) tail
;;     fn0 = colocated u2415919104:0 sig0
;;
;; block0(v0: i64, v1: i64, v2: i64, v3: i64):
;;     jump block1
;;
;; block1:
;;     v4 = load.i64 notrap aligned region0 v0+8
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
;;     gv1 = load.i64 notrap aligned readonly can_move gv0+48
;;
;; block0(v0: i64, v1: i64):
;;     v17 = load.i64 notrap aligned readonly can_move v0+48
;;     v3 = iconst.i32 1
;;     v84 = iconst.i64 36
;;     v86 = iadd v17, v84  ; v84 = 36
;;     v19 = iconst.i64 4
;;     jump block1(v17)
;;
;; block1(v28: i64):
;;     v89 = iconst.i32 1
;;     store notrap aligned v89, v28  ; v89 = 1
;;     v90 = iadd.i64 v17, v84  ; v84 = 36
;;     v91 = icmp eq v28, v90
;;     v92 = iconst.i64 4
;;     v93 = iadd v28, v92  ; v92 = 4
;;     brif v91, block2, block1(v93)
;;
;; block2:
;;     return
;; }
