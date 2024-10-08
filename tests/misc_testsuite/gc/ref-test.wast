(module
  (func (export "nulls-to-nullable-tops") (result i32)
    (ref.test anyref (ref.null any))
    (ref.test anyref (ref.null none))
    i32.and
    (ref.test externref (ref.null extern))
    i32.and
    (ref.test externref (ref.null noextern))
    i32.and
    (ref.test funcref (ref.null func))
    i32.and
    (ref.test funcref (ref.null nofunc))
    i32.and
  )
)

(assert_return (invoke "nulls-to-nullable-tops") (i32.const 1))

(module
  (type $s (struct))
  (func $f (export "non-nulls-to-nullable-tops") (param externref) (result i32)
    (ref.test anyref (struct.new_default $s))
    (ref.test anyref (ref.i31 (i32.const 42)))
    i32.and
    (ref.test externref (local.get 0))
    i32.and
    (ref.test funcref (ref.func $f))
    i32.and
  )
)

(assert_return (invoke "non-nulls-to-nullable-tops" (ref.extern 99)) (i32.const 1))

(module
  (func (export "nulls-to-non-nullable-tops") (result i32)
    (ref.test (ref any) (ref.null any))
    (ref.test (ref any) (ref.null none))
    i32.or
    (ref.test (ref extern) (ref.null extern))
    i32.or
    (ref.test (ref extern) (ref.null noextern))
    i32.or
    (ref.test (ref func) (ref.null func))
    i32.or
    (ref.test (ref func) (ref.null nofunc))
    i32.or
  )
)

(assert_return (invoke "nulls-to-non-nullable-tops") (i32.const 0))

(module
  (type $s (struct))
  (func $f (export "non-nulls-to-non-nullable-tops") (param externref) (result i32)
    (ref.test (ref any) (struct.new_default $s))
    (ref.test (ref any) (ref.i31 (i32.const 42)))
    i32.and
    (ref.test (ref extern) (local.get 0))
    i32.and
    (ref.test (ref func) (ref.func $f))
    i32.and
  )
)

(assert_return (invoke "non-nulls-to-non-nullable-tops" (ref.extern 1)) (i32.const 1))

(module
  (func (export "null-to-nullable-i31") (result i32)
    (ref.test i31ref (ref.null none))
    (ref.test i31ref (ref.null i31))
    i32.and
    (ref.test i31ref (ref.null struct))
    i32.and
    (ref.test i31ref (ref.null array))
    i32.and
    (ref.test i31ref (ref.null eq))
    i32.and
    (ref.test i31ref (ref.null any))
    i32.and
  )
)

(assert_return (invoke "null-to-nullable-i31") (i32.const 1))

(module
  (func (export "truthy-non-null-to-nullable-i31") (result i32)
    (ref.test i31ref (ref.i31 (i32.const 42)))
  )
)

(assert_return (invoke "truthy-non-null-to-nullable-i31") (i32.const 1))

(module
  (type $s (struct))
  (type $a (array i32))
  (func (export "falsey-non-null-to-nullable-i31") (result i32)
    (ref.test i31ref (struct.new_default $s))
    (ref.test i31ref (array.new_default $a (i32.const 3)))
    i32.or
  )
)

(assert_return (invoke "falsey-non-null-to-nullable-i31") (i32.const 0))

(module
  (func (export "null-to-non-nullable-i31") (result i32)
    (ref.test (ref i31) (ref.null none))
    (ref.test (ref i31) (ref.null i31))
    i32.or
    (ref.test (ref i31) (ref.null struct))
    i32.or
    (ref.test (ref i31) (ref.null array))
    i32.or
    (ref.test (ref i31) (ref.null eq))
    i32.or
    (ref.test (ref i31) (ref.null any))
    i32.or
  )
)

(assert_return (invoke "null-to-non-nullable-i31") (i32.const 0))

(module
  (type $s (struct))
  (type $a (array i32))
  (func (export "falsey-non-null-to-non-nullable-i31") (result i32)
    (ref.test (ref i31) (struct.new_default $s))
    (ref.test (ref i31) (array.new_default $a (i32.const 3)))
    i32.or
  )
)

(assert_return (invoke "falsey-non-null-to-non-nullable-i31") (i32.const 0))

