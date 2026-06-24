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
;;     region0 = 2684354560 "VMTableDefinition+0x0"
;;     region1 = 2684354568 "VMTableDefinition+0x8"
;;     region2 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;
;; block0(v0: i64, v1: i64):
;;     v4 = load.i64 notrap aligned region1 v0+56
;;     v5 = ireduce.i32 v4
;;     v6 = uextend.i64 v5
;;     v78 = iconst.i64 4
;;     v84 = icmp ult v6, v78  ; v78 = 4
;;     trapnz v84, user6
;;     v13 = load.i64 notrap aligned region0 v0+48
;;     v95 = iconst.i32 21
;;     v2 = iconst.i32 1
;;     v106 = icmp ule v5, v2  ; v2 = 1
;;     v71 = iconst.i64 0
;;     v17 = iadd v13, v78  ; v78 = 4
;;     v34 = select_spectre_guard v106, v71, v17  ; v71 = 0
;;     store user6 aligned region2 v95, v34  ; v95 = 21
;;     v109 = iconst.i32 23
;;     v115 = iconst.i32 2
;;     v121 = icmp ule v5, v115  ; v115 = 2
;;     v123 = iconst.i64 8
;;     v49 = iadd v13, v123  ; v123 = 8
;;     v51 = select_spectre_guard v121, v71, v49  ; v71 = 0
;;     store user6 aligned region2 v109, v51  ; v109 = 23
;;     v125 = iconst.i32 25
;;     v3 = iconst.i32 3
;;     v136 = icmp ule v5, v3  ; v3 = 3
;;     v138 = iconst.i64 12
;;     v66 = iadd v13, v138  ; v138 = 12
;;     v68 = select_spectre_guard v136, v71, v66  ; v71 = 0
;;     store user6 aligned region2 v125, v68  ; v125 = 25
;;     return
;; }
