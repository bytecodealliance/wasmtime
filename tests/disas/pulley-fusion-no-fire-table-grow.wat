;;! target = "pulley64"
;;! test = "compile"

;; Phase 1 / phase 2 fusion gating: `table.grow` adds slots at the
;; end of the table; new slots default to `ref.null func`. The
;; "eagerly-initialised, fully-populated" predicate doesn't hold
;; after grow, so fusion is disabled.
;;
;; In our `table_mutability` accounting (crates/environ), `table.grow`
;; sets the mutated bit for the target table the same way
;; `table.set` does. This filetest pins the lowering-level
;; consequence: the unfused dispatch sequence on the grown table.
;;
;; Reference: wasm3 #547 — bounds-check ↔ growth races; Luau release/
;; 717 — "writes to userdata did not invalidate the store cache",
;; same shape of "fused-op cached a base pointer that got
;; reallocated".

(module
  (table 1 funcref)

  (func $f1 (result i32) i32.const 1)

  (func (export "grow") (param i32) (result i32)
    ref.func $f1
    local.get 0
    table.grow 0)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

  (elem (i32.const 0) func $f1))
;; wasm[0]::function[0]::f1:
;;       push_frame
;;       xone x0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]:
;;       push_frame_save 32, x16, x17, x18
;;       xmov x12, x0
;;       xmov x18, x2
;;       xzero x16
;;       xmov x17, x12
;;       call2 x17, x16, 0x2b8    // target = 0x2cd
;;       xmov x2, x18
;;       xmov x12, x17
;;       zext32 x6, x2
;;       call4 x12, x16, x6, x0, 0x321    // target = 0x346
;;       pop_frame_restore 32, x16, x17, x18
;;       ret
;;
;; wasm[0]::function[2]:
;;       push_frame_save 16, x16
;;       xload64le_o32 x1, x0, 56
;;       br_if_xulteq32 x1, x2, 0x7d    // target = 0xbd
;;   47: xload64le_o32 x3, x0, 48
;;       xmov x4, x0
;;       zext32 x1, x2
;;       xshl64_u6 x0, x1, 3
;;       xadd64 x0, x3, x0
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x46    // target = 0xab
;;   6c: xmov x16, x4
;;       br_if_xeq64_i8 x0, 0, 0x51    // target = 0xc0
;;   76: xload32le_o32 x1, x0, 16
;;       xload64le_o32 x2, x16, 40
;;       xload32le_o32 x2, x2, 0
;;       br_if_xneq32 x1, x2, 0x38    // target = 0xc3
;;   92: xload64le_o32 x2, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       xmov x1, x16
;;       call_indirect x2
;;       pop_frame_restore 16, x16
;;       ret
;;   ab: xzero x0
;;   ad: xmov x16, x4
;;   b0: call3 x16, x0, x1, 0x258    // target = 0x308
;;   b8: jump -0x49    // target = 0x6f
;;   bd: trap
;;   c0: trap
;;   c3: trap