(module
  (func (export "truthy-non-null-to-non-nullable-i31") (result i32)
    (ref.test (ref i31) (ref.i31 (i32.const 42)))
  )
)

(assert_return (invoke "truthy-non-null-to-non-nullable-i31") (i32.const 1))

(module
  (func (export "null-to-nullable-middle-types") (result i32)
    (ref.test structref (ref.null any))
    (ref.test structref (ref.null eq))
    i32.and
    (ref.test structref (ref.null i31))
    i32.and
    (ref.test structref (ref.null struct))
    i32.and
    (ref.test structref (ref.null array))
    i32.and
    (ref.test structref (ref.null none))
    i32.and
    (ref.test arrayref (ref.null any))
    i32.and
    (ref.test arrayref (ref.null eq))
    i32.and
    (ref.test arrayref (ref.null i31))
    i32.and
    (ref.test arrayref (ref.null array))
    i32.and
    (ref.test arrayref (ref.null array))
    i32.and
    (ref.test arrayref (ref.null none))
    i32.and
  )
)

(assert_return (invoke "null-to-nullable-middle-types") (i32.const 1))

(module
  (type $s (struct))
  (type $a (array i32))
  (func (export "truthy-non-null-to-nullable-middle-types") (result i32)
    (ref.test eqref (ref.i31 (i32.const 42)))
    (ref.test eqref (struct.new_default $s))
    i32.and
    (ref.test eqref (array.new_default $a (i32.const 3)))
    i32.and
    (ref.test structref (struct.new_default $s))
    i32.and
    (ref.test arrayref (array.new_default $a (i32.const 3)))
    i32.and
  )
)

(assert_return (invoke "truthy-non-null-to-nullable-middle-types") (i32.const 1))

(module
  (type $s (struct))
  (type $a (array i32))
  (func (export "falsey-non-null-to-nullable-middle-types") (result i32)
    (ref.test structref (ref.i31 (i32.const 42)))
    (ref.test structref (array.new_default $a (i32.const 3)))
    i32.or
    (ref.test arrayref (ref.i31 (i32.const 42)))
    i32.or
    (ref.test arrayref (struct.new_default $s))
    i32.or
  )
)

(assert_return (invoke "falsey-non-null-to-nullable-middle-types") (i32.const 0))

