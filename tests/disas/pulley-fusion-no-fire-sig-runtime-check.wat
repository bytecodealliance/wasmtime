;;! target = "pulley64"
;;! test = "compile"

;; Phase 2 fusion does NOT match when the sig check is NOT statically
;; elided. With a runtime sig check, the continuation block starts
;; with a sig load (from the funcref's `type_index` field) + comparison
;; + trapz, NOT the two `wasm_call` / `vmctx` loads. Phase 2's
;; recogniser requires the first two CLIF insts in the continuation
;; to be the canonical loads, so it bails. Phase 1's band+brif fusion
;; still applies as fallback.
;;
;; The module shape: an untyped `funcref` table with elem entries of
;; MIXED signatures. With mixed sigs, `try_elide_sig_check_for_immutable_table`
;; cannot establish a uniform static type, and the runtime sig check
;; stays in the dispatch tail.
;;
;; Reference precedent: V8 issue 5913 ("call_indirect signature
;; mismatch with table-sharing") + WebKit changeset 273962
;; ("call_ref / non-null funcref"): sig elision under "assumed-
;; immutable" predicates is a known footgun, and the safe fallback
;; is "keep the runtime sig check".

(module
  (table 3 3 funcref)
  (type $sig (func (param i32) (result i32)))

  ;; $f1, $f2 match $sig.
  (func $f1 (param i32) (result i32) i32.const 1)
  (func $f2 (param i32) (result i32) i32.const 2)
  ;; $f3 has a DIFFERENT signature — defeats uniform-sig elision.
  (func $f3 (result i32) i32.const 3)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    local.get 0
    call_indirect (type $sig))

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
;;       push_frame_save 16, x16, x18
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 3, 0x82    // target = 0x9b
;;   20: xmov x1, x3
;;       xload64le_o32 x0, x1, 48
;;       zext32 x1, x2
;;       xmov x18, x2
;;       xshl64_u6 x2, x1, 3
;;       xadd64 x0, x0, x2
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x48    // target = 0x89
;;   48: xmov x16, x3
;;       br_if_xeq64_i8 x0, 0, 0x53    // target = 0x9e
;;   52: xload32le_o32 x1, x0, 16
;;       xload64le_o32 x2, x16, 40
;;       xload32le_o32 x2, x2, 0
;;       br_if_xneq32 x1, x2, 0x3a    // target = 0xa1
;;   6e: xload64le_o32 x1, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       xmov x2, x18
;;       call_indirect2 x1, x0, x16
;;       pop_frame_restore 16, x16, x18
;;       ret
;;   89: xzero x0
;;   8b: xmov x16, x3
;;   8e: call3 x16, x0, x1, 0x281    // target = 0x30f
;;   96: jump -0x4b    // target = 0x4b
;;   9b: trap
;;   9e: trap
;;   a1: trap
