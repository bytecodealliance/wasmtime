;;! target = 'x86_64'
;;! test = 'optimize'
;;! filter = 'module_start'
;;! flags = '-Wgc -Wfunction-references'

(module
  (table 10 anyref)

  (elem (i32.const 1) (ref i31)
    (item (ref.i31 (i32.const 10)))
    (item (ref.i31 (i32.const 11)))
    (item (ref.i31 (i32.const 12)))
  )
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
;;     region0 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+48
;;     gv2 = load.i64 notrap aligned gv0+56
;;
;; block0(v0: i64, v1: i64):
;;     v4 = load.i64 notrap aligned v0+56
;;     v5 = ireduce.i32 v4
;;     v6 = uextend.i64 v5
;;     v86 = iconst.i64 4
;;     v92 = icmp ult v6, v86  ; v86 = 4
;;     trapnz v92, user6
;;     v13 = load.i64 notrap aligned v0+48
;;     v103 = iconst.i32 21
;;     v2 = iconst.i32 1
;;     v114 = icmp ule v5, v2  ; v2 = 1
;;     v79 = iconst.i64 0
;;     v17 = iadd v13, v86  ; v86 = 4
;;     v34 = select_spectre_guard v114, v79, v17  ; v79 = 0
;;     store user6 aligned region0 v103, v34  ; v103 = 21
;;     v117 = iconst.i32 23
;;     v123 = iconst.i32 2
;;     v129 = icmp ule v5, v123  ; v123 = 2
;;     v131 = iconst.i64 8
;;     v49 = iadd v13, v131  ; v131 = 8
;;     v51 = select_spectre_guard v129, v79, v49  ; v79 = 0
;;     store user6 aligned region0 v117, v51  ; v117 = 23
;;     v133 = iconst.i32 25
;;     v3 = iconst.i32 3
;;     v144 = icmp ule v5, v3  ; v3 = 3
;;     v146 = iconst.i64 12
;;     v66 = iadd v13, v146  ; v146 = 12
;;     v68 = select_spectre_guard v144, v79, v66  ; v79 = 0
;;     store user6 aligned region0 v133, v68  ; v133 = 25
;;     return
;; }
