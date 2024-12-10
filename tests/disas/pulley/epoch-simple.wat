;;! target = "pulley64"
;;! test = "compile"
;;! flags = '-Wepoch-interruption'

(module
  (func)
)
;; wasm[0]::function[0]:
;;       push_frame
;;       xload64le_offset32 x8, x0, 8
;;       xload64le_offset32 x9, x0, 32
;;       xload64le_offset32 x9, x9, 0
;;       xload64le_offset32 x8, x8, 8
;;       xulteq64 x8, x8, x9
;;       zext8 x8, x8
;;       br_if x8, 0x8    // target = 0x2b
;;   29: pop_frame
;;       ret
;;   2b: call 0xa2    // target = 0xcd
;;   30: jump 0xfffffffffffffff9    // target = 0x29
