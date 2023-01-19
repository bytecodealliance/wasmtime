;;! target = "x86_64"
;;!
;;! optimize = true
;;!
;;! settings = [
;;!   "enable_heap_access_spectre_mitigation=true",
;;!   "opt_level=speed_and_size",
;;!   "use_egraphs=true"
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
;; @0057                               v5 = iconst.i64 4
;; @0057                               v6 = uadd_overflow_trap v4, v5, heap_oob  ; v5 = 4
;; @0057                               v7 = load.i64 notrap aligned v1+8
;; @0057                               v8 = load.i64 notrap aligned v1
;; @0057                               v11 = icmp ugt v6, v7
;; @0057                               v10 = iconst.i64 0
;; @0057                               v9 = iadd v8, v4
;; @0057                               v12 = select_spectre_guard v11, v10, v9  ; v10 = 0
;; @0057                               v13 = load.i32 little heap v12
;;                                     v2 -> v13
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v13, v13
;; }
;;
;; function u0:1(i32, i64 vmctx) -> i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+8
;;     gv2 = load.i64 notrap aligned gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0064                               v4 = uextend.i64 v0
;; @0064                               v5 = iconst.i64 1238
;; @0064                               v6 = uadd_overflow_trap v4, v5, heap_oob  ; v5 = 1238
;; @0064                               v7 = load.i64 notrap aligned v1+8
;; @0064                               v8 = load.i64 notrap aligned v1
;; @0064                               v12 = icmp ugt v6, v7
;; @0064                               v11 = iconst.i64 0
;; @0064                               v9 = iadd v8, v4
;;                                     v26 = iconst.i64 1234
;; @0064                               v10 = iadd v9, v26  ; v26 = 1234
;; @0064                               v13 = select_spectre_guard v12, v11, v10  ; v11 = 0
;; @0064                               v14 = load.i32 little heap v13
;;                                     v2 -> v14
;; @006e                               jump block1
;;
;;                                 block1:
;; @006e                               return v14, v14
;; }
