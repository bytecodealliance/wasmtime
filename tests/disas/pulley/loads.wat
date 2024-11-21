;;! target = "pulley32"
;;! test = "compile"
;;! flags = "-Ccranelift-enable-heap-access-spectre-mitigation=no"

(module
  (memory 0)
  (func $i32 (param i32) (result i32)
    local.get 0
    i32.load
  )

  (func $i64 (param i32) (result i64)
    local.get 0
    i64.load
  )
)

;; wasm[0]::function[0]::i32:
;;       push_frame
;;       load32_u_offset8 x6, x0, 52
;;       br_if_xult32 x6, x2, 0x13    // target = 0x18
;;    c: load32_u_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       load32_u x0, x7
;;       pop_frame
;;       ret
;;   18: trap
;;
;; wasm[0]::function[1]::i64:
;;       push_frame
;;       load32_u_offset8 x6, x0, 52
;;       br_if_xult32 x6, x2, 0x13    // target = 0x18
;;    c: load32_u_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       load64 x0, x7
;;       pop_frame
;;       ret
;;   18: trap
