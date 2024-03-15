;; Reachable `if` head and reachable consequent and alternative means that the
;; following block is also reachable.

(module
  (func (param i32) (result i32)
    local.get 0
    if  ;; label = @2
      nop
    else
      nop
    end
    i32.const 0))
