(module
  (type (;0;) (func))
  (type (;1;) (func (param f32)))
  (func (;0;) (type 1) (param f32)
    global.get 1
    i32.eqz
    if ;; label = @1
      unreachable
    end
    global.get 1
    i32.const 1
    i32.sub
    global.set 1
    nop
    local.get 0
    block ;; label = @1
      loop (type 0) ;; label = @2
        global.get 1
        i32.eqz
        if ;; label = @3
          unreachable
        end
        global.get 1
        i32.const 1
        i32.sub
        global.set 1
        local.get 0
        loop (type 1) (param f32) ;; label = @3
          global.get 1
          i32.eqz
          if ;; label = @4
            unreachable
          end
          global.get 1
          i32.const 1
          i32.sub
          global.set 1
          local.tee 0
          call 0
          loop (type 0) ;; label = @4
            global.get 1
            i32.eqz
            if ;; label = @5
              unreachable
            end
            global.get 1
            i32.const 1
            i32.sub
            global.set 1
            local.get 0
            loop ;; label = @5
              global.get 1
              i32.eqz
              if ;; label = @6
                unreachable
              end
              global.get 1
              i32.const 1
              i32.sub
              global.set 1
              local.get 0
              local.set 0
            end
            local.set 0
            block (type 0) ;; label = @5
              loop (type 0) ;; label = @6
                global.get 1
                i32.eqz
                if ;; label = @7
                  unreachable
                end
                global.get 1
                i32.const 1
                i32.sub
                global.set 1
                nop
                loop ;; label = @7
                  global.get 1
                  i32.eqz
                  if ;; label = @8
                    unreachable
                  end
                  global.get 1
                  i32.const 1
                  i32.sub
                  global.set 1
                  block (type 0) ;; label = @8
                    loop (type 0) ;; label = @9
                      global.get 1
                      i32.eqz
                      if ;; label = @10
                        unreachable
                      end
                      global.get 1
                      i32.const 1
                      i32.sub
                      global.set 1
                      loop ;; label = @10
                        global.get 1
                        i32.eqz
                        if ;; label = @11
                          unreachable
                        end
                        global.get 1
                        i32.const 1
                        i32.sub
                        global.set 1
                        local.get 0
                        call 0
                        loop (type 0) ;; label = @11
                          global.get 1
                          i32.eqz
                          if ;; label = @12
                            unreachable
                          end
                          global.get 1
                          i32.const 1
                          i32.sub
                          global.set 1
                          block (type 0) ;; label = @12
                            loop ;; label = @13
                              global.get 1
                              i32.eqz
                              if ;; label = @14
                                unreachable
                              end
                              global.get 1
                              i32.const 1
                              i32.sub
                              global.set 1
                              block ;; label = @14
                                block (type 0) ;; label = @15
                                  block (type 0) ;; label = @16
                                    block (result f64) ;; label = @17
                                      local.get 0
                                      local.tee 0
                                      local.get 0
                                      loop ;; label = @18
                                        global.get 1
                                        i32.eqz
                                        if ;; label = @19
                                          unreachable
                                        end
                                        global.get 1
                                        i32.const 1
                                        i32.sub
                                        global.set 1
                                      end
                                      local.tee 0
                                      local.tee 0
                                      br 11 (;@6;)
                                      block (type 0) ;; label = @18
                                      end
                                      local.get 0
                                      local.set 0
                                      br 1 (;@16;)
                                      nop
                                      local.get 0
                                      local.tee 0
                                      local.get 0
                                      nop
                                      call 0
                                      local.tee 0
                                      call 0
                                      call 0
                                      local.get 0
                                      local.set 0
                                      local.set 0
                                      br 5 (;@12;)
                                      block (result f32) ;; label = @18
                                        nop
                                        br 10 (;@8;)
                                        br 11 (;@7;)
                                        block (result i64) ;; label = @19
                                          nop
                                          br 13 (;@6;)
                                          br 9 (;@10;)
                                          br 12 (;@7;)
                                          block (result i32) ;; label = @20
                                            br 14 (;@6;)
                                            br 7 (;@13;)
                                            loop ;; label = @21
                                              global.get 1
                                              i32.eqz
                                              if ;; label = @22
                                                unreachable
                                              end
                                              global.get 1
                                              i32.const 1
                                              i32.sub
                                              global.set 1
                                            end
                                            br 20
                                            br 9 (;@11;)
                                            block (type 0) ;; label = @21
                                              loop (type 0) ;; label = @22
                                                global.get 1
                                                i32.eqz
                                                if ;; label = @23
                                                  unreachable
                                                end
                                                global.get 1
                                                i32.const 1
                                                i32.sub
                                                global.set 1
                                                nop
                                              end
                                              block (result f32) ;; label = @22
                                                br 17 (;@5;)
                                                loop ;; label = @23
                                                  global.get 1
                                                  i32.eqz
                                                  if ;; label = @24
                                                    unreachable
                                                  end
                                                  global.get 1
                                                  i32.const 1
                                                  i32.sub
                                                  global.set 1
                                                  local.get 0
                                                  local.tee 0
                                                  local.get 0
                                                  local.set 0
                                                  local.tee 0
                                                  local.tee 0
                                                  local.set 0
                                                  loop (result i32) ;; label = @24
                                                    global.get 1
                                                    i32.eqz
                                                    if ;; label = @25
                                                      unreachable
                                                    end
                                                    global.get 1
                                                    i32.const 1
                                                    i32.sub
                                                    global.set 1
                                                    nop
                                                    return
                                                    local.get 0
                                                    local.set 0
                                                    block (result f64) ;; label = @25
                                                      br 17 (;@8;)
                                                      br 4 (;@21;)
                                                      loop (type 0) ;; label = @26
                                                        global.get 1
                                                        i32.eqz
                                                        if ;; label = @27
                                                          unreachable
                                                        end
                                                        global.get 1
                                                        i32.const 1
                                                        i32.sub
                                                        global.set 1
                                                        local.get 0
                                                        br 8 (;@18;)
                                                        nop
                                                      end
                                                      local.get 0
                                                      local.get 0
                                                      local.tee 0
                                                      local.set 0
                                                      block (type 1) (param f32) ;; label = @26
                                                        br 8 (;@18;)
                                                      end
                                                      br 14 (;@11;)
                                                      loop (result f32) ;; label = @26
                                                        global.get 1
                                                        i32.eqz
                                                        if ;; label = @27
                                                          unreachable
                                                        end
                                                        global.get 1
                                                        i32.const 1
                                                        i32.sub
                                                        global.set 1
                                                        return
                                                        br 18 (;@8;)
                                                        br 18 (;@8;)
                                                        local.get 0
                                                        block (type 1) (param f32) ;; label = @27
                                                          block ;; label = @28
                                                          end
                                                          local.set 0
                                                          br 14 (;@13;)
                                                        end
                                                        local.get 0
                                                        local.tee 0
                                                      end
                                                      local.set 0
                                                      br 13 (;@12;)
                                                      br 10 (;@15;)
                                                      br 23 (;@2;)
                                                      loop ;; label = @26
                                                        global.get 1
                                                        i32.eqz
                                                        if ;; label = @27
                                                          unreachable
                                                        end
                                                        global.get 1
                                                        i32.const 1
                                                        i32.sub
                                                        global.set 1
                                                        block (type 0) ;; label = @27
                                                          block ;; label = @28
                                                            block ;; label = @29
                                                              block ;; label = @30
                                                                block ;; label = @31
                                                                  block ;; label = @32
                                                                    loop (result i32) ;; label = @33
                                                                      global.get 1
                                                                      i32.eqz
                                                                      if ;; label = @34
                                                                        unreachable
                                                                      end
                                                                      global.get 1
                                                                      i32.const 1
                                                                      i32.sub
                                                                      global.set 1
                                                                      block ;; label = @34
                                                                      end
                                                                      i32.const 0
                                                                    end
                                                                    global.get 0
                                                                    i32.xor
                                                                    global.set 0
                                                                  end
                                                                end
                                                              end
                                                            end
                                                          end
                                                        end
                                                      end
                                                      f64.const 0x0p+0 (;=0;)
                                                    end
                                                    drop
                                                    i32.const 67108864
                                                  end
                                                  global.get 0
                                                  i32.xor
                                                  global.set 0
                                                end
                                                f32.const 0x0p+0 (;=0;)
                                              end
                                              drop
                                            end
                                            i32.const 1
                                          end
                                          drop
                                          i64.const 231945011200
                                        end
                                        drop
                                        f32.const 0x1.02p-121 (;=0.00000000000000000000000000000000000037909693;)
                                      end
                                      drop
                                      f64.const 0x1.01p-1026 (;=0.00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000139610347079187;)
                                    end
                                    drop
                                  end
                                end
                              end
                            end
                          end
                        end
                      end
                    end
                  end
                end
              end
            end
          end
        end
      end
    end
    drop
  )
  (table (;0;) 1 633 funcref)
  (global (;0;) (mut i32) i32.const 0)
  (global (;1;) (mut i32) i32.const 1000)
  (export "" (func 0))
  (export "1" (table 0))
)
