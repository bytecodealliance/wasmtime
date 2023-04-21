;;! target = "x86_64"
;;!
;;! optimize = true
;;!
;;! settings = [
;;!   "enable_heap_access_spectre_mitigation=true",
;;!   "opt_level=speed_and_size",
;;! ]
;;!
;;! [globals.vmctx]
;;! type = "i64"
;;! vmctx = true
;;!
;;! [globals.heap_base]
;;! type = "i64"
;;! load = { base = "vmctx", offset = 0 }
;;!
;;! [globals.heap_bound]
;;! type = "i64"
;;! load = { base = "vmctx", offset = 8 }
;;!
;;! [[heaps]]
;;! base = "heap_base"
;;! min_size = 0
;;! offset_guard_size = 0x0000ffff
;;! index_type = "i32"
;;! style = { kind = "dynamic", bound = "heap_bound" }

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

;; function u0:0(i32, i64 vmctx) -> i32, i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+8
;;     gv2 = load.i64 notrap aligned gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0047                               v6 = load.i64 notrap aligned v1+8
;; @0047                               v8 = load.i64 notrap aligned v1
;; @0047                               v5 = uextend.i64 v0
;; @0047                               v7 = icmp ugt v5, v6
;; @0047                               v10 = iconst.i64 0
;; @0047                               v9 = iadd v8, v5
;; @0047                               v11 = select_spectre_guard v7, v10, v9  ; v10 = 0
;; @0047                               v12 = load.i32 little heap v11
;;                                     v2 -> v12
;;                                     v33 = iconst.i64 4
;; @004c                               v18 = iadd v9, v33  ; v33 = 4
;; @004c                               v20 = select_spectre_guard v7, v10, v18  ; v10 = 0
;; @004c                               v21 = load.i32 little heap v20
;;                                     v3 -> v21
;; @0051                               v23 = iconst.i64 0x0010_0003
;; @0051                               v24 = uadd_overflow_trap v5, v23, heap_oob  ; v23 = 0x0010_0003
;; @0051                               v26 = icmp ugt v24, v6
;;                                     v34 = iconst.i64 0x000f_ffff
;; @0051                               v29 = iadd v9, v34  ; v34 = 0x000f_ffff
;; @0051                               v31 = select_spectre_guard v26, v10, v29  ; v10 = 0
;; @0051                               v32 = load.i32 little heap v31
;;                                     v4 -> v32
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v12, v21, v32
;; }
;;
;; function u0:1(i32, i32, i32, i32, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+8
;;     gv2 = load.i64 notrap aligned gv0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32, v4: i64):
;; @005d                               v6 = load.i64 notrap aligned v4+8
;; @005d                               v8 = load.i64 notrap aligned v4
;; @005d                               v5 = uextend.i64 v0
;; @005d                               v7 = icmp ugt v5, v6
;; @005d                               v10 = iconst.i64 0
;; @005d                               v9 = iadd v8, v5
;; @005d                               v11 = select_spectre_guard v7, v10, v9  ; v10 = 0
;; @005d                               store little heap v1, v11
;;                                     v30 = iconst.i64 4
;; @0064                               v17 = iadd v9, v30  ; v30 = 4
;; @0064                               v19 = select_spectre_guard v7, v10, v17  ; v10 = 0
;; @0064                               store little heap v2, v19
;; @006b                               v21 = iconst.i64 0x0010_0003
;; @006b                               v22 = uadd_overflow_trap v5, v21, heap_oob  ; v21 = 0x0010_0003
;; @006b                               v24 = icmp ugt v22, v6
;;                                     v31 = iconst.i64 0x000f_ffff
;; @006b                               v27 = iadd v9, v31  ; v31 = 0x000f_ffff
;; @006b                               v29 = select_spectre_guard v24, v10, v27  ; v10 = 0
;; @006b                               store little heap v3, v29
;; @0070                               jump block1
;;
;;                                 block1:
;; @0070                               return
;; }