(module
  (type $s0 (sub     (struct)))
  (type $s1 (sub $s0 (struct (field i32))))
  (type $a0 (sub     (array (ref null $s0))))
  (type $a1 (sub $a0 (array (ref null $s1))))
  (type $f0 (sub     (func)))
  (type $f1 (sub $f0 (func)))
  (func (export "null-to-nullable-concrete-types") (result i32)
    (ref.test (ref null $s0) (ref.null any))
    (ref.test (ref null $s0) (ref.null eq))
    i32.and
    (ref.test (ref null $s0) (ref.null i31))
    i32.and
    (ref.test (ref null $s0) (ref.null struct))
    i32.and
    (ref.test (ref null $s0) (ref.null array))
    i32.and
    (ref.test (ref null $s0) (ref.null $s0))
    i32.and
    (ref.test (ref null $s0) (ref.null $s1))
    i32.and
    (ref.test (ref null $s0) (ref.null $a0))
    i32.and
    (ref.test (ref null $s0) (ref.null $a1))
    i32.and
    (ref.test (ref null $s0) (ref.null none))
    i32.and
    (ref.test (ref null $s1) (ref.null any))
    i32.and
    (ref.test (ref null $s1) (ref.null eq))
    i32.and
    (ref.test (ref null $s1) (ref.null i31))
    i32.and
    (ref.test (ref null $s1) (ref.null struct))
    i32.and
    (ref.test (ref null $s1) (ref.null array))
    i32.and
    (ref.test (ref null $s1) (ref.null $s0))
    i32.and
    (ref.test (ref null $s1) (ref.null $s1))
    i32.and
    (ref.test (ref null $s1) (ref.null $a0))
    i32.and
    (ref.test (ref null $s1) (ref.null $a1))
    i32.and
    (ref.test (ref null $s1) (ref.null none))
    i32.and
    (ref.test (ref null $a0) (ref.null any))
    i32.and
    (ref.test (ref null $a0) (ref.null eq))
    i32.and
    (ref.test (ref null $a0) (ref.null i31))
    i32.and
    (ref.test (ref null $a0) (ref.null struct))
    i32.and
    (ref.test (ref null $a0) (ref.null array))
    i32.and
    (ref.test (ref null $a0) (ref.null $s0))
    i32.and
    (ref.test (ref null $a0) (ref.null $s1))
    i32.and
    (ref.test (ref null $a0) (ref.null $a0))
    i32.and
    (ref.test (ref null $a0) (ref.null $a1))
    i32.and
    (ref.test (ref null $a0) (ref.null none))
    i32.and
    (ref.test (ref null $a1) (ref.null any))
    i32.and
    (ref.test (ref null $a1) (ref.null eq))
    i32.and
    (ref.test (ref null $a1) (ref.null i31))
    i32.and
    (ref.test (ref null $a1) (ref.null struct))
    i32.and
    (ref.test (ref null $a1) (ref.null array))
    i32.and
    (ref.test (ref null $a1) (ref.null $s0))
    i32.and
    (ref.test (ref null $a1) (ref.null $s1))
    i32.and
    (ref.test (ref null $a1) (ref.null $a0))
    i32.and
    (ref.test (ref null $a1) (ref.null $a1))
    i32.and
    (ref.test (ref null $a1) (ref.null none))
    i32.and
    (ref.test (ref null $f0) (ref.null func))
    i32.and
    (ref.test (ref null $f0) (ref.null $f0))
    i32.and
    (ref.test (ref null $f0) (ref.null $f1))
    i32.and
    (ref.test (ref null $f0) (ref.null nofunc))
    i32.and
    (ref.test (ref null $f1) (ref.null func))
    i32.and
    (ref.test (ref null $f1) (ref.null $f0))
    i32.and
    (ref.test (ref null $f1) (ref.null $f1))
    i32.and
    (ref.test (ref null $f1) (ref.null nofunc))
    i32.and
  )
)

(assert_return (invoke "null-to-nullable-concrete-types") (i32.const 1))

(module
  (type $s0 (sub     (struct)))
  (type $s1 (sub $s0 (struct (field i32))))
  (type $a0 (sub     (array (ref null $s0))))
  (type $a1 (sub $a0 (array (ref null $s1))))
  (type $f0 (sub     (func)))
  (type $f1 (sub $f0 (func)))

  (func $g0 (export "g0") (type $f0) unreachable)
  (func $g1 (export "g1") (type $f1) unreachable)

  (func (export "truthy-non-null-to-nullable-concrete-types") (result i32)
    (ref.test (ref null $s0) (struct.new_default $s0))
    (ref.test (ref null $s0) (struct.new_default $s1))
    i32.and
    (ref.test (ref null $s1) (struct.new_default $s1))
    i32.and
    (ref.test (ref null $a0) (array.new_default $a0 (i32.const 3)))
    i32.and
    (ref.test (ref null $a0) (array.new_default $a1 (i32.const 3)))
    i32.and
    (ref.test (ref null $a1) (array.new_default $a1 (i32.const 3)))
    i32.and
    (ref.test (ref null $f0) (ref.func $g0))
    i32.and
    (ref.test (ref null $f0) (ref.func $g1))
    i32.and
    (ref.test (ref null $f1) (ref.func $g1))
    i32.and
  )
)

(assert_return (invoke "truthy-non-null-to-nullable-concrete-types") (i32.const 1))

(module
  (type $s0 (sub     (struct)))
  (type $s1 (sub $s0 (struct (field i32))))
  (type $a0 (sub     (array (ref null $s0))))
  (type $a1 (sub $a0 (array (ref null $s1))))
  (type $f0 (sub     (func)))
  (type $f1 (sub $f0 (func)))

  (func $g0 (export "g0") (type $f0) unreachable)
  (func $g1 (export "g1") (type $f1) unreachable)

  (func (export "falsey-non-null-to-nullable-concrete-types") (result i32)
    (ref.test (ref null $s1) (struct.new_default $s0))
    (ref.test (ref null $a1) (array.new_default $a0 (i32.const 3)))
    i32.or
    (ref.test (ref null $f1) (ref.func $g0))
    i32.or
  )
)

