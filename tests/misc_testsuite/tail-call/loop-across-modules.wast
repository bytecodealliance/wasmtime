;;! tail_call = true
;;! reference_types = true

;; Do the following loop: `A.f` indirect tail calls through the table, which is
;; populated by `B.start` to contain `B.g`, which in turn tail calls `A.f` and
;; the loop begins again.
;;
;; This is smoke testing that tail call chains across Wasm modules really do
;; have O(1) stack usage.

(module $A
  (type (func (param i32) (result i32)))

  (table (export "table") 1 1 funcref)

  (func (export "f") (param i32) (result i32)
    local.get 0
    i32.eqz
    if
      (return (i32.const 42))
    else
      (i32.sub (local.get 0) (i32.const 1))
      i32.const 0
      return_call_indirect (type 0)
    end
    unreachable
  )
)

(module $B
  (import "A" "table" (table $table 1 1 funcref))
  (import "A" "f" (func $f (param i32) (result i32)))

  (func $g (export "g") (param i32) (result i32)
    local.get 0
    return_call $f
  )

  (func $start
    (table.set $table (i32.const 0) (ref.func $g))
  )
  (start $start)
)

(assert_return (invoke $B "g" (i32.const 100000000))
               (i32.const 42))
