(assert_trap
  (component
    (type $enum (enum "case0" "case1" "case2"))

    ;; Returns invalid discriminant 3 (valid range: 0-2)
    (component $producer
      (import "enum" (type $enum' (eq $enum)))
      (core module $core
        (func (export "get") (result i32) (i32.const 3)))
      (core instance $inst (instantiate $core))
      (func (export "get") (result $enum') (canon lift (core func $inst "get"))))

    ;; Calls producer through adapter - validation should trap here
    (component $consumer
      (import "enum" (type $enum' (eq $enum)))
      (import "get" (func $get (result $enum')))
      (core func $lowered (canon lower (func $get)))
      (core module $core
        (import "" "get" (func (result i32)))
        (func $start (call 0) drop)
        (start $start))
      (core instance (instantiate $core (with "" (instance (export "get" (func $lowered)))))))

    (instance $prod (instantiate $producer (with "enum" (type $enum))))
    (instance (instantiate $consumer (with "enum" (type $enum)) (with "get" (func $prod "get")))))
  "invalid variant discriminant")