(assert_return (invoke "falsey-non-null-to-nullable-concrete-types") (i32.const 0))

(module
  (type $s0 (sub     (struct)))
  (type $s1 (sub $s0 (struct (field i32))))
  (type $a0 (sub     (array (ref null $s0))))
  (type $a1 (sub $a0 (array (ref null $s1))))
  (type $f0 (sub     (func)))
  (type $f1 (sub $f0 (func)))

  (func $g0 (export "g0") (type $f0) unreachable)
  (func $g1 (export "g1") (type $f1) unreachable)

  (func (export "null-to-non-nullable-concrete-types") (result i32)
    (ref.test (ref $s0) (ref.null any))
    (ref.test (ref $s0) (ref.null eq))
    i32.or
    (ref.test (ref $s0) (ref.null i31))
    i32.or
    (ref.test (ref $s0) (ref.null struct))
    i32.or
    (ref.test (ref $s0) (ref.null array))
    i32.or
    (ref.test (ref $s0) (ref.null $s0))
    i32.or
    (ref.test (ref $s0) (ref.null $s1))
    i32.or
    (ref.test (ref $s0) (ref.null $a0))
    i32.or
    (ref.test (ref $s0) (ref.null $a1))
    i32.or
    (ref.test (ref $s0) (ref.null none))
    i32.or
    (ref.test (ref $s1) (ref.null any))
    i32.or
    (ref.test (ref $s1) (ref.null eq))
    i32.or
    (ref.test (ref $s1) (ref.null i31))
    i32.or
    (ref.test (ref $s1) (ref.null struct))
    i32.or
    (ref.test (ref $s1) (ref.null array))
    i32.or
    (ref.test (ref $s1) (ref.null $s0))
    i32.or
    (ref.test (ref $s1) (ref.null $s1))
    i32.or
    (ref.test (ref $s1) (ref.null $a0))
    i32.or
    (ref.test (ref $s1) (ref.null $a1))
    i32.or
    (ref.test (ref $s1) (ref.null none))
    i32.or
    (ref.test (ref $a0) (ref.null any))
    i32.or
    (ref.test (ref $a0) (ref.null eq))
    i32.or
    (ref.test (ref $a0) (ref.null i31))
    i32.or
    (ref.test (ref $a0) (ref.null struct))
    i32.or
    (ref.test (ref $a0) (ref.null array))
    i32.or
    (ref.test (ref $a0) (ref.null $s0))
    i32.or
    (ref.test (ref $a0) (ref.null $s1))
    i32.or
    (ref.test (ref $a0) (ref.null $a0))
    i32.or
    (ref.test (ref $a0) (ref.null $a1))
    i32.or
    (ref.test (ref $a0) (ref.null none))
    i32.or
    (ref.test (ref $a1) (ref.null any))
    i32.or
    (ref.test (ref $a1) (ref.null eq))
    i32.or
    (ref.test (ref $a1) (ref.null i31))
    i32.or
    (ref.test (ref $a1) (ref.null struct))
    i32.or
    (ref.test (ref $a1) (ref.null array))
    i32.or
    (ref.test (ref $a1) (ref.null $s0))
    i32.or
    (ref.test (ref $a1) (ref.null $s1))
    i32.or
    (ref.test (ref $a1) (ref.null $a0))
    i32.or
    (ref.test (ref $a1) (ref.null $a1))
    i32.or
    (ref.test (ref $a1) (ref.null none))
    i32.or
    (ref.test (ref $f0) (ref.null nofunc))
    i32.or
    (ref.test (ref $f1) (ref.null nofunc))
    i32.or
  )
)

(assert_return (invoke "null-to-non-nullable-concrete-types") (i32.const 0))

