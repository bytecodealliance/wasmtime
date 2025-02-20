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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+96
;;     gv5 = load.i64 notrap aligned checked gv3+88
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0047                               v7 = load.i64 notrap aligned v0+96
;; @0047                               v9 = load.i64 notrap aligned checked v0+88
;; @0047                               v6 = uextend.i64 v2
;; @0047                               v8 = icmp ugt v6, v7
;; @0047                               v11 = iconst.i64 0
;; @0047                               v10 = iadd v9, v6
;; @0047                               v12 = select_spectre_guard v8, v11, v10  ; v11 = 0
;; @0047                               v13 = load.i32 little heap v12
;; @004c                               v19 = iconst.i64 4
;; @004c                               v20 = iadd v10, v19  ; v19 = 4
;; @004c                               v22 = select_spectre_guard v8, v11, v20  ; v11 = 0
;; @004c                               v23 = load.i32 little heap v22
;; @0051                               v25 = iconst.i64 0x0010_0003
;; @0051                               v26 = uadd_overflow_trap v6, v25, heap_oob  ; v25 = 0x0010_0003
;; @0051                               v28 = icmp ugt v26, v7
;; @0051                               v31 = iconst.i64 0x000f_ffff
;; @0051                               v32 = iadd v10, v31  ; v31 = 0x000f_ffff
;; @0051                               v34 = select_spectre_guard v28, v11, v32  ; v11 = 0
;; @0051                               v35 = load.i32 little heap v34
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v13, v23, v35
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+96
;;     gv5 = load.i64 notrap aligned checked gv3+88
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @005d                               v7 = load.i64 notrap aligned v0+96
;; @005d                               v9 = load.i64 notrap aligned checked v0+88
;; @005d                               v6 = uextend.i64 v2
;; @005d                               v8 = icmp ugt v6, v7
;; @005d                               v11 = iconst.i64 0
;; @005d                               v10 = iadd v9, v6
;; @005d                               v12 = select_spectre_guard v8, v11, v10  ; v11 = 0
;; @005d                               store little heap v3, v12
;; @0064                               v18 = iconst.i64 4
;; @0064                               v19 = iadd v10, v18  ; v18 = 4
;; @0064                               v21 = select_spectre_guard v8, v11, v19  ; v11 = 0
;; @0064                               store little heap v4, v21
;; @006b                               v23 = iconst.i64 0x0010_0003
;; @006b                               v24 = uadd_overflow_trap v6, v23, heap_oob  ; v23 = 0x0010_0003
;; @006b                               v26 = icmp ugt v24, v7
;; @006b                               v29 = iconst.i64 0x000f_ffff
;; @006b                               v30 = iadd v10, v29  ; v29 = 0x000f_ffff
;; @006b                               v32 = select_spectre_guard v26, v11, v30  ; v11 = 0
;; @006b                               store little heap v5, v32
;; @0070                               jump block1
;;
;;                                 block1:
;; @0070                               return
;; }
