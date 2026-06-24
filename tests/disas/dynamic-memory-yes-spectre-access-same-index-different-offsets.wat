;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O static-memory-maximum-size=0 -O dynamic-memory-guard-size=0xffff"

(module
  (memory (export "memory") 0)

  (func (export "loads") (param i32) (result i32 i32 i32)
    ;; Within the guard region.
    local.get 0
    i32.load offset=0
    ;; Also within the guard region, bounds check should GVN with previous.
    local.get 0
    i32.load offset=4
    ;; Outside the guard region, needs additional bounds checks.
    local.get 0
    i32.load offset=0x000fffff
  )

  ;; Same as above, but for stores.
  (func (export "stores") (param i32 i32 i32 i32)
    local.get 0
    local.get 1
    i32.store offset=0
    local.get 0
    local.get 2
    i32.store offset=4
    local.get 0
    local.get 3
    i32.store offset=0x000fffff
  )
)

;; function u0:0(i64 vmctx, i64, i32) -> i32, i32, i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     region4 = 536870912 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0047                               v4 = load.i64 notrap aligned region3 v0+64
;; @0047                               v6 = load.i64 notrap aligned can_move region2 v0+56
;; @0047                               v3 = uextend.i64 v2
;; @0047                               v5 = icmp ugt v3, v4
;; @0047                               v8 = iconst.i64 0
;; @0047                               v7 = iadd v6, v3
;; @0047                               v9 = select_spectre_guard v5, v8, v7  ; v8 = 0
;; @0047                               v10 = load.i32 little region4 v9
;; @004c                               v16 = iconst.i64 4
;; @004c                               v17 = iadd v7, v16  ; v16 = 4
;; @004c                               v19 = select_spectre_guard v5, v8, v17  ; v8 = 0
;; @004c                               v20 = load.i32 little region4 v19
;; @0051                               v22 = iconst.i64 0x0010_0003
;; @0051                               v23 = uadd_overflow_trap v3, v22, heap_oob  ; v22 = 0x0010_0003
;; @0051                               v25 = icmp ugt v23, v4
;; @0051                               v28 = iconst.i64 0x000f_ffff
;; @0051                               v29 = iadd v7, v28  ; v28 = 0x000f_ffff
;; @0051                               v31 = select_spectre_guard v25, v8, v29  ; v8 = 0
;; @0051                               v32 = load.i32 little region4 v31
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v10, v20, v32
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     region4 = 536870912 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @005d                               v7 = load.i64 notrap aligned region3 v0+64
;; @005d                               v9 = load.i64 notrap aligned can_move region2 v0+56
;; @005d                               v6 = uextend.i64 v2
;; @005d                               v8 = icmp ugt v6, v7
;; @005d                               v11 = iconst.i64 0
;; @005d                               v10 = iadd v9, v6
;; @005d                               v12 = select_spectre_guard v8, v11, v10  ; v11 = 0
;; @005d                               store little region4 v3, v12
;; @0064                               v18 = iconst.i64 4
;; @0064                               v19 = iadd v10, v18  ; v18 = 4
;; @0064                               v21 = select_spectre_guard v8, v11, v19  ; v11 = 0
;; @0064                               store little region4 v4, v21
;; @006b                               v23 = iconst.i64 0x0010_0003
;; @006b                               v24 = uadd_overflow_trap v6, v23, heap_oob  ; v23 = 0x0010_0003
;; @006b                               v26 = icmp ugt v24, v7
;; @006b                               v29 = iconst.i64 0x000f_ffff
;; @006b                               v30 = iadd v10, v29  ; v29 = 0x000f_ffff
;; @006b                               v32 = select_spectre_guard v26, v11, v30  ; v11 = 0
;; @006b                               store little region4 v5, v32
;; @0070                               jump block1
;;
;;                                 block1:
;; @0070                               return
;; }
