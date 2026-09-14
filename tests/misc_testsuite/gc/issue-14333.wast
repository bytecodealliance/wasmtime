;;! gc = true

;; An `if` needs no `else` when each param is a subtype of its result.

(module
  (type $sup (sub (struct (field i32))))
  (type $sub (sub $sup (struct (field i32) (field i32))))

  (func (export "no-else") (param i32) (result i32)
    (struct.new $sub (i32.const 7) (i32.const 8))
    (local.get 0)
    if (param (ref $sub)) (result (ref $sup))
      drop
      (struct.new $sup (i32.const 100))
    end
    (struct.get $sup 0))

  (func (export "no-else-i31") (param i32) (result i32)
    (ref.i31 (i32.const 33))
    (local.get 0)
    if (param i31ref) (result anyref)
    end
    (ref.cast i31ref)
    (i31.get_s))

  (func (export "with-else") (param i32) (result i32)
    (struct.new $sub (i32.const 7) (i32.const 8))
    (local.get 0)
    if (param (ref $sub)) (result (ref $sup))
      drop
      (struct.new $sup (i32.const 1))
    else
      drop
      (struct.new $sup (i32.const 2))
    end
    (struct.get $sup 0))

  (func (export "else-required") (param i32) (result i32)
    (struct.new $sup (i32.const 5))
    (local.get 0)
    if (param anyref) (result (ref $sup))
      (ref.cast (ref $sup))
    else
      drop
      (struct.new $sup (i32.const 6))
    end
    (struct.get $sup 0))
)

(assert_return (invoke "no-else" (i32.const 0)) (i32.const 7))
(assert_return (invoke "no-else" (i32.const 1)) (i32.const 100))
(assert_return (invoke "no-else-i31" (i32.const 0)) (i32.const 33))
(assert_return (invoke "no-else-i31" (i32.const 1)) (i32.const 33))
(assert_return (invoke "with-else" (i32.const 0)) (i32.const 2))
(assert_return (invoke "with-else" (i32.const 1)) (i32.const 1))
(assert_return (invoke "else-required" (i32.const 0)) (i32.const 6))
(assert_return (invoke "else-required" (i32.const 1)) (i32.const 5))
