;;! target = "x86_64"
;;!
;;! optimize = true
;;!
;;! settings = [
;;!   "enable_heap_access_spectre_mitigation=false",
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
;; @0047                               v5 = uextend.i64 v0
;; @0047                               v7 = icmp ugt v5, v6
;; @0047                               brif v7, block2, block3
;;
;;                                 block2 cold:
;; @0047                               trap heap_oob
;;
;;                                 block3:
;; @0047                               v8 = load.i64 notrap aligned v1
;; @0047                               v9 = iadd v8, v5
;; @0047                               v10 = load.i32 little heap v9
;;                                     v2 -> v10
;; @004c                               brif.i8 v7, block4, block5
;;
;;                                 block4 cold:
;; @004c                               trap heap_oob
;;
;;                                 block5:
;;                                     v27 = iconst.i64 4
;; @004c                               v16 = iadd.i64 v9, v27  ; v27 = 4
;; @004c                               v17 = load.i32 little heap v16
;;                                     v3 -> v17
;; @0051                               v19 = iconst.i64 0x0010_0003
;; @0051                               v20 = uadd_overflow_trap.i64 v5, v19, heap_oob  ; v19 = 0x0010_0003
;; @0051                               v22 = icmp ugt v20, v6
;; @0051                               brif v22, block6, block7
;;
;;                                 block6 cold:
;; @0051                               trap heap_oob
;;
;;                                 block7:
;; @0051                               v23 = load.i64 notrap aligned v1
;; @0051                               v24 = iadd v23, v5
;;                                     v28 = iconst.i64 0x000f_ffff
;; @0051                               v25 = iadd v24, v28  ; v28 = 0x000f_ffff
;; @0051                               v26 = load.i32 little heap v25
;;                                     v4 -> v26
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v10, v17, v26
;; }
;;
;; function u0:1(i32, i32, i32, i32, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+8
;;     gv2 = load.i64 notrap aligned gv0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32, v4: i64):
;; @005d                               v6 = load.i64 notrap aligned v4+8
;; @005d                               v5 = uextend.i64 v0
;; @005d                               v7 = icmp ugt v5, v6
;; @005d                               brif v7, block2, block3
;;
;;                                 block2 cold:
;; @005d                               trap heap_oob
;;
;;                                 block3:
;; @005d                               v8 = load.i64 notrap aligned v4
;; @005d                               v9 = iadd v8, v5
;; @005d                               store.i32 little heap v1, v9
;; @0064                               brif.i8 v7, block4, block5
;;
;;                                 block4 cold:
;; @0064                               trap heap_oob
;;
;;                                 block5:
;;                                     v24 = iconst.i64 4
;; @0064                               v15 = iadd.i64 v9, v24  ; v24 = 4
;; @0064                               store.i32 little heap v2, v15
;; @006b                               v17 = iconst.i64 0x0010_0003
;; @006b                               v18 = uadd_overflow_trap.i64 v5, v17, heap_oob  ; v17 = 0x0010_0003
;; @006b                               v20 = icmp ugt v18, v6
;; @006b                               brif v20, block6, block7
;;
;;                                 block6 cold:
;; @006b                               trap heap_oob
;;
;;                                 block7:
;; @006b                               v21 = load.i64 notrap aligned v4
;; @006b                               v22 = iadd v21, v5
;;                                     v25 = iconst.i64 0x000f_ffff
;; @006b                               v23 = iadd v22, v25  ; v25 = 0x000f_ffff
;; @006b                               store.i32 little heap v3, v23
;; @0070                               jump block1
;;
;;                                 block1:
;; @0070                               return
;; }
