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
;;       push_frame
;;       xmov x15, x4
;;       xzero x10
;;       xone x11
;;       zext32 x12, x2
;;       zext32 x4, x3
;;       zext32 x5, x15
;;       call4 x0, x10, x11, x12, 0x49f    // target = 0x4c1
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[4]:
;;       push_frame_save 16, x29
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 5, 0x7a    // target = 0xaf
;;   3c: xload64le_o32 x0, x0, 48
;;       zext32 x1, x2
;;       xshl64_u6 x2, x1, 3
;;       xadd64 x0, x0, x2
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x46    // target = 0x9d
;;   5e: xmov x29, x3
;;       br_if_xeq64_i8 x0, 0, 0x51    // target = 0xb2
;;   68: xload32le_o32 x1, x0, 16
;;       xload64le_o32 x2, x29, 40
;;       xload32le_o32 x2, x2, 0
;;       br_if_xneq32 x1, x2, 0x38    // target = 0xb5
;;   84: xload64le_o32 x2, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       xmov x1, x29
;;       call_indirect x2
;;       pop_frame_restore 16, x29
;;       ret
;;   9d: xzero x0
;;   9f: xmov x29, x3
;;   a2: call3 x29, x0, x1, 0x495    // target = 0x537
;;   aa: jump -0x49    // target = 0x61
;;   af: trap
;;   b2: trap
;;   b5: trap
;;
;; wasm[0]::function[5]:
;;       push_frame_save 16, x26
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 5, 0x5e    // target = 0x11e
;;   c7: xload64le_o32 x0, x0, 64
;;       zext32 x15, x2
;;       xshl64_u6 x1, x15, 3
;;       xadd64 x0, x0, x1
;;       xload64le_o32 x1, x0, 0
;;       xband64_s8 x0, x1, -2
;;       br_if_xeq64_i8 x1, 0, 0x2a    // target = 0x10c
;;   e9: xmov x26, x3
;;       br_if_xeq64_i8 x0, 0, 0x35    // target = 0x121
;;   f3: xload64le_o32 x2, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       xmov x1, x26
;;       call_indirect x2
;;       pop_frame_restore 16, x26
;;       ret
;;  10c: xone x0
;;  10e: xmov x26, x3
;;  111: call3 x26, x0, x15, 0x426    // target = 0x537
;;  119: jump -0x2d    // target = 0xec
;;  11e: trap
;;  121: trap
