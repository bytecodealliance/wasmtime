;;! target = "x86_64"
;;!
;;! optimize = true
;;!
;;! settings = [
;;!   "enable_heap_access_spectre_mitigation=true",
;;!   "opt_level=speed_and_size",
;;!   "use_egraphs=false"
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
;;! offset_guard_size = 0xffffffff
;;! index_type = "i32"
;;! style = { kind = "dynamic", bound = "heap_bound" }

(module
  (memory (export "memory") 0)
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
;;                                     v12 -> v4
;; @0057                               v5 = load.i64 notrap aligned v1+8
;;                                     v13 -> v5
;; @0057                               v6 = icmp ugt v4, v5
;;                                     v14 -> v6
;; @0057                               v7 = load.i64 notrap aligned v1
;;                                     v15 -> v7
;; @0057                               v8 = iadd v7, v4
;;                                     v16 -> v8
;; @0057                               v9 = iconst.i64 0
;;                                     v17 -> v9
;; @0057                               v10 = select_spectre_guard v6, v9, v8  ; v9 = 0
;;                                     v18 -> v10
;; @0057                               v11 = load.i32 little heap v10
;;                                     v2 -> v11
;;                                     v19 -> v11
;;                                     v3 -> v19
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v11, v11
;; }
;;
;; function u0:1(i32, i64 vmctx) -> i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+8
;;     gv2 = load.i64 notrap aligned gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0064                               v4 = uextend.i64 v0
;;                                     v13 -> v4
;; @0064                               v5 = load.i64 notrap aligned v1+8
;;                                     v14 -> v5
;; @0064                               v6 = icmp ugt v4, v5
;;                                     v15 -> v6
;; @0064                               v7 = load.i64 notrap aligned v1
;;                                     v16 -> v7
;; @0064                               v8 = iadd v7, v4
;;                                     v17 -> v8
;;                                     v22 = iconst.i64 1234
;;                                     v23 -> v22
;; @0064                               v9 = iadd v8, v22  ; v22 = 1234
;;                                     v18 -> v9
;; @0064                               v10 = iconst.i64 0
;;                                     v19 -> v10
;; @0064                               v11 = select_spectre_guard v6, v10, v9  ; v10 = 0
;;                                     v20 -> v11
;; @0064                               v12 = load.i32 little heap v11
;;                                     v2 -> v12
;;                                     v21 -> v12
;;                                     v3 -> v21
;; @006e                               jump block1
;;
;;                                 block1:
;; @006e                               return v12, v12
;; }
