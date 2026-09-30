;;! gc = true

(module
  (type $s (sub (struct)))
  (func (export "test-struct") (param externref) (result i32)
    (ref.test (ref null $s) (any.convert_extern (local.get 0))))
  (func (export "cast-struct") (param externref) (result i32)
    (drop (ref.cast (ref null $s) (any.convert_extern (local.get 0))))
    (i32.const 0))
  (func (export "br-on-cast-struct") (param externref) (result i32)
    (block $l (result (ref $s))
      (br_on_cast $l anyref (ref $s) (any.convert_extern (local.get 0)))
      (drop)
      (return (i32.const 0)))
    (drop)
    (i32.const 1)))

(assert_return (invoke "test-struct" (ref.extern 1)) (i32.const 0))
(assert_trap (invoke "cast-struct" (ref.extern 1)) "cast failure")
(assert_return (invoke "br-on-cast-struct" (ref.extern 1)) (i32.const 0))

(module
  (type $a (sub (array i8)))
  (func (export "test-array") (param externref) (result i32)
    (ref.test (ref null $a) (any.convert_extern (local.get 0)))))

(assert_return (invoke "test-array" (ref.extern 1)) (i32.const 0))

(module
  (type $s (struct))
  (func (export "test-final") (param externref) (result i32)
    (ref.test (ref null $s) (any.convert_extern (local.get 0)))))

(assert_return (invoke "test-final" (ref.extern 1)) (i32.const 0))

(module
  (func (export "test-abstract") (param externref) (result i32)
    (ref.test (ref null struct) (any.convert_extern (local.get 0)))))

(assert_return (invoke "test-abstract" (ref.extern 1)) (i32.const 0))

(module
  (type $s (sub (struct (field i32))))
  (type $a (sub (array i8)))
  (func (export "test-real-struct") (result i32)
    (ref.test (ref null $s) (struct.new $s (i32.const 1))))
  (func (export "test-real-array") (result i32)
    (ref.test (ref null $a) (array.new_default $a (i32.const 1))))
  (func (export "cast-round-trip") (result i32)
    (struct.get $s 0
      (ref.cast (ref $s)
        (any.convert_extern
          (extern.convert_any (struct.new $s (i32.const 42))))))))

(assert_return (invoke "test-real-struct") (i32.const 1))
(assert_return (invoke "test-real-array") (i32.const 1))
(assert_return (invoke "cast-round-trip") (i32.const 42))
