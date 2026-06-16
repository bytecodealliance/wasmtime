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
;;       push_frame_save 48, x18, x19, x20, x23, x28
;;       xmov x23, x2
;;       xzero x19
;;       xmov x28, x0
;;       call2 x28, x19, 0x313    // target = 0x325
;;       xmov x20, x0
;;       xmov x2, x23
;;       xmov x0, x28
;;       zext32 x18, x2
;;       call3 x28, x19, x18, 0x379    // target = 0x39e
;;       xmov x1, x0
;;       br_if_xeq32_i8 x1, -1, 0x51    // target = 0x81
;;   37: xload64le_o32 x3, x28, 56
;;       zext32 x2, x1
;;       xadd64 x4, x2, x18
;;       zext32 x0, x3
;;       br_if_xult64 x0, x4, 0x43    // target = 0x8a
;;   4e: xload64le_o32 x0, x28, 48
;;       xshl64_u6 x2, x2, 3
;;       xadd64 x0, x0, x2
;;       xshl64_u6 x2, x18, 3
;;       xadd64 x2, x0, x2
;;       br_if_xeq64_i8 x18, 0, 0x20    // target = 0x81
;;   68: xmov x3, x20
;;       xbor64_s8 x4, x3, 1
;;       xstore64le_o32 x0, 0, x4
;;       xadd64_u8 x0, x0, 8
;;       br_if_xneq64 x0, x2, -0xf    // target = 0x6b
;;   81: xmov x0, x1
;;       pop_frame_restore 48, x18, x19, x20, x23, x28
;;       ret
;;   8a: trap
;;
;; wasm[0]::function[2]:
;;       push_frame_save 16, x16
;;       xload64le_o32 x1, x0, 56
;;       br_if_xulteq32 x1, x2, 0x7c    // target = 0x115
;;   a0: xload64le_o32 x3, x0, 48
;;       xmov x4, x0
;;       zext32 x1, x2
;;       xshl64_u6 x0, x1, 3
;;       xadd64 x0, x3, x0
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x45    // target = 0x103
;;   c5: xmov x16, x4
;;       br_if_xeq64_i8 x0, 0, 0x50    // target = 0x118
;;   cf: xload32le_o32 x1, x0, 16
;;       xload64le_o32 x2, x16, 40
;;       xload32le_o32 x2, x2, 0
;;       br_if_xneq32 x1, x2, 0x37    // target = 0x11b
;;   eb: xload64le_o32 x1, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       call_indirect2 x1, x0, x16
;;       pop_frame_restore 16, x16
;;       ret
;;  103: xzero x0
;;  105: xmov x16, x4
;;  108: call3 x16, x0, x1, 0x258    // target = 0x360
;;  110: jump -0x48    // target = 0xc8
;;  115: trap
;;  118: trap
;;  11b: trap
