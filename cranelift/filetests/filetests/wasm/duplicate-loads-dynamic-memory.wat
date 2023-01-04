;;! target = "x86_64"
;;!
;;! optimize = true
;;!
;;! settings = [
;;!   "enable_heap_access_spectre_mitigation=true",
;;!   "opt_level=speed_and_size"
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
;;! min_size = 0x10000
;;! offset_guard_size = 0xffffffff
;;! index_type = "i32"
;;! style = { kind = "dynamic", bound = "heap_bound" }

(module
  (memory (export "memory") 1)
  (func (export "load-without-offset") (param i32) (result i32 i32)
    local.get 0
    i32.load
    local.get 0
    i32.load
  )
  (func (export "load-with-offset") (param i32) (result i32 i32)
    local.get 0
    i32.load offset=1234
    local.get 0
    i32.load offset=1234
  )
)

;; function u0:0(i32, i64 vmctx) -> i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+8
;;     gv2 = load.i64 notrap aligned gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0057                               v4 = uextend.i64 v0
;;                                     v13 -> v4
;; @0057                               v5 = load.i64 notrap aligned v1+8
;;                                     v14 -> v5
;;                                     v22 = iconst.i64 -4
;;                                     v23 -> v22
;; @0057                               v6 = iadd v5, v22  ; v22 = -4
;;                                     v15 -> v6
;; @0057                               v7 = load.i64 notrap aligned v1
;;                                     v16 -> v7
;; @0057                               v8 = iadd v7, v4
;;                                     v17 -> v8
;; @0057                               v9 = iconst.i64 0
;;                                     v18 -> v9
;; @0057                               v10 = icmp ugt v4, v6
;;                                     v19 -> v10
;; @0057                               v11 = select_spectre_guard v10, v9, v8  ; v9 = 0
;;                                     v20 -> v11
;; @0057                               v12 = load.i32 little heap v11
;;                                     v2 -> v12
;; @005c                               v21 = load.i32 little heap v11
;;                                     v3 -> v21
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v12, v21
;; }
;;
;; function u0:1(i32, i64 vmctx) -> i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+8
;;     gv2 = load.i64 notrap aligned gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0064                               v4 = uextend.i64 v0
;;                                     v14 -> v4
;; @0064                               v5 = load.i64 notrap aligned v1+8
;;                                     v15 -> v5
;;                                     v24 = iconst.i64 -1238
;;                                     v26 -> v24
;; @0064                               v6 = iadd v5, v24  ; v24 = -1238
;;                                     v16 -> v6
;; @0064                               v7 = load.i64 notrap aligned v1
;;                                     v17 -> v7
;; @0064                               v8 = iadd v7, v4
;;                                     v18 -> v8
;;                                     v25 = iconst.i64 1234
;;                                     v27 -> v25
;; @0064                               v9 = iadd v8, v25  ; v25 = 1234
;;                                     v19 -> v9
;; @0064                               v10 = iconst.i64 0
;;                                     v20 -> v10
;; @0064                               v11 = icmp ugt v4, v6
;;                                     v21 -> v11
;; @0064                               v12 = select_spectre_guard v11, v10, v9  ; v10 = 0
;;                                     v22 -> v12
;; @0064                               v13 = load.i32 little heap v12
;;                                     v2 -> v13
;; @006a                               v23 = load.i32 little heap v12
;;                                     v3 -> v23
;; @006e                               jump block1
;;
;;                                 block1:
;; @006e                               return v13, v23
;; }
