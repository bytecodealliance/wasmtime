(module
  (type (;0;) (func (param i32 i64 f64) (result f64)))
  (type (;1;) (func))
  (type (;2;) (func (result f32)))
  (type (;3;) (func (result f64)))
  (type (;4;) (func (param f64 f64) (result f64)))
  (type (;5;) (func (result i32)))
  (func (result i32)
      block (result i32)
        unreachable
      end
      block
      end
      i32.clz
  )
  (func (result i32)
      loop (result i32)
        unreachable
      end
      block
      end
      i32.clz
  )
  (func (;0;) (type 5) (result i32)
    nop
    block (result i32)  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          nop
          block  ;; label = @4
            i32.const 1
            if  ;; label = @5
              nop
              block  ;; label = @6
                nop
                nop
                loop (result i32)  ;; label = @7
                  nop
                  block (result i32)  ;; label = @8
                    nop
                    nop
                    block (result i32)  ;; label = @9
                      nop
                      unreachable
                    end
                  end
                end
                block (result i32)  ;; label = @7
                  block  ;; label = @8
                    nop
                  end
                  i32.const 0
                end
                br_if 5 (;@1;)
                drop
              end
            else
              nop
            end
            nop
          end
        end
      end
      unreachable
    end)
  (func
    block (result i32)
      block (result i32)
        i32.const 1
        br 1
      end
    end
    drop
  )
  (table (;0;) 16 anyfunc)
  (elem (i32.const 0))
)
