(module
  (type (;0;) (func))
  (type (;1;) (func (param i64)))
  (func (;0;) (type 0))
  (func (;1;) (type 0))
  (func (;2;) (type 0))
  (func (;3;) (type 0))
  (func (;4;) (type 1) (param i64)
    (local f32 f32 f32)
    loop (result i64)  ;; label = @1
      global.get 0
      if  ;; label = @2
        local.get 1
        return
      end
      block (result i64)  ;; label = @2
        loop  ;; label = @3
          block  ;; label = @4
            global.get 0
            if  ;; label = @5
              i32.const 5
              if (result f32)  ;; label = @6
                block (result f32)  ;; label = @7
                  call 0
                  i32.const 7
                  if (result f32)  ;; label = @8
                    local.get 2
                  else
                    f32.const 0x1p+0 (;=1;)
                  end
                end
              else
                f32.const 0x1p+0 (;=1;)
              end
              local.tee 1
              local.set 3
            end
          end
        end
        i32.const 8
        br_if 1 (;@1;)
        i64.const 4
      end
    end
    return)
  (memory (;0;) 1)
  (global (;0;) i32 (i32.const 0))
)

