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
;;! load = { base = "vmctx", offset = 0, readonly = true }
;;!
;;! [[heaps]]
;;! base = "heap_base"
;;! min_size = 0x10000
;;! offset_guard_size = 0xffffffff
;;! index_type = "i32"
;;! style = { kind = "static", bound = 0x10000000 }

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
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0057                               v5 = load.i64 notrap aligned readonly v1
;; @0057                               v4 = uextend.i64 v0
;; @0057                               v6 = iadd v5, v4
;; @0057                               v7 = load.i32 little heap v6
;;                                     v2 -> v7
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v7, v7
;; }
;;
;; function u0:1(i32, i64 vmctx) -> i32, i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0064                               v5 = load.i64 notrap aligned readonly v1
;; @0064                               v4 = uextend.i64 v0
;; @0064                               v6 = iadd v5, v4
;;                                     v14 = iconst.i64 1234
;; @0064                               v7 = iadd v6, v14  ; v14 = 1234
;; @0064                               v8 = load.i32 little heap v7
;;                                     v2 -> v8
;; @006e                               jump block1
;;
;;                                 block1:
;; @006e                               return v8, v8
;; }
