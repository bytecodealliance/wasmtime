;;! target = "pulley64"
;;! test = "compile"
;;! flags = '-Wepoch-interruption'

(module
  (func)
)
;; wasm[0]::function[0]:
;;       push_frame
;;       xload64le_offset32 x7, x0, 8
;;       xload64le_offset32 x8, x0, 32
;;       xload64le_offset32 x8, x8, 0
;;       xload64le_offset32 x7, x7, 8
;;       xulteq64 x7, x7, x8
;;       br_if x7, 0x8    // target = 0x28
;;   26: pop_frame
;;       ret
;;   28: call 0x9c    // target = 0xc4
;;   2d: jump 0xfffffffffffffff9    // target = 0x26
