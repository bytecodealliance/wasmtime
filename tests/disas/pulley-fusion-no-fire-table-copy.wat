;;! target = "pulley64"
;;! test = "compile"

;; Phase 1 / phase 2 fusion gating: `table.copy` mutates the
;; destination table. With table 0 as the copy destination, its
;; immutability proof is cleared and the eager-init predicate becomes
;; false — fusion does not fire.
;;
;; Note that this only marks the DESTINATION as mutated; the source
;; table (table 1) keeps its proof. wasm-benchmark/`environ`'s
;; `table_mutability` test suite has the integration coverage for the
;; src-vs-dst marking; this filetest pins the lowering-level
;; consequence (Pulley dispatch tail is unfused for the dst table).
;;
;; wasm3 #547 (`op_CallIndirect` SEGV — missing bounds check on table
;; index) is a related precedent: bulk-copy invariants that fail
;; silently in one engine produce dispatch-time crashes in another.

(module
  (table $tdst 5 5 funcref)
  (table $tsrc 5 5 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  ;; Bulk mutator: clears the immutability proof for table $tdst.
  (func (export "copy") (param i32 i32 i32)
    local.get 0 local.get 1 local.get 2
    table.copy $tdst $tsrc)

  ;; Call through the (potentially-mutated) destination table.
  (func (export "call_dst") (param i32) (result i32)
    local.get 0
    call_indirect $tdst (result i32))

  ;; Call through the source table (still immutable from this
  ;; module's perspective; fusion CAN fire here).
  (func (export "call_src") (param i32) (result i32)
    local.get 0
    call_indirect $tsrc (result i32))

  (elem (table $tdst) (i32.const 0) func $f1 $f2 $f3)
  (elem (table $tsrc) (i32.const 0) func $f1 $f2 $f3))
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
;;       push_frame_save 64, x16, x17, x20, x21, x24, x26, x28
;;       zext32 x2, x2
;;       zext32 x1, x4
;;       xadd64 x5, x2, x1
;;       br_if_xugt64_u8 x5, 5, 0x109    // target = 0x128
;;   26: zext32 x5, x3
;;       xadd64 x6, x5, x1
;;       br_if_xugt64_u8 x6, 5, 0xff    // target = 0x12b
;;       br_if_not32 x4, 0xcf    // target = 0x102
;;   39: xload64le_o32 x6, x0, 48
;;       xshl64_u6 x2, x2, 3
;;       xadd64 x17, x6, x2
;;       xload64le_o32 x16, x0, 64
;;       xmov x6, x0
;;       xshl64_u6 x0, x5, 3
;;       xadd64 x20, x16, x0
;;       xshl64_u6 x0, x1, 3
;;       xadd64 x24, x17, x0
;;       xadd64 x26, x20, x0
;;       xadd32 x28, x3, x4
;;       xmov x0, x3
;;       br_if_xulteq64 x20, x17, 0x12    // target = 0x77
;;   6c: xmov x21, x6
;;       xmov x28, x0
;;       jump 0x50    // target = 0xc2
;;   77: xsub32_u8 x28, x28, 1
;;       br_if_xugteq32_u8 x28, 5, 0xb3    // target = 0x12e
;;   82: zext32 x1, x28
;;       xshl64_u6 x0, x1, 3
;;       xadd64 x0, x16, x0
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x72    // target = 0x108
;;   9d: xmov x21, x6
;;       xbor64_s8 x0, x0, 1
;;       xsub64_u8 x24, x24, 8
;;       xstore64le_o32 x24, 0, x0
;;       xsub64_u8 x26, x26, 8
;;       br_if_xeq64 x26, x20, 0x4f    // target = 0x102
;;   ba: xmov x6, x21
;;       jump -0x46    // target = 0x77
;;       br_if_xugteq32_u8 x28, 5, 0x6f    // target = 0x131
;;   c9: zext32 x2, x28
;;       xshl64_u6 x3, x2, 3
;;       xadd64 x3, x16, x3
;;       xload64le_o32 x3, x3, 0
;;       xband64_s8 x0, x3, -2
;;       br_if_xeq64_i8 x3, 0, 0x3d    // target = 0x11a
;;   e4: xbor64_s8 x5, x0, 1
;;       xstore64le_o32 x17, 0, x5
;;       xadd64_u8 x20, x20, 8
;;       xadd64_u8 x17, x17, 8
;;       xadd32_u8 x28, x28, 1
;;       br_if_xneq64 x20, x26, -0x39    // target = 0xc2
;;  102: pop_frame_restore 64, x16, x17, x20, x21, x24, x26, x28
;;       ret
;;  108: xone x0
;;  10a: xmov x21, x6
;;  10d: call3 x21, x0, x1, 0x4bf    // target = 0x5cc
;;  115: jump -0x75    // target = 0xa0
;;  11a: xone x4
;;  11c: call2 x21, x4, 0x4b0    // target = 0x5cc
;;  123: jump -0x3f    // target = 0xe4
;;  128: trap
;;  12b: trap
;;  12e: trap
;;  131: trap
;;
;; wasm[0]::function[4]:
;;       push_frame_save 16, x28
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 5, 0x7c    // target = 0x1b8
;;  143: xmov x1, x3
;;       xload64le_o32 x0, x1, 48
;;       zext32 x1, x2
;;       xshl64_u6 x2, x1, 3
;;       xadd64 x0, x0, x2
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x45    // target = 0x1a6
;;  168: xmov x28, x3
;;       br_if_xeq64_i8 x0, 0, 0x50    // target = 0x1bb
;;  172: xload32le_o32 x1, x0, 16
;;       xload64le_o32 x2, x28, 40
;;       xload32le_o32 x2, x2, 0
;;       br_if_xneq32 x1, x2, 0x37    // target = 0x1be
;;  18e: xload64le_o32 x1, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       call_indirect2 x1, x0, x28
;;       pop_frame_restore 16, x28
;;       ret
;;  1a6: xzero x0
;;  1a8: xmov x28, x3
;;  1ab: call3 x28, x0, x1, 0x421    // target = 0x5cc
;;  1b3: jump -0x48    // target = 0x16b
;;  1b8: trap
;;  1bb: trap
;;  1be: trap
;;
;; wasm[0]::function[5]:
;;       push_frame_save 16, x25
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 5, 0x60    // target = 0x229
;;  1d0: xmov x1, x3
;;       xload64le_o32 x0, x1, 64
;;       zext32 x15, x2
;;       xshl64_u6 x1, x15, 3
;;       xadd64 x0, x0, x1
;;       xload64le_o32 x1, x0, 0
;;       xband64_s8 x0, x1, -2
;;       br_if_xeq64_i8 x1, 0, 0x29    // target = 0x217
;;  1f5: xmov x25, x3
;;       br_if_xeq64_i8 x0, 0, 0x34    // target = 0x22c
;;  1ff: xload64le_o32 x1, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       call_indirect2 x1, x0, x25
;;       pop_frame_restore 16, x25
;;       ret
;;  217: xone x0
;;  219: xmov x25, x3
;;  21c: call3 x25, x0, x15, 0x3b0    // target = 0x5cc
;;  224: jump -0x2c    // target = 0x1f8
;;  229: trap
;;  22c: trap
