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
;; @0057                               v5 = load.i64 notrap aligned v1+8
;; @0057                               v7 = load.i64 notrap aligned v1
;; @0057                               v4 = uextend.i64 v0
;; @0057                               v6 = icmp ugt v4, v5
;; @0057                               v9 = iconst.i64 0
;; @0057                               v8 = iadd v7, v4
;; @0057                               v10 = select_spectre_guard v6, v9, v8  ; v9 = 0
;; @0057                               v11 = load.i32 little heap v10
;;                                     v2 -> v11
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
;; @0064                               v5 = load.i64 notrap aligned v1+8
;; @0064                               v7 = load.i64 notrap aligned v1
;; @0064                               v4 = uextend.i64 v0
;; @0064                               v6 = icmp ugt v4, v5
;; @0064                               v10 = iconst.i64 0
;; @0064                               v8 = iadd v7, v4
;;                                     v22 = iconst.i64 1234
;; @0064                               v9 = iadd v8, v22  ; v22 = 1234
;; @0064                               v11 = select_spectre_guard v6, v10, v9  ; v10 = 0
;; @0064                               v12 = load.i32 little heap v11
;;                                     v2 -> v12
;; @006e                               jump block1
;;
;;                                 block1:
;; @006e                               return v12, v12
;; }
