;; An unreachable `if` means that the consequent, alternative, and following
;; block are also unreachable.

(module
  (func (param i32) (result i32)
    unreachable
    if  ;; label = @2
      nop
    else
      nop
    end
    i32.const 0))
