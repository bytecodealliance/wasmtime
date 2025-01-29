;;! target = "pulley64"
;;! test = "compile"
;;! flags = '-Wepoch-interruption'

(module
  (func)
)
;; wasm[0]::function[0]:
;;       push_frame
;;       xload64le_o32 x6, x0, 8
;;       xload64le_o32 x7, x0, 32
;;       xload64le_o32 x7, x7, 0
;;       xload64le_o32 x6, x6, 8
;;       br_if_xulteq64 x6, x7, 0x9    // target = 0x26
;;   24: pop_frame
;;       ret
;;   26: call 0x9e    // target = 0xc4
;;   2b: jump -0x7    // target = 0x24
