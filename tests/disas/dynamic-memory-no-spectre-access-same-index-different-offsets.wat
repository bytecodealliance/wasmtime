;;! target = "x86_64"
;;! test = "optimize"
;;! flags = [
;;!   "-Ccranelift-enable-heap-access-spectre-mitigation=false",
;;!   "-Ostatic-memory-maximum-size=0",
;;!   "-Odynamic-memory-guard-size=0xffff",
;;! ]

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

;; function u0:0(i64 vmctx, i64, i32) -> i32, i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0047                               v7 = load.i64 notrap aligned v0+104
;; @0047                               v6 = uextend.i64 v2
;; @0047                               v8 = icmp ugt v6, v7
;; @0047                               brif v8, block2, block3
;;
;;                                 block2 cold:
;; @0047                               trap heap_oob
;;
;;                                 block3:
;; @0047                               v9 = load.i64 notrap aligned checked v0+96
;; @0047                               v10 = iadd v9, v6
;; @0047                               v11 = load.i32 little heap v10
;; @004c                               brif.i8 v8, block4, block5
;;
;;                                 block4 cold:
;; @004c                               trap heap_oob
;;
;;                                 block5:
;; @004c                               v17 = iconst.i64 4
;; @004c                               v18 = iadd.i64 v10, v17  ; v17 = 4
;; @004c                               v19 = load.i32 little heap v18
;; @0051                               v21 = iconst.i64 0x0010_0003
;; @0051                               v22 = uadd_overflow_trap.i64 v6, v21, heap_oob  ; v21 = 0x0010_0003
;; @0051                               v24 = icmp ugt v22, v7
;; @0051                               brif v24, block6, block7
;;
;;                                 block6 cold:
;; @0051                               trap heap_oob
;;
;;                                 block7:
;; @0051                               v25 = load.i64 notrap aligned checked v0+96
;; @0051                               v26 = iadd v25, v6
;; @0051                               v27 = iconst.i64 0x000f_ffff
;; @0051                               v28 = iadd v26, v27  ; v27 = 0x000f_ffff
;; @0051                               v29 = load.i32 little heap v28
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v11, v19, v29
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32, i32) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @005d                               v7 = load.i64 notrap aligned v0+104
;; @005d                               v6 = uextend.i64 v2
;; @005d                               v8 = icmp ugt v6, v7
;; @005d                               brif v8, block2, block3
;;
;;                                 block2 cold:
;; @005d                               trap heap_oob
;;
;;                                 block3:
;; @005d                               v9 = load.i64 notrap aligned checked v0+96
;; @005d                               v10 = iadd v9, v6
;; @005d                               store.i32 little heap v3, v10
;; @0064                               brif.i8 v8, block4, block5
;;
;;                                 block4 cold:
;; @0064                               trap heap_oob
;;
;;                                 block5:
;; @0064                               v16 = iconst.i64 4
;; @0064                               v17 = iadd.i64 v10, v16  ; v16 = 4
;; @0064                               store.i32 little heap v4, v17
;; @006b                               v19 = iconst.i64 0x0010_0003
;; @006b                               v20 = uadd_overflow_trap.i64 v6, v19, heap_oob  ; v19 = 0x0010_0003
;; @006b                               v22 = icmp ugt v20, v7
;; @006b                               brif v22, block6, block7
;;
;;                                 block6 cold:
;; @006b                               trap heap_oob
;;
;;                                 block7:
;; @006b                               v23 = load.i64 notrap aligned checked v0+96
;; @006b                               v24 = iadd v23, v6
;; @006b                               v25 = iconst.i64 0x000f_ffff
;; @006b                               v26 = iadd v24, v25  ; v25 = 0x000f_ffff
;; @006b                               store.i32 little heap v5, v26
;; @0070                               jump block1
;;
;;                                 block1:
;; @0070                               return
;; }
