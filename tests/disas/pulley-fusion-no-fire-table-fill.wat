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
;;       push_frame_save 16, x16, x20
;;       xmov x16, x2
;;       xzero x12
;;       xmov x20, x0
;;       call2 x20, x12, 0x3f7    // target = 0x415
;;       xmov x15, x0
;;       xmov x2, x16
;;       xmov x0, x20
;;       zext32 x12, x2
;;       xadd64_u8 x13, x12, 1
;;       br_if_xugt64_u8 x13, 3, 0x3e    // target = 0x73
;;   3c: xload64le_o32 x13, x0, 48
;;       xshl64_u6 x14, x12, 3
;;       xadd64 x13, x13, x14
;;       xmov x0, x15
;;       xmov x12, x13
;;       xbor64_s8 x14, x0, 1
;;       xstore64le_o32 x12, 0, x14
;;       xadd64_u8 x15, x12, 8
;;       br_if_xeq64 x12, x13, 0xf    // target = 0x6d
;;   65: xmov x12, x15
;;       jump -0x19    // target = 0x4f
;;   6d: pop_frame_restore 16, x16, x20
;;       ret
;;   73: trap
;;
;; wasm[0]::function[4]:
;;       push_frame_save 16, x28
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 3, 0x7c    // target = 0xfa
;;   85: xmov x1, x3
;;       xload64le_o32 x0, x1, 48
;;       zext32 x1, x2
;;       xshl64_u6 x2, x1, 3
;;       xadd64 x0, x0, x2
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x45    // target = 0xe8
;;   aa: xmov x28, x3
;;       br_if_xeq64_i8 x0, 0, 0x50    // target = 0xfd
;;   b4: xload32le_o32 x1, x0, 16
;;       xload64le_o32 x2, x28, 40
;;       xload32le_o32 x2, x2, 0
;;       br_if_xneq32 x1, x2, 0x37    // target = 0x100
;;   d0: xload64le_o32 x1, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       call_indirect2 x1, x0, x28
;;       pop_frame_restore 16, x28
;;       ret
;;   e8: xzero x0
;;   ea: xmov x28, x3
;;   ed: call3 x28, x0, x1, 0x363    // target = 0x450
;;   f5: jump -0x48    // target = 0xad
;;   fa: trap
;;   fd: trap
;;  100: trap
