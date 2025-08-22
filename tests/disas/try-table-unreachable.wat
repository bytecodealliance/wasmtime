;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wexceptions=yes"]

(module
  (func
    (unreachable)
    (try_table)))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       ud2
