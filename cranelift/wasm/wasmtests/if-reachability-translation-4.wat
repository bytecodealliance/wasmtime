;; Reachable `if` head and unreachable consequent and alternative means that the
;; following block is unreachable.

(module
  (func (param i32) (result i32)
    local.get 0
    if
      unreachable
    else
      unreachable
    end
    i32.const 0))
