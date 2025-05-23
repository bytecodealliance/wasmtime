(module
  (type (;0;) (func (result i32 i32)))
  (func (;0;) (export "main") (type 0) (result i32 i32)
    loop (result i32) ;; label = @1
      i32.const 0
      i32.const 1
      i32.const 0
      i32.const 1
      br_if 1
      br_if 0 (;@1;)
      block (result i32) ;; label = @2
        call 0
        drop
        loop (type 0) (result i32 i32) ;; label = @3
          i32.const 0
          i32.const 1
          block (type 0) (result i32 i32) ;; label = @4
            call 0
            i32.const 0
            br_table 0 (;@4;) 4
          end
          drop
          drop
        end
        drop
        drop
      end
      drop
      drop
    end
    i32.const 0
  )
)

(assert_return (invoke "main") (i32.const 1) (i32.const 0))
