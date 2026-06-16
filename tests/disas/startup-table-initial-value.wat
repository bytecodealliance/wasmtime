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
;;     v5 = get_frame_pointer.i64 
;;     v4 = load.i64 notrap aligned readonly can_move region0 v0+8
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
;; block0(v0: i64, v1: i64):
;;     v9 = load.i64 notrap aligned v0+56
;;     v10 = ireduce.i32 v9
;;     v11 = uextend.i64 v10
;;     v39 = iconst.i64 10
;;     v51 = icmp ult v11, v39  ; v39 = 10
;;     trapnz v51, user6
;;     v18 = load.i64 notrap aligned v0+48
;;     v3 = iconst.i32 1
;;     v81 = iconst.i64 36
;;     v83 = iadd v18, v81  ; v81 = 36
;;     v20 = iconst.i64 4
;;     jump block1(v18)
;;
;; block1(v29: i64):
;;     v86 = iconst.i32 1
;;     store notrap aligned v86, v29  ; v86 = 1
;;     v87 = iadd.i64 v18, v81  ; v81 = 36
;;     v88 = icmp eq v29, v87
;;     v89 = iconst.i64 4
;;     v90 = iadd v29, v89  ; v89 = 4
;;     brif v88, block2, block1(v90)
;;
;; block2:
;;     return
;; }
