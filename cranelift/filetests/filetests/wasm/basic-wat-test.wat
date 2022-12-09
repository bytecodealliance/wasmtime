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
;;     heap0 = static gv1, min 0, bound 4096, offset_guard 0xffff_ffff, index_type i32
;;
;;                                 block0(v0: i32, v1: i32, v2: i64):
;; @0021                               v4 = heap_addr.i64 heap0, v0, 0, 4
;; @0021                               v5 = load.i32 little heap v4
;; @0026                               v6 = heap_addr.i64 heap0, v1, 0, 4
;; @0026                               v7 = load.i32 little heap v6
;; @0029                               v8 = iadd v5, v7
;; @002a                               jump block1(v8)
;;
;;                                 block1(v3: i32):
;; @002a                               return v3
;; }
