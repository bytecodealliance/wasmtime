;;! target = "pulley64"
;;! test = "compile"
;;! flags = '-Wepoch-interruption'

(module
  (func)
)
;; wasm[0]::function[0]:
;;       push_frame
;;       load64_offset8 x7, x0, 8
;;       load64_offset8 x8, x0, 32
;;       load64 x8, x8
;;       load64_offset8 x7, x7, 8
;;       xulteq64 x7, x7, x8
;;       br_if x7, 0x8    // target = 0x1b
;;   19: pop_frame
;;       ret
;;   1b: call 0xa    // target = 0x25
;;   20: jump 0xfffffffffffffff9    // target = 0x19
