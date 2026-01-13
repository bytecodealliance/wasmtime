;;! multi_memory = true
;;! component_model_fixed_size_lists = true

;; This contains two components which exercise fixed-size-list
;; types. The first acts as a roundtrip, the second calls the first
;; and compares the results.

;; As this was written with C++ and wit-bindgen the functions
;; are not expected to be easily human readable.

;; The exported function run() from the second component
;; calls nested-roundtrip([[1, 2], [3, 4]], [[-1, -2], [-3, -4]])
;; from the first module and compares the resulting tuple with a
;; concatenation of the inputs.

;; Every mismatch increases the return value by 1.

(component
  (component (;0;)
    (type $ty-test:fixed-size-lists/to-test (;0;)
      (instance
        (type (;0;) (list u32 2))
        (type (;1;) (list 0 2))
        (type (;2;) (list s32 2))
        (type (;3;) (list 2 2))
        (type (;4;) (tuple 1 3))
        (type (;5;) (func (param "a" 1) (param "b" 3) (result 4)))
        (export (;0;) "nested-roundtrip" (func (type 5)))
      )
    )
    (import "test:fixed-size-lists/to-test" (instance $test:fixed-size-lists/to-test (;0;) (type $ty-test:fixed-size-lists/to-test)))
    (core module $main (;0;)
      (type (;0;) (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32)))
      (type (;1;) (func (result i32)))
      (import "test:fixed-size-lists/to-test" "nested-roundtrip" (func (;0;) (type 0)))
      (memory (;0;) 2)
      (global (;0;) (mut i32) (i32.const 67232))
      (export "memory" (memory 0))
      ;; test runner, pass the values from the address 1024ff in registers
      ;; to the roundtrip function and
      ;; return the number of non-matching results
      (export "run" (func 1))
      (func (;1;) (type 1) (result i32)
        (local i32 i32 i32 i32 i32 i32 i32 i32 i32)
        (global.set 0
          (local.tee 0
            (i32.add
              (global.get 0)
              (i32.const -64))))
        (i64.store
          (i32.add
            (local.tee 1
              (i32.add
                (local.get 0)
                (i32.const 16)))
            (i32.const 8))
          (i64.load align=4
            (i32.const 1032)))
        (i64.store
          (i32.add
            (local.get 0)
            (i32.const 8))
          (i64.load align=4
            (i32.const 1048)))
        (i64.store offset=16
          (local.get 0)
          (i64.load align=4
            (i32.const 1024)))
        (i64.store
          (local.get 0)
          (i64.load align=4
            (i32.const 1040)))
        (global.set 0
          (local.tee 2
            (i32.sub
              (global.get 0)
              (i32.const 32))))
        (call 0
          (i32.load
            (local.get 1))
          (i32.load offset=4
            (local.get 1))
          (i32.load offset=8
            (local.get 1))
          (i32.load offset=12
            (local.get 1))
          (i32.load
            (local.get 0))
          (i32.load offset=4
            (local.get 0))
          (i32.load offset=8
            (local.get 0))
          (i32.load offset=12
            (local.get 0))
          (local.get 2))
        (i64.store offset=24 align=4
          (local.tee 1
            (i32.add
              (local.get 0)
              (i32.const 32)))
          (i64.load offset=24
            (local.get 2)))
        (i64.store offset=16 align=4
          (local.get 1)
          (i64.load offset=16
            (local.get 2)))
        (i64.store offset=8 align=4
          (local.get 1)
          (i64.load offset=8
            (local.get 2)))
        (i64.store align=4
          (local.get 1)
          (i64.load
            (local.get 2)))
        (global.set 0
          (i32.add
            (local.get 2)
            (i32.const 32)))
        (local.set 2
          (i32.load offset=48
            (local.get 0)))
        (i32.add
          (i32.add
            (i32.add
              (i32.add
                (i32.add
                  (i32.add
                    (i32.add
                      (i32.ne
                        (i32.load offset=32
                          (local.get 0))
                        (local.set 3
                          (i32.load offset=36
                            (local.get 0)))
                        (local.set 4
                          (i32.load offset=52
                            (local.get 0)))
                        (local.set 5
                          (i32.load offset=40
                            (local.get 0)))
                        (local.set 6
                          (i32.load offset=56
                            (local.get 0)))
                        (local.set 7
                          (i32.load offset=44
                            (local.get 0)))
                        (local.set 8
                          (i32.load offset=60
                            (local.get 0)))
                        ;; here the output values are checked
                        (global.set 0
                          (i32.sub
                            (local.get 0)
                            (i32.const -64)))
                        (i32.const 1))
                      (i32.ne
                        (local.get 2)
                        (i32.const -1)))
                    (i32.ne
                      (local.get 3)
                      (i32.const 2)))
                  (i32.ne
                    (local.get 4)
                    (i32.const -2)))
                (i32.ne
                  (local.get 5)
                  (i32.const 3)))
              (i32.ne
                (local.get 6)
                (i32.const -3)))
            (i32.ne
              (local.get 7)
              (i32.const 4)))
          (i32.ne
            (local.get 8)
            (i32.const -4)))
      )
      ;; here are the input values
      (data (;0;) (i32.const 1024) "\01\00\00\00\02\00\00\00\03\00\00\00\04\00\00\00\ff\ff\ff\ff\fe\ff\ff\ff\fd\ff\ff\ff\fc\ff\ff\ff")
      (data (;1;) (i32.const 1056) "\ff\ff\ff\ff\00\00\02")
      (@producers
        (processed-by "wit-component" "0.243.0")
      )
    )
    (core module $wit-component-shim-module (;1;)
      (type (;0;) (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32)))
      (table (;0;) 1 1 funcref)
      (export "0" (func 0))
      (export "$imports" (table 0))
      (func (;0;) (type 0) (param i32 i32 i32 i32 i32 i32 i32 i32 i32)
        (call_indirect (type 0)
          (local.get 0)
          (local.get 1)
          (local.get 2)
          (local.get 3)
          (local.get 4)
          (local.get 5)
          (local.get 6)
          (local.get 7)
          (local.get 8)
          (i32.const 0))
      )
      (@producers
        (processed-by "wit-component" "0.243.0")
      )
    )
    (core module $wit-component-fixup (;2;)
      (type (;0;) (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32)))
      (import "" "0" (func (;0;) (type 0)))
      (import "" "$imports" (table (;0;) 1 1 funcref))
      (elem (;0;) (i32.const 0) func 0)
      (@producers
        (processed-by "wit-component" "0.243.0")
      )
    )
    (core instance $wit-component-shim-instance (;0;) (instantiate $wit-component-shim-module))
    (alias core export $wit-component-shim-instance "0" (core func $indirect-test:fixed-size-lists/to-test-nested-roundtrip (;0;)))
    (core instance $test:fixed-size-lists/to-test (;1;)
      (export "nested-roundtrip" (func $indirect-test:fixed-size-lists/to-test-nested-roundtrip))
    )
    (core instance $main (;2;) (instantiate $main
        (with "test:fixed-size-lists/to-test" (instance $test:fixed-size-lists/to-test))
      )
    )
    (alias core export $main "memory" (core memory $memory (;0;)))
    (alias core export $wit-component-shim-instance "$imports" (core table $"shim table" (;0;)))
    (alias export $test:fixed-size-lists/to-test "nested-roundtrip" (func $nested-roundtrip (;0;)))
    (core func $"#core-func1 indirect-test:fixed-size-lists/to-test-nested-roundtrip" (@name "indirect-test:fixed-size-lists/to-test-nested-roundtrip") (;1;) (canon lower (func $nested-roundtrip) (memory $memory)))
    (core instance $fixup-args (;3;)
      (export "$imports" (table $"shim table"))
      (export "0" (func $"#core-func1 indirect-test:fixed-size-lists/to-test-nested-roundtrip"))
    )
    (core instance $fixup (;4;) (instantiate $wit-component-fixup
        (with "" (instance $fixup-args))
      )
    )
    (type (;1;) (func (result u32)))
    (alias core export $main "run" (core func $run (;2;)))
    (func $run (;1;) (type 1) (canon lift (core func $run)))
    (export $"#func2 run" (@name "run") (;2;) "run" (func $run))
    (@producers
      (processed-by "wit-component" "0.243.0")
    )
  )
  (component (;1;)
    (core module $main (;0;)
      (type (;0;) (func (param i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))
      (memory (;0;) 2)
      (global (;0;) (mut i32) (i32.const 67248))
      (export "memory" (memory 0))
      (export "test:fixed-size-lists/to-test#nested-roundtrip" (func 0))
      ;; This is an obfuscated store from the stack to address 1056, 
      ;; which is then returned
      (func (;0;) (type 0) (param i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
        (local i32 i32 i64 i64)
        (global.set 0
          (local.tee 8
            (i32.sub
              (global.get 0)
              (i32.const 96))))
        (i32.store
          (local.tee 9
            (i32.add
              (local.get 8)
              (i32.const 56)))
          (local.get 2))
        (i32.store
          (local.tee 2
            (i32.add
              (local.get 8)
              (i32.const 40)))
          (local.get 6))
        (i32.store offset=60
          (local.get 8)
          (local.get 3))
        (i64.store
          (local.tee 6
            (i32.add
              (local.tee 3
                (i32.add
                  (local.get 8)
                  (i32.const 16)))
              (i32.const 8)))
          (i64.load
            (local.get 9)))
        (i32.store offset=44
          (local.get 8)
          (local.get 7))
        (i64.store
          (local.tee 7
            (i32.add
              (local.get 8)
              (i32.const 8)))
          (i64.load
            (local.get 2)))
        (i64.store offset=48
          (local.get 8)
          (local.tee 10
            (i64.or
              (i64.extend_i32_u
                (local.get 0))
              (i64.shl
                (i64.extend_i32_u
                  (local.get 1))
                (i64.const 32)))))
        (i64.store offset=32
          (local.get 8)
          (local.tee 11
            (i64.or
              (i64.extend_i32_u
                (local.get 4))
              (i64.shl
                (i64.extend_i32_u
                  (local.get 5))
                (i64.const 32)))))
        (i64.store offset=16
          (local.get 8)
          (local.get 10))
        (i64.store
          (local.get 8)
          (local.get 11))
        (i64.store align=4
          (local.tee 0
            (i32.sub
              (local.get 8)
              (i32.const -64)))
          (i64.load align=4
            (local.get 3)))
        (i64.store offset=16 align=4
          (local.get 0)
          (i64.load align=4
            (local.get 8)))
        (i64.store align=4
          (local.tee 1
            (i32.add
              (local.get 0)
              (i32.const 8)))
          (i64.load align=4
            (local.get 6)))
        (i64.store align=4
          (i32.add
            (local.get 0)
            (i32.const 24))
          (i64.load align=4
            (local.get 7)))
        (i64.store
          (i32.const 1064)
          (i64.load align=4
            (local.get 1)))
        (i64.store
          (i32.const 1056)
          (i64.load offset=64 align=4
            (local.get 8)))
        (i64.store
          (i32.const 1072)
          (i64.load offset=80 align=4
            (local.get 8)))
        (i64.store
          (i32.const 1080)
          (i64.load align=4
            (i32.add
              (local.get 8)
              (i32.const 88))))
        (global.set 0
          (i32.add
            (local.get 8)
            (i32.const 96)))
        (i32.const 1056)
      )
      (data (;0;) (i32.const 1024) "\ff\ff\ff\ff\00\00\02")
      (@producers
        (processed-by "wit-component" "0.243.0")
      )
    )
    (core instance $main (;0;) (instantiate $main))
    (alias core export $main "memory" (core memory $memory (;0;)))
    (type (;0;) (list u32 2))
    (type (;1;) (list 0 2))
    (type (;2;) (list s32 2))
    (type (;3;) (list 2 2))
    (type (;4;) (tuple 1 3))
    (type (;5;) (func (param "a" 1) (param "b" 3) (result 4)))
    (alias core export $main "test:fixed-size-lists/to-test#nested-roundtrip" (core func $test:fixed-size-lists/to-test#nested-roundtrip (;0;)))
    (func $nested-roundtrip (;0;) (type 5) (canon lift (core func $test:fixed-size-lists/to-test#nested-roundtrip) (memory $memory)))
    (component $test:fixed-size-lists/to-test-shim-component (;0;)
      (type (;0;) (list u32 2))
      (type (;1;) (list 0 2))
      (type (;2;) (list s32 2))
      (type (;3;) (list 2 2))
      (type (;4;) (tuple 1 3))
      (type (;5;) (func (param "a" 1) (param "b" 3) (result 4)))
      (import "import-func-nested-roundtrip" (func (;0;) (type 5)))
      (type (;6;) (list u32 2))
      (type (;7;) (list 6 2))
      (type (;8;) (list s32 2))
      (type (;9;) (list 8 2))
      (type (;10;) (tuple 7 9))
      (type (;11;) (func (param "a" 7) (param "b" 9) (result 10)))
      (export (;1;) "nested-roundtrip" (func 0) (func (type 11)))
    )
    (instance $test:fixed-size-lists/to-test-shim-instance (;0;) (instantiate $test:fixed-size-lists/to-test-shim-component
        (with "import-func-nested-roundtrip" (func $nested-roundtrip))
      )
    )
    (export $test:fixed-size-lists/to-test (;1;) "test:fixed-size-lists/to-test" (instance $test:fixed-size-lists/to-test-shim-instance))
    (@producers
      (processed-by "wit-component" "0.243.0")
    )
  )
  (instance (;0;) (instantiate 1))
  (alias export 0 "test:fixed-size-lists/to-test" (instance (;1;)))
  (instance (;2;) (instantiate 0
      (with "test:fixed-size-lists/to-test" (instance 1))
    )
  )
  (alias export 2 "run" (func (;0;)))
  (export (;1;) "run" (func 0))
)

;; call run, it will return the number of mismatches in the test
(assert_return (invoke "run")
  (u32.const 0))
