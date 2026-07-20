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
;;     region1 = 67108936 "VMStoreContext+0x48"
;;     region2 = 67108928 "VMStoreContext+0x40"
;;     region3 = 67108944 "VMStoreContext+0x50"
;;     region4 = 67109000 "VMStoreContext+0x88"
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
;;     region0 = 671088640 "VMTableDefinition+0x0"
;;     region1 = 335544320 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;
;; block0(v0: i64, v1: i64):
;;     v96 = iconst.i32 21
;;     v12 = load.i64 notrap aligned readonly can_move region0 v0+48
;;     v75 = iconst.i64 4
;;     v16 = iadd v12, v75  ; v75 = 4
;;     store user6 aligned region1 v96, v16  ; v96 = 21
;;     v113 = iconst.i32 23
;;     v130 = iconst.i64 8
;;     v46 = iadd v12, v130  ; v130 = 8
;;     store user6 aligned region1 v113, v46  ; v113 = 23
;;     v132 = iconst.i32 25
;;     v148 = iconst.i64 12
;;     v62 = iadd v12, v148  ; v148 = 12
;;     store user6 aligned region1 v132, v62  ; v132 = 25
;;     return
;; }
