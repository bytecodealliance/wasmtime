;;! target = "pulley64"
;;! test = "compile"

;; Phase 1 / phase 2 fusion gating: `table.fill` is a bulk-memory op
;; that mutates an arbitrary range of the table. Like `table.set`, it
;; sets `tables_mutated[idx] = true` for the target table and disables
;; the eager-init predicate. The dispatch tail must be the unfused
;; sequence with the original `brif value` (not `brif value_masked`),
;; so neither phase 1 nor phase 2 fires.
;;
;; Reference: GHSA-q49f-xg75-m9xw (wasmtime Winch table.fill host
;; panic) — bulk table ops are a classic invariant-edge for any
;; "immutable-table" cache or fusion. wasm3 #335 (null table element
;; on Swift reactor-mode tables) showed how a partially-initialised
;; table breaks a "table is fully populated" assumption.

(module
  (table 3 3 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  ;; Bulk mutator: clears the immutability proof for table 0.
  (func (export "fill_some") (param $dst i32)
    local.get $dst
    ref.func $f1
    i32.const 1
    table.fill 0)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

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
;;       push_frame_save 32, x16, x17, x18
;;       xmov x12, x0
;;       xmov x18, x2
;;       xzero x16
;;       xmov x17, x12
;;       call2 x17, x16, 0x3be    // target = 0x3df
;;       xmov x2, x18
;;       xmov x12, x17
;;       zext32 x7, x2
;;       xone x4
;;       call4 x12, x16, x7, x0, 0x425    // target = 0x458
;;       pop_frame_restore 32, x16, x17, x18
;;       ret
;;
;; wasm[0]::function[4]:
;;       push_frame_save 16, x29
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 3, 0x7a    // target = 0xc4
;;   51: xload64le_o32 x0, x0, 48
;;       zext32 x1, x2
;;       xshl64_u6 x2, x1, 3
;;       xadd64 x0, x0, x2
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x46    // target = 0xb2
;;   73: xmov x29, x3
;;       br_if_xeq64_i8 x0, 0, 0x51    // target = 0xc7
;;   7d: xload32le_o32 x1, x0, 16
;;       xload64le_o32 x2, x29, 40
;;       xload32le_o32 x2, x2, 0
;;       br_if_xneq32 x1, x2, 0x38    // target = 0xca
;;   99: xload64le_o32 x2, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       xmov x1, x29
;;       call_indirect x2
;;       pop_frame_restore 16, x29
;;       ret
;;   b2: xzero x0
;;   b4: xmov x29, x3
;;   b7: call3 x29, x0, x1, 0x363    // target = 0x41a
;;   bf: jump -0x49    // target = 0x76
;;   c4: trap
;;   c7: trap
;;   ca: trap
