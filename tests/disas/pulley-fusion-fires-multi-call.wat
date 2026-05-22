;;! target = "pulley64"
;;! test = "compile"

;; Multiple call_indirect sites in the same function should each fuse
;; independently. The pre-pass scans every brif in every block; each
;; matching pattern marks its own pair of continuation loads as
;; absorbed. The lowering emits a separate FuncrefDispatch MachInst
;; at each brif.
;;
;; This test pins that the optimisation is per-call-site, not
;; per-function. A bug that misuses the pre-pass's `to_sink` list
;; (e.g. accidental dedup, missing one of two patterns) would show up
;; as one of the two dispatch tails reverting to unfused form.
;;
;; Reference precedent: ChakraCore #5915 ("setPrototypeOf does not
;; invalidate cached instanceof IC inside currently-executing
;; frame") — fused-op caches must be per-site, not per-function.

(module
  (table 3 3 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  (func (export "call_two") (param i32 i32) (result i32)
    local.get 0
    call_indirect (result i32)
    local.get 1
    call_indirect (result i32)
    i32.add)

  (elem (i32.const 0) func $f1 $f2 $f3))
;; wasm[0]::function[0]::f1:
;;       push_frame
;;       xone x0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]::f2:
;;       push_frame
;;       xconst8 x0, 2
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[2]::f3:
;;       push_frame
;;       xconst8 x0, 3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[3]:
;;       push_frame_save 48, x16, x17, x18, x27, x28, x29
;;       xmov x29, x3
;;       br_if_xugteq32_u8 x2, 3, 0x95    // target = 0xae
;;   20: xload64le_o32 x28, x0, 48
;;       xmov x4, x0
;;       zext32 x1, x2
;;       xshl64_u6 x0, x1, 3
;;       xadd64 x0, x28, x0
;;       xload64le_o32 x0, x0, 0
;;       xband_funcref_dispatch_not_x64 x0, x17, x16, x0, 8, 24, 0x50    // target = 0x8a
;;       xmov x18, x4
;;       call_indirect2 x17, x16, x18
;;       xmov x3, x29
;;       xmov x4, x18
;;       xmov x17, x0
;;       br_if_xugteq32_u8 x3, 3, 0x5c    // target = 0xb1
;;   5c: zext32 x1, x3
;;       xshl64_u6 x0, x1, 3
;;       xadd64 x0, x28, x0
;;       xload64le_o32 x0, x0, 0
;;       xband_funcref_dispatch_not_x64 x0, x27, x28, x0, 8, 24, 0x30    // target = 0x9c
;;       xmov x16, x4
;;       call_indirect2 x27, x28, x16
;;       xmov x1, x17
;;       xadd32 x0, x1, x0
;;       pop_frame_restore 48, x16, x17, x18, x27, x28, x29
;;       ret
;;   8a: xzero x0
;;   8c: xmov x18, x4
;;   8f: call3 x18, x0, x1, 0x28f    // target = 0x31e
;;   97: jump -0x4f    // target = 0x48
;;   9c: xzero x0
;;   9e: xmov x16, x4
;;   a1: call3 x16, x0, x1, 0x27d    // target = 0x31e
;;   a9: jump -0x2f    // target = 0x7a
;;   ae: trap
;;   b1: trap
