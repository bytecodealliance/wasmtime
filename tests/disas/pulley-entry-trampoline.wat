;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame
;;       xload64le_o32 x5, x0, 8
;;       xmov_fp x6
;;       xstore64le_o32 x5, 56, x6
;;       call -0x16    // target = 0x0
;;       xone x0
;;       pop_frame
;;       ret
