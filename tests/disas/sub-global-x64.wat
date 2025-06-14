;;! target = "x86_64"
;;! flags = "-W function-references"
;;! test = "compile"

(module
  (global $g (mut i32) (i32.const 0x1000))
  (func
    global.get $g
    i32.const 16
    i32.sub
    global.set $g
  )
)

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subl    $0x10, 0x30(%rdi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