(module
  (type $s0 (sub     (struct)))
  (type $s1 (sub $s0 (struct (field i32))))
  (type $a0 (sub     (array (ref null $s0))))
  (type $a1 (sub $a0 (array (ref null $s1))))
  (type $f0 (sub     (func)))
  (type $f1 (sub $f0 (func)))

  (func $g0 (export "g0") (type $f0) unreachable)
  (func $g1 (export "g1") (type $f1) unreachable)

  (func (export "truthy-non-null-to-non-nullable-concrete-types") (result i32)
    (ref.test (ref $s0) (struct.new_default $s0))
    (ref.test (ref $s0) (struct.new_default $s1))
    i32.and
    (ref.test (ref $s1) (struct.new_default $s1))
    i32.and
    (ref.test (ref $a0) (array.new_default $a0 (i32.const 3)))
    i32.and
    (ref.test (ref $a0) (array.new_default $a1 (i32.const 3)))
    i32.and
    (ref.test (ref $a1) (array.new_default $a1 (i32.const 3)))
    i32.and
    (ref.test (ref $f0) (ref.func $g0))
    i32.and
    (ref.test (ref $f0) (ref.func $g1))
    i32.and
    (ref.test (ref $f1) (ref.func $g1))
    i32.and
  )
)

(assert_return (invoke "truthy-non-null-to-non-nullable-concrete-types") (i32.const 1))

(module
  (type $s0 (sub     (struct)))
  (type $s1 (sub $s0 (struct (field i32))))
  (type $a0 (sub     (array (ref null $s0))))
  (type $a1 (sub $a0 (array (ref null $s1))))
  (type $f0 (sub     (func)))
  (type $f1 (sub $f0 (func)))

  (func $g0 (export "g0") (type $f0) unreachable)
  (func $g1 (export "g1") (type $f1) unreachable)

  (func (export "falsey-non-null-to-non-nullable-concrete-types") (result i32)
    (ref.test (ref $s1) (struct.new_default $s0))
    (ref.test (ref $a1) (array.new_default $a0 (i32.const 3)))
    i32.or
    (ref.test (ref $f1) (ref.func $g0))
    i32.or
  )
)

(assert_return (invoke "falsey-non-null-to-non-nullable-concrete-types") (i32.const 0))

(module
  (func (export "null-to-nullable-bottom-type") (result i32)
    (ref.test nullref (ref.null any))
    (ref.test nullref (ref.null eq))
    i32.and
    (ref.test nullref (ref.null i31))
    i32.and
    (ref.test nullref (ref.null struct))
    i32.and
    (ref.test nullref (ref.null array))
    i32.and
    (ref.test nullref (ref.null none))
    i32.and
    (ref.test nullexternref (ref.null extern))
    i32.and
    (ref.test nullexternref (ref.null noextern))
    i32.and
    (ref.test nullfuncref (ref.null func))
    i32.and
    (ref.test nullfuncref (ref.null nofunc))
    i32.and
  )
)

(assert_return (invoke "null-to-nullable-bottom-type") (i32.const 1))

(module
  (type $s (struct))
  (func $f (export "non-null-to-nullable-bottom-type") (param externref) (result i32)
    (ref.test nullref (struct.new_default $s))
    (ref.test nullexternref (local.get 0))
    i32.or
    (ref.test nullfuncref (ref.func $f))
    i32.or
  )
)

(assert_return (invoke "non-null-to-nullable-bottom-type" (ref.extern 1)) (i32.const 0))

(module
  (func (export "null-to-non-nullable-bottom-type") (result i32)
    (ref.test (ref none) (ref.null any))
    (ref.test (ref none) (ref.null eq))
    i32.or
    (ref.test (ref none) (ref.null i31))
    i32.or
    (ref.test (ref none) (ref.null struct))
    i32.or
    (ref.test (ref none) (ref.null array))
    i32.or
    (ref.test (ref none) (ref.null none))
    i32.or
    (ref.test (ref noextern) (ref.null extern))
    i32.or
    (ref.test (ref noextern) (ref.null noextern))
    i32.or
    (ref.test (ref nofunc) (ref.null func))
    i32.or
    (ref.test (ref nofunc) (ref.null nofunc))
    i32.or
  )
)

(assert_return (invoke "null-to-non-nullable-bottom-type") (i32.const 0))

(module
  (type $s (struct))
  (func $f (export "non-null-to-non-nullable-bottom-type") (param externref) (result i32)
    (ref.test (ref none) (struct.new_default $s))
    (ref.test (ref noextern) (local.get 0))
    i32.or
    (ref.test (ref nofunc) (ref.func $f))
    i32.or
  )
)

(assert_return (invoke "non-null-to-non-nullable-bottom-type" (ref.extern 1)) (i32.const 0))
