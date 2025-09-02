;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame
;;       xload32le_o32 x6, x0, 0
;;       br_if_xneq32_i32 x6, 1701998435, 0x25    // target = 0x30
;;   15: xload64le_o32 x7, x0, 8
;;       xmov_fp x8
;;       xstore64le_o32 x7, 56, x8
;;       call -0x27    // target = 0x0
;;       xone x0
;;       pop_frame
;;       ret
;;   30: trap
