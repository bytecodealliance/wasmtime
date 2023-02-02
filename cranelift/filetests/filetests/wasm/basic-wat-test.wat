;;! target = "x86_64"
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
;;! min_size = 0
;;! offset_guard_size = 0xFFFFFFFF
;;! index_type = "i32"
;;! style = { kind = "static", bound = 0x1000 }

(module
  (memory 0)
  (func (param i32 i32) (result i32)
    local.get 0
    i32.load
    local.get 1
    i32.load
    i32.add))

;; function u0:0(i32, i32, i64 vmctx) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64):
;; @0021                               v4 = uextend.i64 v0
;; @0021                               v5 = global_value.i64 gv1
;; @0021                               v6 = iadd v5, v4
;; @0021                               v7 = load.i32 little heap v6
;; @0026                               v8 = uextend.i64 v1
;; @0026                               v9 = global_value.i64 gv1
;; @0026                               v10 = iadd v9, v8
;; @0026                               v11 = load.i32 little heap v10
;; @0029                               v12 = iadd v7, v11
;; @002a                               jump block1(v12)
;;
;;                                 block1(v3: i32):
;; @002a                               return v3
;; }