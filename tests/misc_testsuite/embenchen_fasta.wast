;; copied from a historical cranelift-wasm test and provided here as proof that
;; this still compiles on various platforms and such

(module $env
  (memory (export "memory") 2 2)
  (table (export "table") 9 9 funcref)
  (global (export "DYNAMICTOP_PTR") i32 i32.const 0)
  (global (export "STACKTOP") i32 i32.const 0)
  (global (export "STACK_MAX") i32 i32.const 0)
  (global (export "memoryBase") i32 i32.const 0)
  (global (export "tableBase") i32 i32.const 0)
  (func (export "abort") (param i32))
  (func (export "enlargeMemory") (result i32) unreachable)
  (func (export "getTotalMemory") (result i32) unreachable)
  (func (export "abortOnCannotGrowMemory") (result i32) unreachable)
  (func (export "_pthread_cleanup_pop") (param i32))
  (func (export "___syscall6") (param i32 i32) (result i32) unreachable)
  (func (export "_pthread_cleanup_push") (param i32 i32))
  (func (export "_abort"))
  (func (export "___setErrNo") (param i32))
  (func (export "_emscripten_memcpy_big") (param i32 i32 i32) (result i32) unreachable)
  (func (export "___syscall54") (param i32 i32) (result i32) unreachable)
  (func (export "___syscall140") (param i32 i32) (result i32) unreachable)
  (func (export "___syscall146") (param i32 i32) (result i32) unreachable)
)

(module
  (type $0 (;0;) (func (param i32 i32 i32) (result i32)))
  (type $1 (;1;) (func))
  (type $2 (;2;) (func (param i32) (result i32)))
  (type $3 (;3;) (func (param i32)))
  (type $4 (;4;) (func (result i32)))
  (type $5 (;5;) (func (param i32 i32)))
  (type $6 (;6;) (func (param i32 i32) (result i32)))
  (type $7 (;7;) (func (param i32 i32 i32 i32 i32) (result i32)))
  (type $8 (;8;) (func (param i32 i32 i32)))
  (type $9 (;9;) (func (param i64 i32) (result i32)))
  (type $10 (;10;) (func (param i32 i32 i32 i32 i32)))
  (type $11 (;11;) (func (param f64 i32) (result f64)))
  (type $12 (;12;) (func (param i32 i32 i32 i32) (result i32)))
  (import "env" "memory" (memory $16 (;0;) 2 2))
  (import "env" "table" (table $timport$17 (;0;) 9 9 funcref))
  (import "env" "DYNAMICTOP_PTR" (global $gimport$0 (;0;) i32))
  (import "env" "STACKTOP" (global $gimport$1 (;1;) i32))
  (import "env" "STACK_MAX" (global $gimport$2 (;2;) i32))
  (import "env" "memoryBase" (global $gimport$18 (;3;) i32))
  (import "env" "tableBase" (global $gimport$19 (;4;) i32))
  (import "env" "abort" (func $fimport$3 (;0;) (type $3)))
  (import "env" "enlargeMemory" (func $fimport$4 (;1;) (type $4)))
  (import "env" "getTotalMemory" (func $fimport$5 (;2;) (type $4)))
  (import "env" "abortOnCannotGrowMemory" (func $fimport$6 (;3;) (type $4)))
  (import "env" "_pthread_cleanup_pop" (func $fimport$7 (;4;) (type $3)))
  (import "env" "_abort" (func $fimport$8 (;5;) (type $1)))
  (import "env" "_pthread_cleanup_push" (func $fimport$9 (;6;) (type $5)))
  (import "env" "___syscall6" (func $fimport$10 (;7;) (type $6)))
  (import "env" "___setErrNo" (func $fimport$11 (;8;) (type $3)))
  (import "env" "_emscripten_memcpy_big" (func $fimport$12 (;9;) (type $0)))
  (import "env" "___syscall54" (func $fimport$13 (;10;) (type $6)))
  (import "env" "___syscall140" (func $fimport$14 (;11;) (type $6)))
  (import "env" "___syscall146" (func $fimport$15 (;12;) (type $6)))
  (func $0 (;13;) (type $2) (param $0 i32) (result i32)
    (local $1 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $1
      global.get $global$1
      local.get $0
      i32.add
      global.set $global$1
      global.get $global$1
      i32.const 15
      i32.add
      i32.const -16
      i32.and
      global.set $global$1
      local.get $1
    end
  )
  (func $1 (;14;) (type $4) (result i32)
    global.get $global$1
  )
  (func $2 (;15;) (type $3) (param $0 i32)
    local.get $0
    global.set $global$1
  )
  (func $3 (;16;) (type $5) (param $0 i32) (param $1 i32)
    block $label$1 ;; label = @1
      local.get $0
      global.set $global$1
      local.get $1
      global.set $global$2
    end
  )
  (func $4 (;17;) (type $5) (param $0 i32) (param $1 i32)
    global.get $global$3
    i32.eqz
    if ;; label = @1
      block ;; label = @2
        local.get $0
        global.set $global$3
        local.get $1
        global.set $global$4
      end
    end
  )
  (func $5 (;18;) (type $3) (param $0 i32)
    local.get $0
    global.set $global$5
  )
  (func $6 (;19;) (type $4) (result i32)
    global.get $global$5
  )
  (func $7 (;20;) (type $6) (param $0 i32) (param $1 i32) (result i32)
    (local $2 i32) (local $3 i32) (local $4 i32) (local $5 i32) (local $6 i32) (local $7 i32) (local $8 i32) (local $9 i32) (local $10 i32) (local $11 f32) (local $12 f32) (local $13 f64)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $5
      global.get $global$1
      i32.const 4256
      i32.add
      global.set $global$1
      local.get $5
      local.set $3
      local.get $5
      i32.const 2128
      i32.add
      local.set $6
      local.get $5
      i32.const 8
      i32.add
      local.set $7
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          local.get $0
          i32.const 1
          i32.le_s
          br_if 0 (;@3;)
          block $label$4 ;; label = @4
            block $label$5 ;; label = @5
              block $label$6 ;; label = @6
                block $label$7 ;; label = @7
                  block $label$8 ;; label = @8
                    block $label$9 ;; label = @9
                      block $label$10 ;; label = @10
                        local.get $1
                        i32.load offset=4
                        i32.load8_s
                        local.tee $0
                        i32.const 48
                        i32.sub
                        br_table 5 (;@5;) 0 (;@10;) 2 (;@8;) 1 (;@9;) 3 (;@7;) 4 (;@6;) 6 (;@4;)
                      end
                      i32.const 950000
                      local.set $4
                      br 7 (;@2;)
                    end
                    br 5 (;@3;)
                  end
                  i32.const 9500000
                  local.set $4
                  br 5 (;@2;)
                end
                i32.const 95000000
                local.set $4
                br 4 (;@2;)
              end
              i32.const 190000000
              local.set $4
              br 3 (;@2;)
            end
            local.get $5
            global.set $global$1
            i32.const 0
            return
          end
          local.get $3
          local.get $0
          i32.const -48
          i32.add
          i32.store
          i32.const 1400
          local.get $3
          call $34
          drop
          local.get $5
          global.set $global$1
          i32.const -1
          return
        end
        i32.const 19000000
        local.set $4
      end
      i32.const 347
      call $40
      local.tee $8
      i32.const 1411
      i32.const 287
      call $47
      drop
      local.get $8
      i32.const 287
      i32.add
      local.tee $0
      i32.const 1411
      i64.load align=1
      i64.store align=1
      local.get $0
      i32.const 1419
      i64.load align=1
      i64.store offset=8 align=1
      local.get $0
      i32.const 1427
      i64.load align=1
      i64.store offset=16 align=1
      local.get $0
      i32.const 1435
      i64.load align=1
      i64.store offset=24 align=1
      local.get $0
      i32.const 1443
      i64.load align=1
      i64.store offset=32 align=1
      local.get $0
      i32.const 1451
      i64.load align=1
      i64.store offset=40 align=1
      local.get $0
      i32.const 1459
      i64.load align=1
      i64.store offset=48 align=1
      local.get $0
      i32.const 1467
      i32.load align=1
      i32.store offset=56 align=1
      local.get $4
      i32.const 1
      i32.shl
      local.set $0
      i32.const 0
      local.set $1
      loop $label$11 ;; label = @2
        local.get $0
        i32.const 60
        i32.lt_u
        if (result i32) ;; label = @3
          local.get $0
        else
          i32.const 60
        end
        local.tee $3
        i32.const 2
        i32.add
        call $40
        local.tee $2
        local.get $8
        local.get $1
        i32.add
        local.get $3
        call $47
        drop
        local.get $2
        local.get $3
        i32.add
        i32.const 0
        i32.store8
        local.get $2
        call $31
        local.tee $10
        i32.const 1024
        i32.load
        local.tee $9
        i32.gt_s
        if ;; label = @3
          local.get $9
          i32.const 0
          i32.gt_s
          if ;; label = @4
            block ;; label = @5
              local.get $2
              local.get $9
              i32.add
              i32.const 0
              i32.store8
              local.get $2
              call $35
              drop
              i32.const 1024
              i32.const 0
              i32.store
            end
          end
        else
          block ;; label = @4
            local.get $2
            call $35
            drop
            i32.const 1024
            i32.const 1024
            i32.load
            local.get $10
            i32.sub
            i32.store
          end
        end
        local.get $2
        call $41
        local.get $3
        local.get $1
        i32.add
        local.tee $2
        i32.const -287
        i32.add
        local.set $1
        local.get $2
        i32.const 287
        i32.le_u
        if ;; label = @3
          local.get $2
          local.set $1
        end
        local.get $0
        local.get $3
        i32.sub
        local.tee $0
        br_if 0 (;@2;)
      end
      local.get $8
      call $42
      i32.const 1028
      i32.load
      if ;; label = @2
        block ;; label = @3
          i32.const 1028
          local.set $0
          f32.const 0x0p+0 (;=0;)
          local.set $11
          loop $label$19 ;; label = @4
            local.get $11
            local.get $0
            i32.const 4
            i32.add
            local.tee $1
            f32.load
            f32.add
            local.tee $11
            f64.promote_f32
            local.tee $13
            f64.const 0x1p+0 (;=1;)
            f64.lt
            if (result f64) ;; label = @5
              local.get $13
            else
              f64.const 0x1p+0 (;=1;)
            end
            f32.demote_f64
            local.set $12
            local.get $1
            local.get $12
            f32.store
            local.get $0
            local.get $12
            f32.const 0x1p+9 (;=512;)
            f32.mul
            i32.trunc_f32_s
            i32.store offset=8
            local.get $0
            i32.const 12
            i32.add
            local.tee $0
            i32.load
            br_if 0 (;@4;)
            i32.const 0
            local.set $1
            i32.const 1028
            local.set $0
          end
        end
      else
        block ;; label = @3
          i32.const 0
          local.set $1
          i32.const 1028
          local.set $0
        end
      end
      loop $label$23 ;; label = @2
        loop $label$24 ;; label = @3
          local.get $0
          i32.const 12
          i32.add
          local.set $3
          local.get $1
          local.get $0
          i32.load offset=8
          local.tee $2
          i32.gt_u
          local.get $2
          i32.const 0
          i32.ne
          i32.and
          if ;; label = @4
            block ;; label = @5
              local.get $3
              local.set $0
              br 2 (;@3;)
            end
          end
        end
        local.get $6
        local.get $1
        i32.const 2
        i32.shl
        i32.add
        local.get $0
        i32.store
        local.get $1
        i32.const 1
        i32.add
        local.tee $1
        i32.const 513
        i32.ne
        br_if 0 (;@2;)
      end
      local.get $6
      i32.const 2116
      i32.add
      i32.const 0
      i32.store
      local.get $4
      i32.const 3
      i32.mul
      local.set $0
      loop $label$26 ;; label = @2
        local.get $6
        local.get $0
        i32.const 60
        i32.lt_u
        if (result i32) ;; label = @3
          local.get $0
        else
          i32.const 60
        end
        local.tee $1
        call $8
        local.get $0
        local.get $1
        i32.sub
        local.tee $0
        br_if 0 (;@2;)
      end
      i32.const 1220
      i32.load
      if ;; label = @2
        block ;; label = @3
          i32.const 1220
          local.set $0
          f32.const 0x0p+0 (;=0;)
          local.set $11
          loop $label$30 ;; label = @4
            local.get $11
            local.get $0
            i32.const 4
            i32.add
            local.tee $1
            f32.load
            f32.add
            local.tee $11
            f64.promote_f32
            local.tee $13
            f64.const 0x1p+0 (;=1;)
            f64.lt
            if (result f64) ;; label = @5
              local.get $13
            else
              f64.const 0x1p+0 (;=1;)
            end
            f32.demote_f64
            local.set $12
            local.get $1
            local.get $12
            f32.store
            local.get $0
            local.get $12
            f32.const 0x1p+9 (;=512;)
            f32.mul
            i32.trunc_f32_s
            i32.store offset=8
            local.get $0
            i32.const 12
            i32.add
            local.tee $0
            i32.load
            br_if 0 (;@4;)
            i32.const 0
            local.set $1
            i32.const 1220
            local.set $0
          end
        end
      else
        block ;; label = @3
          i32.const 0
          local.set $1
          i32.const 1220
          local.set $0
        end
      end
      loop $label$34 ;; label = @2
        loop $label$35 ;; label = @3
          local.get $0
          i32.const 12
          i32.add
          local.set $3
          local.get $1
          local.get $0
          i32.load offset=8
          local.tee $2
          i32.gt_u
          local.get $2
          i32.const 0
          i32.ne
          i32.and
          if ;; label = @4
            block ;; label = @5
              local.get $3
              local.set $0
              br 2 (;@3;)
            end
          end
        end
        local.get $7
        local.get $1
        i32.const 2
        i32.shl
        i32.add
        local.get $0
        i32.store
        local.get $1
        i32.const 1
        i32.add
        local.tee $1
        i32.const 513
        i32.ne
        br_if 0 (;@2;)
      end
      local.get $7
      i32.const 2116
      i32.add
      i32.const 0
      i32.store
      local.get $4
      i32.const 5
      i32.mul
      local.set $0
      loop $label$37 ;; label = @2
        local.get $7
        local.get $0
        i32.const 60
        i32.lt_u
        if (result i32) ;; label = @3
          local.get $0
        else
          i32.const 60
        end
        local.tee $1
        call $8
        local.get $0
        local.get $1
        i32.sub
        local.tee $0
        br_if 0 (;@2;)
        i32.const 0
        local.set $0
      end
      local.get $5
      global.set $global$1
      local.get $0
    end
  )
  (func $8 (;21;) (type $5) (param $0 i32) (param $1 i32)
    (local $2 i32) (local $3 i32) (local $4 i32) (local $5 i32) (local $6 f32)
    block $label$1 ;; label = @1
      local.get $1
      if ;; label = @2
        block ;; label = @3
          i32.const 0
          local.set $3
          i32.const 1396
          i32.load
          local.set $2
          loop $label$3 ;; label = @4
            local.get $0
            local.get $2
            i32.const 3877
            i32.mul
            i32.const 29573
            i32.add
            i32.const 139968
            i32.rem_u
            local.tee $4
            f32.convert_i32_u
            f32.const 0x1.116p+17 (;=139968;)
            f32.div
            local.tee $6
            f32.const 0x1p+9 (;=512;)
            f32.mul
            i32.trunc_f32_s
            i32.const 2
            i32.shl
            i32.add
            i32.load
            local.set $2
            loop $label$4 ;; label = @5
              local.get $2
              i32.const 12
              i32.add
              local.set $5
              local.get $2
              f32.load offset=4
              local.get $6
              f32.lt
              if ;; label = @6
                block ;; label = @7
                  local.get $5
                  local.set $2
                  br 2 (;@5;)
                end
              end
            end
            local.get $0
            i32.const 2052
            i32.add
            local.get $3
            i32.add
            local.get $2
            i32.load
            i32.store8
            local.get $3
            i32.const 1
            i32.add
            local.tee $3
            local.get $1
            i32.ne
            if ;; label = @5
              block ;; label = @6
                local.get $4
                local.set $2
                br 2 (;@4;)
              end
            end
          end
          i32.const 1396
          local.get $4
          i32.store
        end
      end
      local.get $0
      i32.const 2052
      i32.add
      local.get $1
      i32.add
      i32.const 10
      i32.store8
      local.get $0
      i32.const 2052
      i32.add
      local.get $1
      i32.const 1
      i32.add
      local.tee $1
      i32.add
      i32.const 0
      i32.store8
      local.get $0
      i32.const 2116
      i32.add
      local.get $1
      i32.store
      local.get $0
      i32.const 2052
      i32.add
      local.tee $1
      call $31
      local.tee $3
      i32.const 1024
      i32.load
      local.tee $2
      i32.le_s
      if ;; label = @2
        block ;; label = @3
          local.get $1
          call $35
          drop
          i32.const 1024
          i32.const 1024
          i32.load
          local.get $3
          i32.sub
          i32.store
          return
        end
      end
      local.get $2
      i32.const 0
      i32.le_s
      if ;; label = @2
        return
      end
      local.get $0
      i32.const 2052
      i32.add
      local.get $2
      i32.add
      i32.const 0
      i32.store8
      local.get $1
      call $35
      drop
      local.get $0
      i32.const 2052
      i32.add
      i32.const 1024
      i32.load
      i32.add
      i32.const 122
      i32.store8
      i32.const 1024
      i32.const 0
      i32.store
    end
  )
  (func $9 (;22;) (type $2) (param $0 i32) (result i32)
    (local $1 i32) (local $2 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $1
      global.get $global$1
      i32.const 16
      i32.add
      global.set $global$1
      local.get $1
      local.tee $2
      local.get $0
      i32.load offset=60
      i32.store
      i32.const 6
      local.get $2
      call $fimport$10
      call $11
      local.set $0
      local.get $1
      global.set $global$1
      local.get $0
    end
  )
  (func $10 (;23;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    (local $3 i32) (local $4 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $4
      global.get $global$1
      i32.const 32
      i32.add
      global.set $global$1
      local.get $4
      local.tee $3
      local.get $0
      i32.load offset=60
      i32.store
      local.get $3
      i32.const 0
      i32.store offset=4
      local.get $3
      local.get $1
      i32.store offset=8
      local.get $3
      local.get $4
      i32.const 20
      i32.add
      local.tee $0
      i32.store offset=12
      local.get $3
      local.get $2
      i32.store offset=16
      i32.const 140
      local.get $3
      call $fimport$14
      call $11
      i32.const 0
      i32.lt_s
      if (result i32) ;; label = @2
        block (result i32) ;; label = @3
          local.get $0
          i32.const -1
          i32.store
          i32.const -1
        end
      else
        local.get $0
        i32.load
      end
      local.set $0
      local.get $4
      global.set $global$1
      local.get $0
    end
  )
  (func $11 (;24;) (type $2) (param $0 i32) (result i32)
    local.get $0
    i32.const -4096
    i32.gt_u
    if (result i32) ;; label = @1
      block (result i32) ;; label = @2
        call $12
        i32.const 0
        local.get $0
        i32.sub
        i32.store
        i32.const -1
      end
    else
      local.get $0
    end
  )
  (func $12 (;25;) (type $4) (result i32)
    i32.const 4172
  )
  (func $13 (;26;) (type $3) (param $0 i32)
    nop
  )
  (func $14 (;27;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    (local $3 i32) (local $4 i32) (local $5 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $4
      global.get $global$1
      i32.const 80
      i32.add
      global.set $global$1
      local.get $4
      local.set $3
      local.get $4
      i32.const 12
      i32.add
      local.set $5
      local.get $0
      i32.const 3
      i32.store offset=36
      local.get $0
      i32.load
      i32.const 64
      i32.and
      i32.eqz
      if ;; label = @2
        block ;; label = @3
          local.get $3
          local.get $0
          i32.load offset=60
          i32.store
          local.get $3
          i32.const 21505
          i32.store offset=4
          local.get $3
          local.get $5
          i32.store offset=8
          i32.const 54
          local.get $3
          call $fimport$13
          if ;; label = @4
            local.get $0
            i32.const -1
            i32.store8 offset=75
          end
        end
      end
      local.get $0
      local.get $1
      local.get $2
      call $15
      local.set $0
      local.get $4
      global.set $global$1
      local.get $0
    end
  )
  (func $15 (;28;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    (local $3 i32) (local $4 i32) (local $5 i32) (local $6 i32) (local $7 i32) (local $8 i32) (local $9 i32) (local $10 i32) (local $11 i32) (local $12 i32) (local $13 i32) (local $14 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $8
      global.get $global$1
      i32.const 48
      i32.add
      global.set $global$1
      local.get $8
      i32.const 16
      i32.add
      local.set $9
      local.get $8
      local.set $10
      local.get $8
      i32.const 32
      i32.add
      local.tee $3
      local.get $0
      i32.const 28
      i32.add
      local.tee $6
      i32.load
      local.tee $4
      i32.store
      local.get $3
      local.get $0
      i32.const 20
      i32.add
      local.tee $11
      i32.load
      local.get $4
      i32.sub
      local.tee $5
      i32.store offset=4
      local.get $3
      local.get $1
      i32.store offset=8
      local.get $3
      local.get $2
      i32.store offset=12
      local.get $0
      i32.const 60
      i32.add
      local.set $13
      local.get $0
      i32.const 44
      i32.add
      local.set $14
      local.get $3
      local.set $1
      i32.const 2
      local.set $4
      local.get $5
      local.get $2
      i32.add
      local.set $12
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          block $label$4 ;; label = @4
            loop $label$5 ;; label = @5
              i32.const 4128
              i32.load
              if ;; label = @6
                block ;; label = @7
                  i32.const 1
                  local.get $0
                  call $fimport$9
                  local.get $10
                  local.get $13
                  i32.load
                  i32.store
                  local.get $10
                  local.get $1
                  i32.store offset=4
                  local.get $10
                  local.get $4
                  i32.store offset=8
                  i32.const 146
                  local.get $10
                  call $fimport$15
                  call $11
                  local.set $3
                  i32.const 0
                  call $fimport$7
                end
              else
                block ;; label = @7
                  local.get $9
                  local.get $13
                  i32.load
                  i32.store
                  local.get $9
                  local.get $1
                  i32.store offset=4
                  local.get $9
                  local.get $4
                  i32.store offset=8
                  i32.const 146
                  local.get $9
                  call $fimport$15
                  call $11
                  local.set $3
                end
              end
              local.get $12
              local.get $3
              i32.eq
              br_if 1 (;@4;)
              local.get $3
              i32.const 0
              i32.lt_s
              br_if 2 (;@3;)
              local.get $3
              local.get $1
              i32.load offset=4
              local.tee $5
              i32.gt_u
              if (result i32) ;; label = @6
                block (result i32) ;; label = @7
                  local.get $6
                  local.get $14
                  i32.load
                  local.tee $7
                  i32.store
                  local.get $11
                  local.get $7
                  i32.store
                  local.get $1
                  i32.load offset=12
                  local.set $7
                  local.get $1
                  i32.const 8
                  i32.add
                  local.set $1
                  local.get $4
                  i32.const -1
                  i32.add
                  local.set $4
                  local.get $3
                  local.get $5
                  i32.sub
                end
              else
                local.get $4
                i32.const 2
                i32.eq
                if (result i32) ;; label = @7
                  block (result i32) ;; label = @8
                    local.get $6
                    local.get $6
                    i32.load
                    local.get $3
                    i32.add
                    i32.store
                    local.get $5
                    local.set $7
                    i32.const 2
                    local.set $4
                    local.get $3
                  end
                else
                  block (result i32) ;; label = @8
                    local.get $5
                    local.set $7
                    local.get $3
                  end
                end
              end
              local.set $5
              local.get $1
              local.get $1
              i32.load
              local.get $5
              i32.add
              i32.store
              local.get $1
              local.get $7
              local.get $5
              i32.sub
              i32.store offset=4
              local.get $12
              local.get $3
              i32.sub
              local.set $12
              br 0 (;@5;)
            end
          end
          local.get $0
          local.get $14
          i32.load
          local.tee $1
          local.get $0
          i32.load offset=48
          i32.add
          i32.store offset=16
          local.get $6
          local.get $1
          i32.store
          local.get $11
          local.get $1
          i32.store
          br 1 (;@2;)
        end
        local.get $0
        i32.const 0
        i32.store offset=16
        local.get $6
        i32.const 0
        i32.store
        local.get $11
        i32.const 0
        i32.store
        local.get $0
        local.get $0
        i32.load
        i32.const 32
        i32.or
        i32.store
        local.get $4
        i32.const 2
        i32.eq
        if (result i32) ;; label = @3
          i32.const 0
        else
          local.get $2
          local.get $1
          i32.load offset=4
          i32.sub
        end
        local.set $2
      end
      local.get $8
      global.set $global$1
      local.get $2
    end
  )
  (func $16 (;29;) (type $3) (param $0 i32)
    local.get $0
    i32.load offset=68
    i32.eqz
    if ;; label = @1
      local.get $0
      call $13
    end
  )
  (func $17 (;30;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    (local $3 i32) (local $4 i32) (local $5 i32)
    block $label$1 (result i32) ;; label = @1
      local.get $1
      i32.const 255
      i32.and
      local.set $5
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          block $label$4 ;; label = @4
            local.get $2
            i32.const 0
            i32.ne
            local.tee $4
            local.get $0
            i32.const 3
            i32.and
            i32.const 0
            i32.ne
            i32.and
            if ;; label = @5
              block ;; label = @6
                local.get $1
                i32.const 255
                i32.and
                local.set $4
                local.get $2
                local.set $3
                local.get $0
                local.set $2
                loop $label$6 ;; label = @7
                  local.get $2
                  i32.load8_s
                  local.get $4
                  i32.const 24
                  i32.shl
                  i32.const 24
                  i32.shr_s
                  i32.eq
                  if ;; label = @8
                    block ;; label = @9
                      local.get $3
                      local.set $0
                      br 6 (;@3;)
                    end
                  end
                  local.get $3
                  i32.const -1
                  i32.add
                  local.tee $3
                  i32.const 0
                  i32.ne
                  local.tee $0
                  local.get $2
                  i32.const 1
                  i32.add
                  local.tee $2
                  i32.const 3
                  i32.and
                  i32.const 0
                  i32.ne
                  i32.and
                  br_if 0 (;@7;)
                  br 3 (;@4;)
                end
              end
            else
              block ;; label = @6
                local.get $2
                local.set $3
                local.get $0
                local.set $2
                local.get $4
                local.set $0
              end
            end
          end
          local.get $0
          if ;; label = @4
            block ;; label = @5
              local.get $3
              local.set $0
              br 2 (;@3;)
            end
          else
            i32.const 0
            local.set $0
          end
          br 1 (;@2;)
        end
        local.get $2
        i32.load8_s
        local.get $1
        i32.const 255
        i32.and
        local.tee $1
        i32.const 24
        i32.shl
        i32.const 24
        i32.shr_s
        i32.ne
        if ;; label = @3
          block ;; label = @4
            local.get $5
            i32.const 16843009
            i32.mul
            local.set $3
            block $label$12 ;; label = @5
              block $label$13 ;; label = @6
                local.get $0
                i32.const 3
                i32.le_u
                br_if 0 (;@6;)
                loop $label$14 ;; label = @7
                  local.get $2
                  i32.load
                  local.get $3
                  i32.xor
                  local.tee $4
                  i32.const -2139062144
                  i32.and
                  i32.const -2139062144
                  i32.xor
                  local.get $4
                  i32.const -16843009
                  i32.add
                  i32.and
                  i32.eqz
                  if ;; label = @8
                    block ;; label = @9
                      local.get $2
                      i32.const 4
                      i32.add
                      local.set $2
                      local.get $0
                      i32.const -4
                      i32.add
                      local.tee $0
                      i32.const 3
                      i32.gt_u
                      br_if 2 (;@7;)
                      br 3 (;@6;)
                    end
                  end
                end
                br 1 (;@5;)
              end
              local.get $0
              i32.eqz
              if ;; label = @6
                block ;; label = @7
                  i32.const 0
                  local.set $0
                  br 5 (;@2;)
                end
              end
            end
            loop $label$17 ;; label = @5
              local.get $2
              i32.load8_s
              local.get $1
              i32.const 24
              i32.shl
              i32.const 24
              i32.shr_s
              i32.eq
              br_if 3 (;@2;)
              local.get $2
              i32.const 1
              i32.add
              local.set $2
              local.get $0
              i32.const -1
              i32.add
              local.tee $0
              br_if 0 (;@5;)
              i32.const 0
              local.set $0
            end
          end
        end
      end
      local.get $0
      if (result i32) ;; label = @2
        local.get $2
      else
        i32.const 0
      end
    end
  )
  (func $18 (;31;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    (local $3 i32) (local $4 i32) (local $5 i32) (local $6 i32) (local $7 i32) (local $8 i32) (local $9 i32) (local $10 i32) (local $11 i32) (local $12 i32) (local $13 i32) (local $14 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $4
      global.get $global$1
      i32.const 224
      i32.add
      global.set $global$1
      local.get $4
      i32.const 136
      i32.add
      local.set $5
      local.get $4
      i32.const 80
      i32.add
      local.tee $3
      i64.const 0
      i64.store align=4
      local.get $3
      i64.const 0
      i64.store offset=8 align=4
      local.get $3
      i64.const 0
      i64.store offset=16 align=4
      local.get $3
      i64.const 0
      i64.store offset=24 align=4
      local.get $3
      i64.const 0
      i64.store offset=32 align=4
      local.get $4
      i32.const 120
      i32.add
      local.tee $6
      local.get $2
      i32.load
      i32.store
      i32.const 0
      local.get $1
      local.get $6
      local.get $4
      local.tee $2
      local.get $3
      call $19
      i32.const 0
      i32.lt_s
      if ;; label = @2
        i32.const -1
        local.set $1
      else
        block ;; label = @3
          local.get $0
          i32.load offset=76
          i32.const -1
          i32.gt_s
          if (result i32) ;; label = @4
            local.get $0
            call $20
          else
            i32.const 0
          end
          local.set $12
          local.get $0
          i32.load
          local.set $7
          local.get $0
          i32.load8_s offset=74
          i32.const 1
          i32.lt_s
          if ;; label = @4
            local.get $0
            local.get $7
            i32.const -33
            i32.and
            i32.store
          end
          local.get $0
          i32.const 48
          i32.add
          local.tee $8
          i32.load
          if ;; label = @4
            local.get $0
            local.get $1
            local.get $6
            local.get $2
            local.get $3
            call $19
            local.set $1
          else
            block ;; label = @5
              local.get $0
              i32.const 44
              i32.add
              local.tee $9
              i32.load
              local.set $10
              local.get $9
              local.get $5
              i32.store
              local.get $0
              i32.const 28
              i32.add
              local.tee $13
              local.get $5
              i32.store
              local.get $0
              i32.const 20
              i32.add
              local.tee $11
              local.get $5
              i32.store
              local.get $8
              i32.const 80
              i32.store
              local.get $0
              i32.const 16
              i32.add
              local.tee $14
              local.get $5
              i32.const 80
              i32.add
              i32.store
              local.get $0
              local.get $1
              local.get $6
              local.get $2
              local.get $3
              call $19
              local.set $1
              local.get $10
              if ;; label = @6
                block ;; label = @7
                  local.get $0
                  i32.const 0
                  i32.const 0
                  local.get $0
                  i32.load offset=36
                  i32.const 3
                  i32.and
                  i32.const 2
                  i32.add
                  call_indirect (type $0)
                  drop
                  local.get $11
                  i32.load
                  i32.eqz
                  if ;; label = @8
                    i32.const -1
                    local.set $1
                  end
                  local.get $9
                  local.get $10
                  i32.store
                  local.get $8
                  i32.const 0
                  i32.store
                  local.get $14
                  i32.const 0
                  i32.store
                  local.get $13
                  i32.const 0
                  i32.store
                  local.get $11
                  i32.const 0
                  i32.store
                end
              end
            end
          end
          local.get $0
          local.get $0
          i32.load
          local.tee $2
          local.get $7
          i32.const 32
          i32.and
          i32.or
          i32.store
          local.get $12
          if ;; label = @4
            local.get $0
            call $13
          end
          local.get $2
          i32.const 32
          i32.and
          if ;; label = @4
            i32.const -1
            local.set $1
          end
        end
      end
      local.get $4
      global.set $global$1
      local.get $1
    end
  )
  (func $19 (;32;) (type $7) (param $0 i32) (param $1 i32) (param $2 i32) (param $3 i32) (param $4 i32) (result i32)
    (local $5 i32) (local $6 i32) (local $7 i32) (local $8 i32) (local $9 i32) (local $10 i32) (local $11 i32) (local $12 i32) (local $13 i32) (local $14 i32) (local $15 i32) (local $16 i32) (local $17 i32) (local $18 i32) (local $19 i32) (local $20 i32) (local $21 i32) (local $22 i32) (local $23 i32) (local $24 i32) (local $25 i32) (local $26 i32) (local $27 i32) (local $28 i32) (local $29 i32) (local $30 i32) (local $31 i32) (local $32 i32) (local $33 i32) (local $34 i32) (local $35 i32) (local $36 i32) (local $37 i32) (local $38 i32) (local $39 i32) (local $40 i32) (local $41 i32) (local $42 i32) (local $43 i32) (local $44 i32) (local $45 i32) (local $46 i32) (local $47 i32) (local $48 i32) (local $49 i32) (local $50 i64) (local $51 i64) (local $52 f64) (local $53 f64)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $23
      global.get $global$1
      i32.const 624
      i32.add
      global.set $global$1
      local.get $23
      i32.const 16
      i32.add
      local.set $20
      local.get $23
      local.set $16
      local.get $23
      i32.const 528
      i32.add
      local.set $36
      local.get $0
      i32.const 0
      i32.ne
      local.set $30
      local.get $23
      i32.const 536
      i32.add
      local.tee $17
      i32.const 40
      i32.add
      local.tee $21
      local.set $38
      local.get $17
      i32.const 39
      i32.add
      local.set $39
      local.get $23
      i32.const 8
      i32.add
      local.tee $37
      i32.const 4
      i32.add
      local.set $42
      i32.const 0
      local.get $23
      i32.const 588
      i32.add
      local.tee $19
      local.tee $27
      i32.sub
      local.set $43
      local.get $23
      i32.const 576
      i32.add
      local.tee $17
      i32.const 12
      i32.add
      local.set $33
      local.get $17
      i32.const 11
      i32.add
      local.set $40
      local.get $33
      local.tee $28
      local.get $27
      i32.sub
      local.set $44
      i32.const -2
      local.get $27
      i32.sub
      local.set $45
      local.get $28
      i32.const 2
      i32.add
      local.set $46
      local.get $23
      i32.const 24
      i32.add
      local.tee $47
      i32.const 288
      i32.add
      local.set $48
      local.get $19
      i32.const 9
      i32.add
      local.tee $31
      local.set $41
      local.get $19
      i32.const 8
      i32.add
      local.set $34
      i32.const 0
      local.set $15
      i32.const 0
      local.set $10
      i32.const 0
      local.set $17
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          loop $label$4 ;; label = @4
            block $label$5 ;; label = @5
              local.get $15
              i32.const -1
              i32.gt_s
              if ;; label = @6
                local.get $10
                i32.const 2147483647
                local.get $15
                i32.sub
                i32.gt_s
                if (result i32) ;; label = @7
                  block (result i32) ;; label = @8
                    call $12
                    i32.const 75
                    i32.store
                    i32.const -1
                  end
                else
                  local.get $10
                  local.get $15
                  i32.add
                end
                local.set $15
              end
              local.get $1
              i32.load8_s
              local.tee $5
              i32.const 24
              i32.shl
              i32.const 24
              i32.shr_s
              i32.eqz
              br_if 2 (;@3;)
              local.get $1
              local.set $11
              block $label$9 ;; label = @6
                block $label$10 ;; label = @7
                  loop $label$11 ;; label = @8
                    block $label$12 ;; label = @9
                      block $label$13 ;; label = @10
                        block $label$14 ;; label = @11
                          block $label$15 ;; label = @12
                            local.get $5
                            i32.const 24
                            i32.shl
                            i32.const 24
                            i32.shr_s
                            i32.const 0
                            i32.sub
                            br_table 1 (;@11;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 2 (;@10;) 0 (;@12;) 2 (;@10;)
                          end
                          local.get $11
                          local.set $5
                          br 4 (;@7;)
                        end
                        local.get $11
                        local.set $5
                        br 1 (;@9;)
                      end
                      local.get $11
                      i32.const 1
                      i32.add
                      local.tee $11
                      i32.load8_s
                      local.set $5
                      br 1 (;@8;)
                    end
                  end
                  br 1 (;@6;)
                end
                loop $label$16 ;; label = @7
                  local.get $5
                  i32.load8_s offset=1
                  i32.const 37
                  i32.ne
                  br_if 1 (;@6;)
                  local.get $11
                  i32.const 1
                  i32.add
                  local.set $11
                  local.get $5
                  i32.const 2
                  i32.add
                  local.tee $5
                  i32.load8_s
                  i32.const 37
                  i32.eq
                  br_if 0 (;@7;)
                end
              end
              local.get $11
              local.get $1
              i32.sub
              local.set $10
              local.get $30
              if ;; label = @6
                local.get $0
                i32.load
                i32.const 32
                i32.and
                i32.eqz
                if ;; label = @7
                  local.get $1
                  local.get $10
                  local.get $0
                  call $21
                  drop
                end
              end
              local.get $10
              if ;; label = @6
                block ;; label = @7
                  local.get $5
                  local.set $1
                  br 3 (;@4;)
                end
              end
              local.get $5
              i32.const 1
              i32.add
              local.tee $11
              i32.load8_s
              local.tee $10
              i32.const 24
              i32.shl
              i32.const 24
              i32.shr_s
              i32.const -48
              i32.add
              local.tee $9
              i32.const 10
              i32.lt_u
              if (result i32) ;; label = @6
                block (result i32) ;; label = @7
                  local.get $5
                  i32.const 3
                  i32.add
                  local.set $10
                  local.get $5
                  i32.load8_s offset=2
                  i32.const 36
                  i32.eq
                  local.tee $12
                  if ;; label = @8
                    local.get $10
                    local.set $11
                  end
                  local.get $12
                  if ;; label = @8
                    i32.const 1
                    local.set $17
                  end
                  local.get $11
                  i32.load8_s
                  local.set $5
                  local.get $12
                  i32.eqz
                  if ;; label = @8
                    i32.const -1
                    local.set $9
                  end
                  local.get $17
                end
              else
                block (result i32) ;; label = @7
                  local.get $10
                  local.set $5
                  i32.const -1
                  local.set $9
                  local.get $17
                end
              end
              local.set $10
              block $label$25 ;; label = @6
                local.get $5
                i32.const 24
                i32.shl
                i32.const 24
                i32.shr_s
                i32.const -32
                i32.add
                local.tee $12
                i32.const 32
                i32.lt_u
                if ;; label = @7
                  block ;; label = @8
                    i32.const 0
                    local.set $17
                    loop $label$27 ;; label = @9
                      i32.const 1
                      local.get $12
                      i32.shl
                      i32.const 75913
                      i32.and
                      i32.eqz
                      br_if 3 (;@6;)
                      i32.const 1
                      local.get $5
                      i32.const 24
                      i32.shl
                      i32.const 24
                      i32.shr_s
                      i32.const -32
                      i32.add
                      i32.shl
                      local.get $17
                      i32.or
                      local.set $17
                      local.get $11
                      i32.const 1
                      i32.add
                      local.tee $11
                      i32.load8_s
                      local.tee $5
                      i32.const 24
                      i32.shl
                      i32.const 24
                      i32.shr_s
                      i32.const -32
                      i32.add
                      local.tee $12
                      i32.const 32
                      i32.lt_u
                      br_if 0 (;@9;)
                    end
                  end
                else
                  i32.const 0
                  local.set $17
                end
              end
              block $label$29 ;; label = @6
                local.get $5
                i32.const 24
                i32.shl
                i32.const 24
                i32.shr_s
                i32.const 42
                i32.eq
                if ;; label = @7
                  block ;; label = @8
                    block $label$31 (result i32) ;; label = @9
                      block $label$32 ;; label = @10
                        local.get $11
                        i32.const 1
                        i32.add
                        local.tee $7
                        i32.load8_s
                        local.tee $5
                        i32.const 24
                        i32.shl
                        i32.const 24
                        i32.shr_s
                        i32.const -48
                        i32.add
                        local.tee $12
                        i32.const 10
                        i32.ge_u
                        br_if 0 (;@10;)
                        local.get $11
                        i32.load8_s offset=2
                        i32.const 36
                        i32.ne
                        br_if 0 (;@10;)
                        local.get $4
                        local.get $12
                        i32.const 2
                        i32.shl
                        i32.add
                        i32.const 10
                        i32.store
                        i32.const 1
                        local.set $8
                        local.get $3
                        local.get $7
                        i32.load8_s
                        i32.const -48
                        i32.add
                        i32.const 3
                        i32.shl
                        i32.add
                        i64.load
                        i32.wrap_i64
                        local.set $10
                        local.get $11
                        i32.const 3
                        i32.add
                        br 1 (;@9;)
                      end
                      local.get $10
                      if ;; label = @10
                        block ;; label = @11
                          i32.const -1
                          local.set $15
                          br 6 (;@5;)
                        end
                      end
                      local.get $30
                      i32.eqz
                      if ;; label = @10
                        block ;; label = @11
                          local.get $17
                          local.set $12
                          i32.const 0
                          local.set $17
                          local.get $7
                          local.set $11
                          i32.const 0
                          local.set $10
                          br 5 (;@6;)
                        end
                      end
                      local.get $2
                      i32.load
                      i32.const 3
                      i32.add
                      i32.const -4
                      i32.and
                      local.tee $11
                      i32.load
                      local.set $10
                      local.get $2
                      local.get $11
                      i32.const 4
                      i32.add
                      i32.store
                      i32.const 0
                      local.set $8
                      local.get $7
                    end
                    local.set $11
                    local.get $17
                    i32.const 8192
                    i32.or
                    local.set $12
                    i32.const 0
                    local.get $10
                    i32.sub
                    local.set $7
                    local.get $11
                    i32.load8_s
                    local.set $5
                    local.get $10
                    i32.const 0
                    i32.lt_s
                    local.tee $6
                    i32.eqz
                    if ;; label = @9
                      local.get $17
                      local.set $12
                    end
                    local.get $8
                    local.set $17
                    local.get $6
                    if ;; label = @9
                      local.get $7
                      local.set $10
                    end
                  end
                else
                  local.get $5
                  i32.const 24
                  i32.shl
                  i32.const 24
                  i32.shr_s
                  i32.const -48
                  i32.add
                  local.tee $12
                  i32.const 10
                  i32.lt_u
                  if ;; label = @8
                    block ;; label = @9
                      i32.const 0
                      local.set $7
                      local.get $12
                      local.set $5
                      loop $label$39 ;; label = @10
                        local.get $7
                        i32.const 10
                        i32.mul
                        local.get $5
                        i32.add
                        local.set $7
                        local.get $11
                        i32.const 1
                        i32.add
                        local.tee $11
                        i32.load8_s
                        local.tee $12
                        i32.const 24
                        i32.shl
                        i32.const 24
                        i32.shr_s
                        i32.const -48
                        i32.add
                        local.tee $5
                        i32.const 10
                        i32.lt_u
                        br_if 0 (;@10;)
                      end
                      local.get $7
                      i32.const 0
                      i32.lt_s
                      if ;; label = @10
                        block ;; label = @11
                          i32.const -1
                          local.set $15
                          br 6 (;@5;)
                        end
                      else
                        block ;; label = @11
                          local.get $12
                          local.set $5
                          local.get $17
                          local.set $12
                          local.get $10
                          local.set $17
                          local.get $7
                          local.set $10
                        end
                      end
                    end
                  else
                    block ;; label = @9
                      local.get $17
                      local.set $12
                      local.get $10
                      local.set $17
                      i32.const 0
                      local.set $10
                    end
                  end
                end
              end
              block $label$43 ;; label = @6
                local.get $5
                i32.const 24
                i32.shl
                i32.const 24
                i32.shr_s
                i32.const 46
                i32.eq
                if ;; label = @7
                  block ;; label = @8
                    local.get $11
                    i32.const 1
                    i32.add
                    local.tee $7
                    i32.load8_s
                    local.tee $5
                    i32.const 24
                    i32.shl
                    i32.const 24
                    i32.shr_s
                    i32.const 42
                    i32.ne
                    if ;; label = @9
                      block ;; label = @10
                        local.get $5
                        i32.const 24
                        i32.shl
                        i32.const 24
                        i32.shr_s
                        i32.const -48
                        i32.add
                        local.tee $5
                        i32.const 10
                        i32.lt_u
                        if ;; label = @11
                          block ;; label = @12
                            local.get $7
                            local.set $11
                            i32.const 0
                            local.set $7
                          end
                        else
                          block ;; label = @12
                            i32.const 0
                            local.set $5
                            local.get $7
                            local.set $11
                            br 6 (;@6;)
                          end
                        end
                        loop $label$48 ;; label = @11
                          local.get $7
                          i32.const 10
                          i32.mul
                          local.get $5
                          i32.add
                          local.set $5
                          local.get $11
                          i32.const 1
                          i32.add
                          local.tee $11
                          i32.load8_s
                          i32.const -48
                          i32.add
                          local.tee $8
                          i32.const 10
                          i32.ge_u
                          br_if 5 (;@6;)
                          local.get $5
                          local.set $7
                          local.get $8
                          local.set $5
                          br 0 (;@11;)
                        end
                      end
                    end
                    local.get $11
                    i32.const 2
                    i32.add
                    local.tee $7
                    i32.load8_s
                    i32.const -48
                    i32.add
                    local.tee $5
                    i32.const 10
                    i32.lt_u
                    if ;; label = @9
                      local.get $11
                      i32.load8_s offset=3
                      i32.const 36
                      i32.eq
                      if ;; label = @10
                        block ;; label = @11
                          local.get $4
                          local.get $5
                          i32.const 2
                          i32.shl
                          i32.add
                          i32.const 10
                          i32.store
                          local.get $3
                          local.get $7
                          i32.load8_s
                          i32.const -48
                          i32.add
                          i32.const 3
                          i32.shl
                          i32.add
                          i64.load
                          i32.wrap_i64
                          local.set $5
                          local.get $11
                          i32.const 4
                          i32.add
                          local.set $11
                          br 5 (;@6;)
                        end
                      end
                    end
                    local.get $17
                    if ;; label = @9
                      block ;; label = @10
                        i32.const -1
                        local.set $15
                        br 5 (;@5;)
                      end
                    end
                    local.get $30
                    if (result i32) ;; label = @9
                      block (result i32) ;; label = @10
                        local.get $2
                        i32.load
                        i32.const 3
                        i32.add
                        i32.const -4
                        i32.and
                        local.tee $11
                        i32.load
                        local.set $5
                        local.get $2
                        local.get $11
                        i32.const 4
                        i32.add
                        i32.store
                        local.get $7
                      end
                    else
                      block (result i32) ;; label = @10
                        i32.const 0
                        local.set $5
                        local.get $7
                      end
                    end
                    local.set $11
                  end
                else
                  i32.const -1
                  local.set $5
                end
              end
              local.get $11
              local.set $7
              i32.const 0
              local.set $8
              loop $label$55 ;; label = @6
                local.get $7
                i32.load8_s
                i32.const -65
                i32.add
                local.tee $6
                i32.const 57
                i32.gt_u
                if ;; label = @7
                  block ;; label = @8
                    i32.const -1
                    local.set $15
                    br 3 (;@5;)
                  end
                end
                local.get $7
                i32.const 1
                i32.add
                local.set $11
                local.get $8
                i32.const 58
                i32.mul
                i32.const 1699
                i32.add
                local.get $6
                i32.add
                i32.load8_s
                local.tee $13
                i32.const 255
                i32.and
                local.tee $6
                i32.const -1
                i32.add
                i32.const 8
                i32.lt_u
                if ;; label = @7
                  block ;; label = @8
                    local.get $11
                    local.set $7
                    local.get $6
                    local.set $8
                    br 2 (;@6;)
                  end
                end
              end
              local.get $13
              i32.const 24
              i32.shl
              i32.const 24
              i32.shr_s
              i32.eqz
              if ;; label = @6
                block ;; label = @7
                  i32.const -1
                  local.set $15
                  br 2 (;@5;)
                end
              end
              local.get $9
              i32.const -1
              i32.gt_s
              local.set $14
              block $label$59 ;; label = @6
                block $label$60 ;; label = @7
                  local.get $13
                  i32.const 24
                  i32.shl
                  i32.const 24
                  i32.shr_s
                  i32.const 19
                  i32.eq
                  if ;; label = @8
                    local.get $14
                    if ;; label = @9
                      block ;; label = @10
                        i32.const -1
                        local.set $15
                        br 5 (;@5;)
                      end
                    else
                      br 2 (;@7;)
                    end
                  else
                    block ;; label = @9
                      local.get $14
                      if ;; label = @10
                        block ;; label = @11
                          local.get $4
                          local.get $9
                          i32.const 2
                          i32.shl
                          i32.add
                          local.get $6
                          i32.store
                          local.get $16
                          local.get $3
                          local.get $9
                          i32.const 3
                          i32.shl
                          i32.add
                          i64.load
                          i64.store
                          br 4 (;@7;)
                        end
                      end
                      local.get $30
                      i32.eqz
                      if ;; label = @10
                        block ;; label = @11
                          i32.const 0
                          local.set $15
                          br 6 (;@5;)
                        end
                      end
                      local.get $16
                      local.get $6
                      local.get $2
                      call $22
                    end
                  end
                  br 1 (;@6;)
                end
                local.get $30
                i32.eqz
                if ;; label = @7
                  block ;; label = @8
                    i32.const 0
                    local.set $10
                    local.get $11
                    local.set $1
                    br 4 (;@4;)
                  end
                end
              end
              local.get $7
              i32.load8_s
              local.tee $7
              i32.const -33
              i32.and
              local.set $9
              local.get $8
              i32.const 0
              i32.ne
              local.get $7
              i32.const 15
              i32.and
              i32.const 3
              i32.eq
              i32.and
              i32.eqz
              if ;; label = @6
                local.get $7
                local.set $9
              end
              local.get $12
              i32.const -65537
              i32.and
              local.set $7
              local.get $12
              i32.const 8192
              i32.and
              if ;; label = @6
                local.get $7
                local.set $12
              end
              block $label$70 ;; label = @6
                block $label$71 ;; label = @7
                  block $label$72 ;; label = @8
                    block $label$73 ;; label = @9
                      block $label$74 ;; label = @10
                        block $label$75 ;; label = @11
                          block $label$76 ;; label = @12
                            block $label$77 ;; label = @13
                              block $label$78 ;; label = @14
                                block $label$79 ;; label = @15
                                  block $label$80 ;; label = @16
                                    block $label$81 ;; label = @17
                                      block $label$82 ;; label = @18
                                        block $label$83 ;; label = @19
                                          block $label$84 ;; label = @20
                                            block $label$85 ;; label = @21
                                              block $label$86 ;; label = @22
                                                block $label$87 ;; label = @23
                                                  block $label$88 ;; label = @24
                                                    block $label$89 ;; label = @25
                                                      local.get $9
                                                      i32.const 65
                                                      i32.sub
                                                      br_table 11 (;@14;) 12 (;@13;) 9 (;@16;) 12 (;@13;) 11 (;@14;) 11 (;@14;) 11 (;@14;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 10 (;@15;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 2 (;@23;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 11 (;@14;) 12 (;@13;) 6 (;@19;) 4 (;@21;) 11 (;@14;) 11 (;@14;) 11 (;@14;) 12 (;@13;) 4 (;@21;) 12 (;@13;) 12 (;@13;) 12 (;@13;) 7 (;@18;) 0 (;@25;) 3 (;@22;) 1 (;@24;) 12 (;@13;) 12 (;@13;) 8 (;@17;) 12 (;@13;) 5 (;@20;) 12 (;@13;) 12 (;@13;) 2 (;@23;) 12 (;@13;)
                                                    end
                                                    block $label$90 ;; label = @25
                                                      block $label$91 ;; label = @26
                                                        block $label$92 ;; label = @27
                                                          block $label$93 ;; label = @28
                                                            block $label$94 ;; label = @29
                                                              block $label$95 ;; label = @30
                                                                block $label$96 ;; label = @31
                                                                  block $label$97 ;; label = @32
                                                                    local.get $8
                                                                    i32.const 255
                                                                    i32.and
                                                                    i32.const 24
                                                                    i32.shl
                                                                    i32.const 24
                                                                    i32.shr_s
                                                                    i32.const 0
                                                                    i32.sub
                                                                    br_table 0 (;@32;) 1 (;@31;) 2 (;@30;) 3 (;@29;) 4 (;@28;) 7 (;@25;) 5 (;@27;) 6 (;@26;) 7 (;@25;)
                                                                  end
                                                                  local.get $16
                                                                  i32.load
                                                                  local.get $15
                                                                  i32.store
                                                                  i32.const 0
                                                                  local.set $10
                                                                  local.get $11
                                                                  local.set $1
                                                                  br 27 (;@4;)
                                                                end
                                                                local.get $16
                                                                i32.load
                                                                local.get $15
                                                                i32.store
                                                                i32.const 0
                                                                local.set $10
                                                                local.get $11
                                                                local.set $1
                                                                br 26 (;@4;)
                                                              end
                                                              local.get $16
                                                              i32.load
                                                              local.get $15
                                                              i64.extend_i32_s
                                                              i64.store
                                                              i32.const 0
                                                              local.set $10
                                                              local.get $11
                                                              local.set $1
                                                              br 25 (;@4;)
                                                            end
                                                            local.get $16
                                                            i32.load
                                                            local.get $15
                                                            i32.store16
                                                            i32.const 0
                                                            local.set $10
                                                            local.get $11
                                                            local.set $1
                                                            br 24 (;@4;)
                                                          end
                                                          local.get $16
                                                          i32.load
                                                          local.get $15
                                                          i32.store8
                                                          i32.const 0
                                                          local.set $10
                                                          local.get $11
                                                          local.set $1
                                                          br 23 (;@4;)
                                                        end
                                                        local.get $16
                                                        i32.load
                                                        local.get $15
                                                        i32.store
                                                        i32.const 0
                                                        local.set $10
                                                        local.get $11
                                                        local.set $1
                                                        br 22 (;@4;)
                                                      end
                                                      local.get $16
                                                      i32.load
                                                      local.get $15
                                                      i64.extend_i32_s
                                                      i64.store
                                                      i32.const 0
                                                      local.set $10
                                                      local.get $11
                                                      local.set $1
                                                      br 21 (;@4;)
                                                    end
                                                    i32.const 0
                                                    local.set $10
                                                    local.get $11
                                                    local.set $1
                                                    br 20 (;@4;)
                                                  end
                                                  local.get $12
                                                  i32.const 8
                                                  i32.or
                                                  local.set $12
                                                  local.get $5
                                                  i32.const 8
                                                  i32.le_u
                                                  if ;; label = @24
                                                    i32.const 8
                                                    local.set $5
                                                  end
                                                  i32.const 120
                                                  local.set $9
                                                  br 11 (;@12;)
                                                end
                                                br 10 (;@12;)
                                              end
                                              local.get $16
                                              i64.load
                                              local.tee $50
                                              i64.const 0
                                              i64.eq
                                              if ;; label = @22
                                                local.get $21
                                                local.set $7
                                              else
                                                block ;; label = @23
                                                  local.get $21
                                                  local.set $1
                                                  loop $label$101 ;; label = @24
                                                    local.get $1
                                                    i32.const -1
                                                    i32.add
                                                    local.tee $1
                                                    local.get $50
                                                    i64.const 7
                                                    i64.and
                                                    i64.const 48
                                                    i64.or
                                                    i64.store8
                                                    local.get $50
                                                    i64.const 3
                                                    i64.shr_u
                                                    local.tee $50
                                                    i64.const 0
                                                    i64.ne
                                                    br_if 0 (;@24;)
                                                    local.get $1
                                                    local.set $7
                                                  end
                                                end
                                              end
                                              local.get $12
                                              i32.const 8
                                              i32.and
                                              if ;; label = @22
                                                block ;; label = @23
                                                  local.get $38
                                                  local.get $7
                                                  i32.sub
                                                  local.tee $1
                                                  i32.const 1
                                                  i32.add
                                                  local.set $8
                                                  local.get $5
                                                  local.get $1
                                                  i32.le_s
                                                  if ;; label = @24
                                                    local.get $8
                                                    local.set $5
                                                  end
                                                  i32.const 0
                                                  local.set $6
                                                  i32.const 2179
                                                  local.set $8
                                                  br 16 (;@7;)
                                                end
                                              else
                                                block ;; label = @23
                                                  i32.const 0
                                                  local.set $6
                                                  i32.const 2179
                                                  local.set $8
                                                  br 16 (;@7;)
                                                end
                                              end
                                            end
                                            local.get $16
                                            i64.load
                                            local.tee $50
                                            i64.const 0
                                            i64.lt_s
                                            if ;; label = @21
                                              block ;; label = @22
                                                local.get $16
                                                i64.const 0
                                                local.get $50
                                                i64.sub
                                                local.tee $50
                                                i64.store
                                                i32.const 1
                                                local.set $6
                                                i32.const 2179
                                                local.set $8
                                                br 11 (;@11;)
                                              end
                                            end
                                            local.get $12
                                            i32.const 2048
                                            i32.and
                                            if ;; label = @21
                                              block ;; label = @22
                                                i32.const 1
                                                local.set $6
                                                i32.const 2180
                                                local.set $8
                                                br 11 (;@11;)
                                              end
                                            else
                                              block ;; label = @22
                                                local.get $12
                                                i32.const 1
                                                i32.and
                                                local.tee $1
                                                local.set $6
                                                local.get $1
                                                if (result i32) ;; label = @23
                                                  i32.const 2181
                                                else
                                                  i32.const 2179
                                                end
                                                local.set $8
                                                br 11 (;@11;)
                                              end
                                            end
                                          end
                                          local.get $16
                                          i64.load
                                          local.set $50
                                          i32.const 0
                                          local.set $6
                                          i32.const 2179
                                          local.set $8
                                          br 8 (;@11;)
                                        end
                                        local.get $39
                                        local.get $16
                                        i64.load
                                        i64.store8
                                        local.get $39
                                        local.set $1
                                        local.get $7
                                        local.set $12
                                        i32.const 1
                                        local.set $7
                                        i32.const 0
                                        local.set $6
                                        i32.const 2179
                                        local.set $8
                                        local.get $21
                                        local.set $5
                                        br 12 (;@6;)
                                      end
                                      call $12
                                      i32.load
                                      call $24
                                      local.set $1
                                      br 7 (;@10;)
                                    end
                                    local.get $16
                                    i32.load
                                    local.tee $1
                                    i32.eqz
                                    if ;; label = @17
                                      i32.const 2189
                                      local.set $1
                                    end
                                    br 6 (;@10;)
                                  end
                                  local.get $37
                                  local.get $16
                                  i64.load
                                  i64.store32
                                  local.get $42
                                  i32.const 0
                                  i32.store
                                  local.get $16
                                  local.get $37
                                  i32.store
                                  local.get $37
                                  local.set $7
                                  i32.const -1
                                  local.set $6
                                  br 6 (;@9;)
                                end
                                local.get $16
                                i32.load
                                local.set $7
                                local.get $5
                                if ;; label = @15
                                  block ;; label = @16
                                    local.get $5
                                    local.set $6
                                    br 7 (;@9;)
                                  end
                                else
                                  block ;; label = @16
                                    local.get $0
                                    i32.const 32
                                    local.get $10
                                    i32.const 0
                                    local.get $12
                                    call $25
                                    i32.const 0
                                    local.set $1
                                    br 8 (;@8;)
                                  end
                                end
                              end
                              local.get $16
                              f64.load
                              local.set $52
                              local.get $20
                              i32.const 0
                              i32.store
                              local.get $52
                              i64.reinterpret_f64
                              i64.const 0
                              i64.lt_s
                              if (result i32) ;; label = @14
                                block (result i32) ;; label = @15
                                  i32.const 1
                                  local.set $24
                                  local.get $52
                                  f64.neg
                                  local.set $52
                                  i32.const 2196
                                end
                              else
                                block (result i32) ;; label = @15
                                  local.get $12
                                  i32.const 1
                                  i32.and
                                  local.set $1
                                  local.get $12
                                  i32.const 2048
                                  i32.and
                                  if (result i32) ;; label = @16
                                    block (result i32) ;; label = @17
                                      i32.const 1
                                      local.set $24
                                      i32.const 2199
                                    end
                                  else
                                    block (result i32) ;; label = @17
                                      local.get $1
                                      local.set $24
                                      local.get $1
                                      if (result i32) ;; label = @18
                                        i32.const 2202
                                      else
                                        i32.const 2197
                                      end
                                    end
                                  end
                                end
                              end
                              local.set $26
                              block $label$119 ;; label = @14
                                local.get $52
                                i64.reinterpret_f64
                                i64.const 9218868437227405312
                                i64.and
                                i64.const 9218868437227405312
                                i64.lt_u
                                if ;; label = @15
                                  block ;; label = @16
                                    local.get $52
                                    local.get $20
                                    call $27
                                    f64.const 0x1p+1 (;=2;)
                                    f64.mul
                                    local.tee $52
                                    f64.const 0x0p+0 (;=0;)
                                    f64.ne
                                    local.tee $1
                                    if ;; label = @17
                                      local.get $20
                                      local.get $20
                                      i32.load
                                      i32.const -1
                                      i32.add
                                      i32.store
                                    end
                                    local.get $9
                                    i32.const 32
                                    i32.or
                                    local.tee $22
                                    i32.const 97
                                    i32.eq
                                    if ;; label = @17
                                      block ;; label = @18
                                        local.get $26
                                        i32.const 9
                                        i32.add
                                        local.set $1
                                        local.get $9
                                        i32.const 32
                                        i32.and
                                        local.tee $6
                                        if ;; label = @19
                                          local.get $1
                                          local.set $26
                                        end
                                        local.get $5
                                        i32.const 11
                                        i32.gt_u
                                        i32.const 12
                                        local.get $5
                                        i32.sub
                                        local.tee $1
                                        i32.eqz
                                        i32.or
                                        i32.eqz
                                        if ;; label = @19
                                          block ;; label = @20
                                            f64.const 0x1p+3 (;=8;)
                                            local.set $53
                                            loop $label$125 ;; label = @21
                                              local.get $53
                                              f64.const 0x1p+4 (;=16;)
                                              f64.mul
                                              local.set $53
                                              local.get $1
                                              i32.const -1
                                              i32.add
                                              local.tee $1
                                              br_if 0 (;@21;)
                                            end
                                            local.get $26
                                            i32.load8_s
                                            i32.const 45
                                            i32.eq
                                            if (result f64) ;; label = @21
                                              local.get $53
                                              local.get $52
                                              f64.neg
                                              local.get $53
                                              f64.sub
                                              f64.add
                                              f64.neg
                                            else
                                              local.get $52
                                              local.get $53
                                              f64.add
                                              local.get $53
                                              f64.sub
                                            end
                                            local.set $52
                                          end
                                        end
                                        i32.const 0
                                        local.get $20
                                        i32.load
                                        local.tee $7
                                        i32.sub
                                        local.set $1
                                        local.get $7
                                        i32.const 0
                                        i32.lt_s
                                        if (result i32) ;; label = @19
                                          local.get $1
                                        else
                                          local.get $7
                                        end
                                        i64.extend_i32_s
                                        local.get $33
                                        call $23
                                        local.tee $1
                                        local.get $33
                                        i32.eq
                                        if ;; label = @19
                                          block ;; label = @20
                                            local.get $40
                                            i32.const 48
                                            i32.store8
                                            local.get $40
                                            local.set $1
                                          end
                                        end
                                        local.get $24
                                        i32.const 2
                                        i32.or
                                        local.set $13
                                        local.get $1
                                        i32.const -1
                                        i32.add
                                        local.get $7
                                        i32.const 31
                                        i32.shr_s
                                        i32.const 2
                                        i32.and
                                        i32.const 43
                                        i32.add
                                        i32.store8
                                        local.get $1
                                        i32.const -2
                                        i32.add
                                        local.tee $8
                                        local.get $9
                                        i32.const 15
                                        i32.add
                                        i32.store8
                                        local.get $5
                                        i32.const 1
                                        i32.lt_s
                                        local.set $9
                                        local.get $12
                                        i32.const 8
                                        i32.and
                                        i32.eqz
                                        local.set $14
                                        local.get $19
                                        local.set $1
                                        loop $label$131 ;; label = @19
                                          local.get $1
                                          local.get $52
                                          i32.trunc_f64_s
                                          local.tee $7
                                          i32.const 2163
                                          i32.add
                                          i32.load8_u
                                          local.get $6
                                          i32.or
                                          i32.store8
                                          local.get $52
                                          local.get $7
                                          f64.convert_i32_s
                                          f64.sub
                                          f64.const 0x1p+4 (;=16;)
                                          f64.mul
                                          local.set $52
                                          block $label$132 (result i32) ;; label = @20
                                            local.get $1
                                            i32.const 1
                                            i32.add
                                            local.tee $7
                                            local.get $27
                                            i32.sub
                                            i32.const 1
                                            i32.eq
                                            if (result i32) ;; label = @21
                                              block (result i32) ;; label = @22
                                                local.get $7
                                                local.get $14
                                                local.get $9
                                                local.get $52
                                                f64.const 0x0p+0 (;=0;)
                                                f64.eq
                                                i32.and
                                                i32.and
                                                br_if 2 (;@20;)
                                                drop
                                                local.get $7
                                                i32.const 46
                                                i32.store8
                                                local.get $1
                                                i32.const 2
                                                i32.add
                                              end
                                            else
                                              local.get $7
                                            end
                                          end
                                          local.set $1
                                          local.get $52
                                          f64.const 0x0p+0 (;=0;)
                                          f64.ne
                                          br_if 0 (;@19;)
                                        end
                                        local.get $46
                                        local.get $5
                                        i32.add
                                        local.get $8
                                        local.tee $7
                                        i32.sub
                                        local.set $6
                                        local.get $44
                                        local.get $7
                                        i32.sub
                                        local.get $1
                                        i32.add
                                        local.set $9
                                        local.get $0
                                        i32.const 32
                                        local.get $10
                                        local.get $5
                                        i32.const 0
                                        i32.ne
                                        local.get $45
                                        local.get $1
                                        i32.add
                                        local.get $5
                                        i32.lt_s
                                        i32.and
                                        if (result i32) ;; label = @19
                                          local.get $6
                                        else
                                          local.get $9
                                          local.tee $6
                                        end
                                        local.get $13
                                        i32.add
                                        local.tee $5
                                        local.get $12
                                        call $25
                                        local.get $0
                                        i32.load
                                        i32.const 32
                                        i32.and
                                        i32.eqz
                                        if ;; label = @19
                                          local.get $26
                                          local.get $13
                                          local.get $0
                                          call $21
                                          drop
                                        end
                                        local.get $0
                                        i32.const 48
                                        local.get $10
                                        local.get $5
                                        local.get $12
                                        i32.const 65536
                                        i32.xor
                                        call $25
                                        local.get $1
                                        local.get $27
                                        i32.sub
                                        local.set $1
                                        local.get $0
                                        i32.load
                                        i32.const 32
                                        i32.and
                                        i32.eqz
                                        if ;; label = @19
                                          local.get $19
                                          local.get $1
                                          local.get $0
                                          call $21
                                          drop
                                        end
                                        local.get $0
                                        i32.const 48
                                        local.get $6
                                        local.get $1
                                        local.get $28
                                        local.get $7
                                        i32.sub
                                        local.tee $1
                                        i32.add
                                        i32.sub
                                        i32.const 0
                                        i32.const 0
                                        call $25
                                        local.get $0
                                        i32.load
                                        i32.const 32
                                        i32.and
                                        i32.eqz
                                        if ;; label = @19
                                          local.get $8
                                          local.get $1
                                          local.get $0
                                          call $21
                                          drop
                                        end
                                        local.get $0
                                        i32.const 32
                                        local.get $10
                                        local.get $5
                                        local.get $12
                                        i32.const 8192
                                        i32.xor
                                        call $25
                                        local.get $5
                                        local.get $10
                                        i32.ge_s
                                        if ;; label = @19
                                          local.get $5
                                          local.set $10
                                        end
                                        br 4 (;@14;)
                                      end
                                    end
                                    local.get $1
                                    if ;; label = @17
                                      block ;; label = @18
                                        local.get $20
                                        local.get $20
                                        i32.load
                                        i32.const -28
                                        i32.add
                                        local.tee $6
                                        i32.store
                                        local.get $52
                                        f64.const 0x1p+28 (;=268435456;)
                                        f64.mul
                                        local.set $52
                                      end
                                    else
                                      local.get $20
                                      i32.load
                                      local.set $6
                                    end
                                    local.get $6
                                    i32.const 0
                                    i32.lt_s
                                    if (result i32) ;; label = @17
                                      local.get $47
                                    else
                                      local.get $48
                                    end
                                    local.tee $7
                                    local.set $8
                                    loop $label$145 ;; label = @17
                                      local.get $8
                                      local.get $52
                                      i32.trunc_f64_s
                                      local.tee $1
                                      i32.store
                                      local.get $8
                                      i32.const 4
                                      i32.add
                                      local.set $8
                                      local.get $52
                                      local.get $1
                                      f64.convert_i32_u
                                      f64.sub
                                      f64.const 0x1.dcd65p+29 (;=1000000000;)
                                      f64.mul
                                      local.tee $52
                                      f64.const 0x0p+0 (;=0;)
                                      f64.ne
                                      br_if 0 (;@17;)
                                    end
                                    local.get $6
                                    i32.const 0
                                    i32.gt_s
                                    if ;; label = @17
                                      block ;; label = @18
                                        local.get $7
                                        local.set $1
                                        loop $label$147 ;; label = @19
                                          local.get $6
                                          i32.const 29
                                          i32.gt_s
                                          if (result i32) ;; label = @20
                                            i32.const 29
                                          else
                                            local.get $6
                                          end
                                          local.set $14
                                          block $label$150 ;; label = @20
                                            local.get $8
                                            i32.const -4
                                            i32.add
                                            local.tee $6
                                            local.get $1
                                            i32.ge_u
                                            if ;; label = @21
                                              block ;; label = @22
                                                local.get $14
                                                i64.extend_i32_u
                                                local.set $50
                                                i32.const 0
                                                local.set $13
                                                loop $label$152 ;; label = @23
                                                  local.get $6
                                                  local.get $6
                                                  i32.load
                                                  i64.extend_i32_u
                                                  local.get $50
                                                  i64.shl
                                                  local.get $13
                                                  i64.extend_i32_u
                                                  i64.add
                                                  local.tee $51
                                                  i64.const 1000000000
                                                  i64.rem_u
                                                  i64.store32
                                                  local.get $51
                                                  i64.const 1000000000
                                                  i64.div_u
                                                  i32.wrap_i64
                                                  local.set $13
                                                  local.get $6
                                                  i32.const -4
                                                  i32.add
                                                  local.tee $6
                                                  local.get $1
                                                  i32.ge_u
                                                  br_if 0 (;@23;)
                                                end
                                                local.get $13
                                                i32.eqz
                                                br_if 2 (;@20;)
                                                local.get $1
                                                i32.const -4
                                                i32.add
                                                local.tee $1
                                                local.get $13
                                                i32.store
                                              end
                                            end
                                          end
                                          loop $label$153 ;; label = @20
                                            local.get $8
                                            local.get $1
                                            i32.gt_u
                                            if ;; label = @21
                                              local.get $8
                                              i32.const -4
                                              i32.add
                                              local.tee $6
                                              i32.load
                                              i32.eqz
                                              if ;; label = @22
                                                block ;; label = @23
                                                  local.get $6
                                                  local.set $8
                                                  br 3 (;@20;)
                                                end
                                              end
                                            end
                                          end
                                          local.get $20
                                          local.get $20
                                          i32.load
                                          local.get $14
                                          i32.sub
                                          local.tee $6
                                          i32.store
                                          local.get $6
                                          i32.const 0
                                          i32.gt_s
                                          br_if 0 (;@19;)
                                        end
                                      end
                                    else
                                      local.get $7
                                      local.set $1
                                    end
                                    local.get $5
                                    i32.const 0
                                    i32.lt_s
                                    if (result i32) ;; label = @17
                                      i32.const 6
                                    else
                                      local.get $5
                                    end
                                    local.set $18
                                    local.get $6
                                    i32.const 0
                                    i32.lt_s
                                    if ;; label = @17
                                      block ;; label = @18
                                        local.get $18
                                        i32.const 25
                                        i32.add
                                        i32.const 9
                                        i32.div_s
                                        i32.const 1
                                        i32.add
                                        local.set $14
                                        local.get $22
                                        i32.const 102
                                        i32.eq
                                        local.set $25
                                        local.get $8
                                        local.set $5
                                        loop $label$160 ;; label = @19
                                          i32.const 0
                                          local.get $6
                                          i32.sub
                                          local.tee $13
                                          i32.const 9
                                          i32.gt_s
                                          if ;; label = @20
                                            i32.const 9
                                            local.set $13
                                          end
                                          block $label$162 ;; label = @20
                                            local.get $1
                                            local.get $5
                                            i32.lt_u
                                            if ;; label = @21
                                              block ;; label = @22
                                                i32.const 1
                                                local.get $13
                                                i32.shl
                                                i32.const -1
                                                i32.add
                                                local.set $29
                                                i32.const 1000000000
                                                local.get $13
                                                i32.shr_u
                                                local.set $35
                                                i32.const 0
                                                local.set $6
                                                local.get $1
                                                local.set $8
                                                loop $label$164 ;; label = @23
                                                  local.get $8
                                                  local.get $8
                                                  i32.load
                                                  local.tee $32
                                                  local.get $13
                                                  i32.shr_u
                                                  local.get $6
                                                  i32.add
                                                  i32.store
                                                  local.get $32
                                                  local.get $29
                                                  i32.and
                                                  local.get $35
                                                  i32.mul
                                                  local.set $6
                                                  local.get $8
                                                  i32.const 4
                                                  i32.add
                                                  local.tee $8
                                                  local.get $5
                                                  i32.lt_u
                                                  br_if 0 (;@23;)
                                                end
                                                local.get $1
                                                i32.const 4
                                                i32.add
                                                local.set $8
                                                local.get $1
                                                i32.load
                                                i32.eqz
                                                if ;; label = @23
                                                  local.get $8
                                                  local.set $1
                                                end
                                                local.get $6
                                                i32.eqz
                                                br_if 2 (;@20;)
                                                local.get $5
                                                local.get $6
                                                i32.store
                                                local.get $5
                                                i32.const 4
                                                i32.add
                                                local.set $5
                                              end
                                            else
                                              block ;; label = @22
                                                local.get $1
                                                i32.const 4
                                                i32.add
                                                local.set $8
                                                local.get $1
                                                i32.load
                                                i32.eqz
                                                if ;; label = @23
                                                  local.get $8
                                                  local.set $1
                                                end
                                              end
                                            end
                                          end
                                          local.get $25
                                          if (result i32) ;; label = @20
                                            local.get $7
                                          else
                                            local.get $1
                                          end
                                          local.tee $8
                                          local.get $14
                                          i32.const 2
                                          i32.shl
                                          i32.add
                                          local.set $6
                                          local.get $5
                                          local.get $8
                                          i32.sub
                                          i32.const 2
                                          i32.shr_s
                                          local.get $14
                                          i32.gt_s
                                          if ;; label = @20
                                            local.get $6
                                            local.set $5
                                          end
                                          local.get $20
                                          local.get $20
                                          i32.load
                                          local.get $13
                                          i32.add
                                          local.tee $6
                                          i32.store
                                          local.get $6
                                          i32.const 0
                                          i32.lt_s
                                          br_if 0 (;@19;)
                                          local.get $5
                                          local.set $13
                                        end
                                      end
                                    else
                                      local.get $8
                                      local.set $13
                                    end
                                    local.get $7
                                    local.set $25
                                    block $label$172 ;; label = @17
                                      local.get $1
                                      local.get $13
                                      i32.lt_u
                                      if ;; label = @18
                                        block ;; label = @19
                                          local.get $25
                                          local.get $1
                                          i32.sub
                                          i32.const 2
                                          i32.shr_s
                                          i32.const 9
                                          i32.mul
                                          local.set $5
                                          local.get $1
                                          i32.load
                                          local.tee $6
                                          i32.const 10
                                          i32.lt_u
                                          br_if 2 (;@17;)
                                          i32.const 10
                                          local.set $8
                                          loop $label$174 ;; label = @20
                                            local.get $5
                                            i32.const 1
                                            i32.add
                                            local.set $5
                                            local.get $6
                                            local.get $8
                                            i32.const 10
                                            i32.mul
                                            local.tee $8
                                            i32.ge_u
                                            br_if 0 (;@20;)
                                          end
                                        end
                                      else
                                        i32.const 0
                                        local.set $5
                                      end
                                    end
                                    local.get $22
                                    i32.const 103
                                    i32.eq
                                    local.set $29
                                    local.get $18
                                    i32.const 0
                                    i32.ne
                                    local.set $35
                                    local.get $18
                                    local.get $22
                                    i32.const 102
                                    i32.ne
                                    if (result i32) ;; label = @17
                                      local.get $5
                                    else
                                      i32.const 0
                                    end
                                    i32.sub
                                    local.get $35
                                    local.get $29
                                    i32.and
                                    i32.const 31
                                    i32.shl
                                    i32.const 31
                                    i32.shr_s
                                    i32.add
                                    local.tee $8
                                    local.get $13
                                    local.get $25
                                    i32.sub
                                    i32.const 2
                                    i32.shr_s
                                    i32.const 9
                                    i32.mul
                                    i32.const -9
                                    i32.add
                                    i32.lt_s
                                    if ;; label = @17
                                      block ;; label = @18
                                        local.get $8
                                        i32.const 9216
                                        i32.add
                                        local.tee $14
                                        i32.const 9
                                        i32.rem_s
                                        i32.const 1
                                        i32.add
                                        local.tee $8
                                        i32.const 9
                                        i32.lt_s
                                        if ;; label = @19
                                          block ;; label = @20
                                            i32.const 10
                                            local.set $6
                                            loop $label$180 ;; label = @21
                                              local.get $6
                                              i32.const 10
                                              i32.mul
                                              local.set $6
                                              local.get $8
                                              i32.const 1
                                              i32.add
                                              local.tee $8
                                              i32.const 9
                                              i32.ne
                                              br_if 0 (;@21;)
                                            end
                                          end
                                        else
                                          i32.const 10
                                          local.set $6
                                        end
                                        local.get $7
                                        i32.const 4
                                        i32.add
                                        local.get $14
                                        i32.const 9
                                        i32.div_s
                                        i32.const -1024
                                        i32.add
                                        i32.const 2
                                        i32.shl
                                        i32.add
                                        local.tee $8
                                        i32.load
                                        local.tee $22
                                        local.get $6
                                        i32.rem_u
                                        local.set $14
                                        block $label$182 ;; label = @19
                                          local.get $8
                                          i32.const 4
                                          i32.add
                                          local.get $13
                                          i32.eq
                                          local.tee $32
                                          local.get $14
                                          i32.eqz
                                          i32.and
                                          i32.eqz
                                          if ;; label = @20
                                            block ;; label = @21
                                              local.get $14
                                              local.get $6
                                              i32.const 2
                                              i32.div_s
                                              local.tee $49
                                              i32.lt_u
                                              if (result f64) ;; label = @22
                                                f64.const 0x1p-1 (;=0.5;)
                                              else
                                                local.get $32
                                                local.get $14
                                                local.get $49
                                                i32.eq
                                                i32.and
                                                if (result f64) ;; label = @23
                                                  f64.const 0x1p+0 (;=1;)
                                                else
                                                  f64.const 0x1.8p+0 (;=1.5;)
                                                end
                                              end
                                              local.set $52
                                              local.get $22
                                              local.get $6
                                              i32.div_u
                                              i32.const 1
                                              i32.and
                                              if (result f64) ;; label = @22
                                                f64.const 0x1.0000000000001p+53 (;=9007199254740994;)
                                              else
                                                f64.const 0x1p+53 (;=9007199254740992;)
                                              end
                                              local.set $53
                                              block $label$190 ;; label = @22
                                                local.get $24
                                                if ;; label = @23
                                                  block ;; label = @24
                                                    local.get $26
                                                    i32.load8_s
                                                    i32.const 45
                                                    i32.ne
                                                    br_if 2 (;@22;)
                                                    local.get $53
                                                    f64.neg
                                                    local.set $53
                                                    local.get $52
                                                    f64.neg
                                                    local.set $52
                                                  end
                                                end
                                              end
                                              local.get $8
                                              local.get $22
                                              local.get $14
                                              i32.sub
                                              local.tee $14
                                              i32.store
                                              local.get $53
                                              local.get $52
                                              f64.add
                                              local.get $53
                                              f64.eq
                                              br_if 2 (;@19;)
                                              local.get $8
                                              local.get $14
                                              local.get $6
                                              i32.add
                                              local.tee $5
                                              i32.store
                                              local.get $5
                                              i32.const 999999999
                                              i32.gt_u
                                              if ;; label = @22
                                                loop $label$193 ;; label = @23
                                                  local.get $8
                                                  i32.const 0
                                                  i32.store
                                                  local.get $8
                                                  i32.const -4
                                                  i32.add
                                                  local.tee $8
                                                  local.get $1
                                                  i32.lt_u
                                                  if ;; label = @24
                                                    local.get $1
                                                    i32.const -4
                                                    i32.add
                                                    local.tee $1
                                                    i32.const 0
                                                    i32.store
                                                  end
                                                  local.get $8
                                                  local.get $8
                                                  i32.load
                                                  i32.const 1
                                                  i32.add
                                                  local.tee $5
                                                  i32.store
                                                  local.get $5
                                                  i32.const 999999999
                                                  i32.gt_u
                                                  br_if 0 (;@23;)
                                                end
                                              end
                                              local.get $25
                                              local.get $1
                                              i32.sub
                                              i32.const 2
                                              i32.shr_s
                                              i32.const 9
                                              i32.mul
                                              local.set $5
                                              local.get $1
                                              i32.load
                                              local.tee $14
                                              i32.const 10
                                              i32.lt_u
                                              br_if 2 (;@19;)
                                              i32.const 10
                                              local.set $6
                                              loop $label$195 ;; label = @22
                                                local.get $5
                                                i32.const 1
                                                i32.add
                                                local.set $5
                                                local.get $14
                                                local.get $6
                                                i32.const 10
                                                i32.mul
                                                local.tee $6
                                                i32.ge_u
                                                br_if 0 (;@22;)
                                              end
                                            end
                                          end
                                        end
                                        local.get $1
                                        local.set $14
                                        local.get $5
                                        local.set $6
                                        local.get $13
                                        local.get $8
                                        i32.const 4
                                        i32.add
                                        local.tee $8
                                        i32.le_u
                                        if ;; label = @19
                                          local.get $13
                                          local.set $8
                                        end
                                      end
                                    else
                                      block ;; label = @18
                                        local.get $1
                                        local.set $14
                                        local.get $5
                                        local.set $6
                                        local.get $13
                                        local.set $8
                                      end
                                    end
                                    i32.const 0
                                    local.get $6
                                    i32.sub
                                    local.set $32
                                    loop $label$198 ;; label = @17
                                      block $label$199 ;; label = @18
                                        local.get $8
                                        local.get $14
                                        i32.le_u
                                        if ;; label = @19
                                          block ;; label = @20
                                            i32.const 0
                                            local.set $22
                                            br 2 (;@18;)
                                          end
                                        end
                                        local.get $8
                                        i32.const -4
                                        i32.add
                                        local.tee $1
                                        i32.load
                                        if ;; label = @19
                                          i32.const 1
                                          local.set $22
                                        else
                                          block ;; label = @20
                                            local.get $1
                                            local.set $8
                                            br 3 (;@17;)
                                          end
                                        end
                                      end
                                    end
                                    block $label$203 ;; label = @17
                                      local.get $29
                                      if ;; label = @18
                                        block ;; label = @19
                                          local.get $35
                                          i32.const 1
                                          i32.and
                                          i32.const 1
                                          i32.xor
                                          local.get $18
                                          i32.add
                                          local.tee $1
                                          local.get $6
                                          i32.gt_s
                                          local.get $6
                                          i32.const -5
                                          i32.gt_s
                                          i32.and
                                          if (result i32) ;; label = @20
                                            block (result i32) ;; label = @21
                                              local.get $9
                                              i32.const -1
                                              i32.add
                                              local.set $5
                                              local.get $1
                                              i32.const -1
                                              i32.add
                                              local.get $6
                                              i32.sub
                                            end
                                          else
                                            block (result i32) ;; label = @21
                                              local.get $9
                                              i32.const -2
                                              i32.add
                                              local.set $5
                                              local.get $1
                                              i32.const -1
                                              i32.add
                                            end
                                          end
                                          local.set $1
                                          local.get $12
                                          i32.const 8
                                          i32.and
                                          local.tee $13
                                          br_if 2 (;@17;)
                                          block $label$207 ;; label = @20
                                            local.get $22
                                            if ;; label = @21
                                              block ;; label = @22
                                                local.get $8
                                                i32.const -4
                                                i32.add
                                                i32.load
                                                local.tee $18
                                                i32.eqz
                                                if ;; label = @23
                                                  block ;; label = @24
                                                    i32.const 9
                                                    local.set $9
                                                    br 4 (;@20;)
                                                  end
                                                end
                                                local.get $18
                                                i32.const 10
                                                i32.rem_u
                                                if ;; label = @23
                                                  block ;; label = @24
                                                    i32.const 0
                                                    local.set $9
                                                    br 4 (;@20;)
                                                  end
                                                else
                                                  block ;; label = @24
                                                    i32.const 10
                                                    local.set $13
                                                    i32.const 0
                                                    local.set $9
                                                  end
                                                end
                                                loop $label$212 ;; label = @23
                                                  local.get $9
                                                  i32.const 1
                                                  i32.add
                                                  local.set $9
                                                  local.get $18
                                                  local.get $13
                                                  i32.const 10
                                                  i32.mul
                                                  local.tee $13
                                                  i32.rem_u
                                                  i32.eqz
                                                  br_if 0 (;@23;)
                                                end
                                              end
                                            else
                                              i32.const 9
                                              local.set $9
                                            end
                                          end
                                          local.get $8
                                          local.get $25
                                          i32.sub
                                          i32.const 2
                                          i32.shr_s
                                          i32.const 9
                                          i32.mul
                                          i32.const -9
                                          i32.add
                                          local.set $18
                                          local.get $5
                                          i32.const 32
                                          i32.or
                                          i32.const 102
                                          i32.eq
                                          if ;; label = @20
                                            block ;; label = @21
                                              i32.const 0
                                              local.set $13
                                              local.get $1
                                              local.get $18
                                              local.get $9
                                              i32.sub
                                              local.tee $9
                                              i32.const 0
                                              i32.lt_s
                                              if (result i32) ;; label = @22
                                                i32.const 0
                                                local.tee $9
                                              else
                                                local.get $9
                                              end
                                              i32.ge_s
                                              if ;; label = @22
                                                local.get $9
                                                local.set $1
                                              end
                                            end
                                          else
                                            block ;; label = @21
                                              i32.const 0
                                              local.set $13
                                              local.get $1
                                              local.get $18
                                              local.get $6
                                              i32.add
                                              local.get $9
                                              i32.sub
                                              local.tee $9
                                              i32.const 0
                                              i32.lt_s
                                              if (result i32) ;; label = @22
                                                i32.const 0
                                                local.tee $9
                                              else
                                                local.get $9
                                              end
                                              i32.ge_s
                                              if ;; label = @22
                                                local.get $9
                                                local.set $1
                                              end
                                            end
                                          end
                                        end
                                      else
                                        block ;; label = @19
                                          local.get $12
                                          i32.const 8
                                          i32.and
                                          local.set $13
                                          local.get $18
                                          local.set $1
                                          local.get $9
                                          local.set $5
                                        end
                                      end
                                    end
                                    local.get $5
                                    i32.const 32
                                    i32.or
                                    i32.const 102
                                    i32.eq
                                    local.tee $25
                                    if ;; label = @17
                                      block ;; label = @18
                                        i32.const 0
                                        local.set $9
                                        local.get $6
                                        i32.const 0
                                        i32.le_s
                                        if ;; label = @19
                                          i32.const 0
                                          local.set $6
                                        end
                                      end
                                    else
                                      block ;; label = @18
                                        local.get $28
                                        local.get $6
                                        i32.const 0
                                        i32.lt_s
                                        if (result i32) ;; label = @19
                                          local.get $32
                                        else
                                          local.get $6
                                        end
                                        i64.extend_i32_s
                                        local.get $33
                                        call $23
                                        local.tee $9
                                        i32.sub
                                        i32.const 2
                                        i32.lt_s
                                        if ;; label = @19
                                          loop $label$229 ;; label = @20
                                            local.get $9
                                            i32.const -1
                                            i32.add
                                            local.tee $9
                                            i32.const 48
                                            i32.store8
                                            local.get $28
                                            local.get $9
                                            i32.sub
                                            i32.const 2
                                            i32.lt_s
                                            br_if 0 (;@20;)
                                          end
                                        end
                                        local.get $9
                                        i32.const -1
                                        i32.add
                                        local.get $6
                                        i32.const 31
                                        i32.shr_s
                                        i32.const 2
                                        i32.and
                                        i32.const 43
                                        i32.add
                                        i32.store8
                                        local.get $9
                                        i32.const -2
                                        i32.add
                                        local.tee $6
                                        local.get $5
                                        i32.store8
                                        local.get $6
                                        local.set $9
                                        local.get $28
                                        local.get $6
                                        i32.sub
                                        local.set $6
                                      end
                                    end
                                    local.get $0
                                    i32.const 32
                                    local.get $10
                                    local.get $24
                                    i32.const 1
                                    i32.add
                                    local.get $1
                                    i32.add
                                    local.get $1
                                    local.get $13
                                    i32.or
                                    local.tee $29
                                    i32.const 0
                                    i32.ne
                                    i32.add
                                    local.get $6
                                    i32.add
                                    local.tee $18
                                    local.get $12
                                    call $25
                                    local.get $0
                                    i32.load
                                    i32.const 32
                                    i32.and
                                    i32.eqz
                                    if ;; label = @17
                                      local.get $26
                                      local.get $24
                                      local.get $0
                                      call $21
                                      drop
                                    end
                                    local.get $0
                                    i32.const 48
                                    local.get $10
                                    local.get $18
                                    local.get $12
                                    i32.const 65536
                                    i32.xor
                                    call $25
                                    block $label$231 ;; label = @17
                                      local.get $25
                                      if ;; label = @18
                                        block ;; label = @19
                                          local.get $14
                                          local.get $7
                                          i32.gt_u
                                          if (result i32) ;; label = @20
                                            local.get $7
                                          else
                                            local.get $14
                                          end
                                          local.tee $9
                                          local.set $6
                                          loop $label$235 ;; label = @20
                                            local.get $6
                                            i32.load
                                            i64.extend_i32_u
                                            local.get $31
                                            call $23
                                            local.set $5
                                            block $label$236 ;; label = @21
                                              local.get $6
                                              local.get $9
                                              i32.eq
                                              if ;; label = @22
                                                block ;; label = @23
                                                  local.get $5
                                                  local.get $31
                                                  i32.ne
                                                  br_if 2 (;@21;)
                                                  local.get $34
                                                  i32.const 48
                                                  i32.store8
                                                  local.get $34
                                                  local.set $5
                                                end
                                              else
                                                block ;; label = @23
                                                  local.get $5
                                                  local.get $19
                                                  i32.le_u
                                                  br_if 2 (;@21;)
                                                  local.get $19
                                                  i32.const 48
                                                  local.get $5
                                                  local.get $27
                                                  i32.sub
                                                  call $46
                                                  drop
                                                  loop $label$239 ;; label = @24
                                                    local.get $5
                                                    i32.const -1
                                                    i32.add
                                                    local.tee $5
                                                    local.get $19
                                                    i32.gt_u
                                                    br_if 0 (;@24;)
                                                  end
                                                end
                                              end
                                            end
                                            local.get $0
                                            i32.load
                                            i32.const 32
                                            i32.and
                                            i32.eqz
                                            if ;; label = @21
                                              local.get $5
                                              local.get $41
                                              local.get $5
                                              i32.sub
                                              local.get $0
                                              call $21
                                              drop
                                            end
                                            local.get $6
                                            i32.const 4
                                            i32.add
                                            local.tee $5
                                            local.get $7
                                            i32.le_u
                                            if ;; label = @21
                                              block ;; label = @22
                                                local.get $5
                                                local.set $6
                                                br 2 (;@20;)
                                              end
                                            end
                                          end
                                          block $label$242 ;; label = @20
                                            local.get $29
                                            if ;; label = @21
                                              block ;; label = @22
                                                local.get $0
                                                i32.load
                                                i32.const 32
                                                i32.and
                                                br_if 2 (;@20;)
                                                i32.const 2231
                                                i32.const 1
                                                local.get $0
                                                call $21
                                                drop
                                              end
                                            end
                                          end
                                          local.get $1
                                          i32.const 0
                                          i32.gt_s
                                          local.get $5
                                          local.get $8
                                          i32.lt_u
                                          i32.and
                                          if ;; label = @20
                                            loop $label$245 ;; label = @21
                                              local.get $5
                                              i32.load
                                              i64.extend_i32_u
                                              local.get $31
                                              call $23
                                              local.tee $7
                                              local.get $19
                                              i32.gt_u
                                              if ;; label = @22
                                                block ;; label = @23
                                                  local.get $19
                                                  i32.const 48
                                                  local.get $7
                                                  local.get $27
                                                  i32.sub
                                                  call $46
                                                  drop
                                                  loop $label$247 ;; label = @24
                                                    local.get $7
                                                    i32.const -1
                                                    i32.add
                                                    local.tee $7
                                                    local.get $19
                                                    i32.gt_u
                                                    br_if 0 (;@24;)
                                                  end
                                                end
                                              end
                                              local.get $0
                                              i32.load
                                              i32.const 32
                                              i32.and
                                              i32.eqz
                                              if ;; label = @22
                                                local.get $7
                                                local.get $1
                                                i32.const 9
                                                i32.gt_s
                                                if (result i32) ;; label = @23
                                                  i32.const 9
                                                else
                                                  local.get $1
                                                end
                                                local.get $0
                                                call $21
                                                drop
                                              end
                                              local.get $1
                                              i32.const -9
                                              i32.add
                                              local.set $7
                                              local.get $1
                                              i32.const 9
                                              i32.gt_s
                                              local.get $5
                                              i32.const 4
                                              i32.add
                                              local.tee $5
                                              local.get $8
                                              i32.lt_u
                                              i32.and
                                              if ;; label = @22
                                                block ;; label = @23
                                                  local.get $7
                                                  local.set $1
                                                  br 2 (;@21;)
                                                end
                                              else
                                                local.get $7
                                                local.set $1
                                              end
                                            end
                                          end
                                          local.get $0
                                          i32.const 48
                                          local.get $1
                                          i32.const 9
                                          i32.add
                                          i32.const 9
                                          i32.const 0
                                          call $25
                                        end
                                      else
                                        block ;; label = @19
                                          local.get $14
                                          i32.const 4
                                          i32.add
                                          local.set $5
                                          local.get $22
                                          i32.eqz
                                          if ;; label = @20
                                            local.get $5
                                            local.set $8
                                          end
                                          local.get $1
                                          i32.const -1
                                          i32.gt_s
                                          if ;; label = @20
                                            block ;; label = @21
                                              local.get $13
                                              i32.eqz
                                              local.set $13
                                              local.get $14
                                              local.set $7
                                              local.get $1
                                              local.set $5
                                              loop $label$256 ;; label = @22
                                                local.get $7
                                                i32.load
                                                i64.extend_i32_u
                                                local.get $31
                                                call $23
                                                local.tee $1
                                                local.get $31
                                                i32.eq
                                                if ;; label = @23
                                                  block ;; label = @24
                                                    local.get $34
                                                    i32.const 48
                                                    i32.store8
                                                    local.get $34
                                                    local.set $1
                                                  end
                                                end
                                                block $label$258 ;; label = @23
                                                  local.get $7
                                                  local.get $14
                                                  i32.eq
                                                  if ;; label = @24
                                                    block ;; label = @25
                                                      local.get $0
                                                      i32.load
                                                      i32.const 32
                                                      i32.and
                                                      i32.eqz
                                                      if ;; label = @26
                                                        local.get $1
                                                        i32.const 1
                                                        local.get $0
                                                        call $21
                                                        drop
                                                      end
                                                      local.get $1
                                                      i32.const 1
                                                      i32.add
                                                      local.set $1
                                                      local.get $13
                                                      local.get $5
                                                      i32.const 1
                                                      i32.lt_s
                                                      i32.and
                                                      br_if 2 (;@23;)
                                                      local.get $0
                                                      i32.load
                                                      i32.const 32
                                                      i32.and
                                                      br_if 2 (;@23;)
                                                      i32.const 2231
                                                      i32.const 1
                                                      local.get $0
                                                      call $21
                                                      drop
                                                    end
                                                  else
                                                    block ;; label = @25
                                                      local.get $1
                                                      local.get $19
                                                      i32.le_u
                                                      br_if 2 (;@23;)
                                                      local.get $19
                                                      i32.const 48
                                                      local.get $1
                                                      local.get $43
                                                      i32.add
                                                      call $46
                                                      drop
                                                      loop $label$262 ;; label = @26
                                                        local.get $1
                                                        i32.const -1
                                                        i32.add
                                                        local.tee $1
                                                        local.get $19
                                                        i32.gt_u
                                                        br_if 0 (;@26;)
                                                      end
                                                    end
                                                  end
                                                end
                                                local.get $41
                                                local.get $1
                                                i32.sub
                                                local.set $6
                                                local.get $0
                                                i32.load
                                                i32.const 32
                                                i32.and
                                                i32.eqz
                                                if ;; label = @23
                                                  local.get $1
                                                  local.get $5
                                                  local.get $6
                                                  i32.gt_s
                                                  if (result i32) ;; label = @24
                                                    local.get $6
                                                  else
                                                    local.get $5
                                                  end
                                                  local.get $0
                                                  call $21
                                                  drop
                                                end
                                                local.get $7
                                                i32.const 4
                                                i32.add
                                                local.tee $7
                                                local.get $8
                                                i32.lt_u
                                                local.get $5
                                                local.get $6
                                                i32.sub
                                                local.tee $5
                                                i32.const -1
                                                i32.gt_s
                                                i32.and
                                                br_if 0 (;@22;)
                                                local.get $5
                                                local.set $1
                                              end
                                            end
                                          end
                                          local.get $0
                                          i32.const 48
                                          local.get $1
                                          i32.const 18
                                          i32.add
                                          i32.const 18
                                          i32.const 0
                                          call $25
                                          local.get $0
                                          i32.load
                                          i32.const 32
                                          i32.and
                                          br_if 2 (;@17;)
                                          local.get $9
                                          local.get $28
                                          local.get $9
                                          i32.sub
                                          local.get $0
                                          call $21
                                          drop
                                        end
                                      end
                                    end
                                    local.get $0
                                    i32.const 32
                                    local.get $10
                                    local.get $18
                                    local.get $12
                                    i32.const 8192
                                    i32.xor
                                    call $25
                                    local.get $18
                                    local.get $10
                                    i32.ge_s
                                    if ;; label = @17
                                      local.get $18
                                      local.set $10
                                    end
                                  end
                                else
                                  block ;; label = @16
                                    local.get $0
                                    i32.const 32
                                    local.get $10
                                    local.get $52
                                    local.get $52
                                    f64.ne
                                    i32.const 0
                                    i32.or
                                    local.tee $6
                                    if (result i32) ;; label = @17
                                      i32.const 0
                                      local.tee $24
                                    else
                                      local.get $24
                                    end
                                    i32.const 3
                                    i32.add
                                    local.tee $8
                                    local.get $7
                                    call $25
                                    local.get $0
                                    i32.load
                                    local.tee $1
                                    i32.const 32
                                    i32.and
                                    i32.eqz
                                    if ;; label = @17
                                      block ;; label = @18
                                        local.get $26
                                        local.get $24
                                        local.get $0
                                        call $21
                                        drop
                                        local.get $0
                                        i32.load
                                        local.set $1
                                      end
                                    end
                                    local.get $9
                                    i32.const 32
                                    i32.and
                                    i32.const 0
                                    i32.ne
                                    local.tee $5
                                    if (result i32) ;; label = @17
                                      i32.const 2215
                                    else
                                      i32.const 2219
                                    end
                                    local.set $7
                                    local.get $5
                                    if (result i32) ;; label = @17
                                      i32.const 2223
                                    else
                                      i32.const 2227
                                    end
                                    local.set $5
                                    local.get $6
                                    i32.eqz
                                    if ;; label = @17
                                      local.get $7
                                      local.set $5
                                    end
                                    local.get $1
                                    i32.const 32
                                    i32.and
                                    i32.eqz
                                    if ;; label = @17
                                      local.get $5
                                      i32.const 3
                                      local.get $0
                                      call $21
                                      drop
                                    end
                                    local.get $0
                                    i32.const 32
                                    local.get $10
                                    local.get $8
                                    local.get $12
                                    i32.const 8192
                                    i32.xor
                                    call $25
                                    local.get $8
                                    local.get $10
                                    i32.ge_s
                                    if ;; label = @17
                                      local.get $8
                                      local.set $10
                                    end
                                  end
                                end
                              end
                              local.get $11
                              local.set $1
                              br 9 (;@4;)
                            end
                            local.get $5
                            local.set $7
                            i32.const 0
                            local.set $6
                            i32.const 2179
                            local.set $8
                            local.get $21
                            local.set $5
                            br 6 (;@6;)
                          end
                          local.get $9
                          i32.const 32
                          i32.and
                          local.set $7
                          local.get $16
                          i64.load
                          local.tee $50
                          i64.const 0
                          i64.eq
                          if (result i32) ;; label = @12
                            block (result i32) ;; label = @13
                              i64.const 0
                              local.set $50
                              local.get $21
                            end
                          else
                            block (result i32) ;; label = @13
                              local.get $21
                              local.set $1
                              loop $label$280 ;; label = @14
                                local.get $1
                                i32.const -1
                                i32.add
                                local.tee $1
                                local.get $50
                                i32.wrap_i64
                                i32.const 15
                                i32.and
                                i32.const 2163
                                i32.add
                                i32.load8_u
                                local.get $7
                                i32.or
                                i32.store8
                                local.get $50
                                i64.const 4
                                i64.shr_u
                                local.tee $50
                                i64.const 0
                                i64.ne
                                br_if 0 (;@14;)
                              end
                              local.get $16
                              i64.load
                              local.set $50
                              local.get $1
                            end
                          end
                          local.set $7
                          local.get $9
                          i32.const 4
                          i32.shr_s
                          i32.const 2179
                          i32.add
                          local.set $8
                          local.get $12
                          i32.const 8
                          i32.and
                          i32.eqz
                          local.get $50
                          i64.const 0
                          i64.eq
                          i32.or
                          local.tee $1
                          if ;; label = @12
                            i32.const 2179
                            local.set $8
                          end
                          local.get $1
                          if (result i32) ;; label = @12
                            i32.const 0
                          else
                            i32.const 2
                          end
                          local.set $6
                          br 4 (;@7;)
                        end
                        local.get $50
                        local.get $21
                        call $23
                        local.set $7
                        br 3 (;@7;)
                      end
                      local.get $1
                      i32.const 0
                      local.get $5
                      call $17
                      local.tee $13
                      i32.eqz
                      local.set $14
                      local.get $13
                      local.get $1
                      i32.sub
                      local.set $8
                      local.get $1
                      local.get $5
                      i32.add
                      local.set $9
                      local.get $7
                      local.set $12
                      local.get $14
                      if (result i32) ;; label = @10
                        local.get $5
                      else
                        local.get $8
                      end
                      local.set $7
                      i32.const 0
                      local.set $6
                      i32.const 2179
                      local.set $8
                      local.get $14
                      if (result i32) ;; label = @10
                        local.get $9
                      else
                        local.get $13
                      end
                      local.set $5
                      br 3 (;@6;)
                    end
                    i32.const 0
                    local.set $1
                    i32.const 0
                    local.set $5
                    local.get $7
                    local.set $8
                    loop $label$288 ;; label = @9
                      block $label$289 ;; label = @10
                        local.get $8
                        i32.load
                        local.tee $9
                        i32.eqz
                        br_if 0 (;@10;)
                        local.get $36
                        local.get $9
                        call $26
                        local.tee $5
                        i32.const 0
                        i32.lt_s
                        local.get $5
                        local.get $6
                        local.get $1
                        i32.sub
                        i32.gt_u
                        i32.or
                        br_if 0 (;@10;)
                        local.get $8
                        i32.const 4
                        i32.add
                        local.set $8
                        local.get $6
                        local.get $5
                        local.get $1
                        i32.add
                        local.tee $1
                        i32.gt_u
                        br_if 1 (;@9;)
                      end
                    end
                    local.get $5
                    i32.const 0
                    i32.lt_s
                    if ;; label = @9
                      block ;; label = @10
                        i32.const -1
                        local.set $15
                        br 5 (;@5;)
                      end
                    end
                    local.get $0
                    i32.const 32
                    local.get $10
                    local.get $1
                    local.get $12
                    call $25
                    local.get $1
                    if ;; label = @9
                      block ;; label = @10
                        i32.const 0
                        local.set $5
                        loop $label$292 ;; label = @11
                          local.get $7
                          i32.load
                          local.tee $8
                          i32.eqz
                          br_if 3 (;@8;)
                          local.get $36
                          local.get $8
                          call $26
                          local.tee $8
                          local.get $5
                          i32.add
                          local.tee $5
                          local.get $1
                          i32.gt_s
                          br_if 3 (;@8;)
                          local.get $0
                          i32.load
                          i32.const 32
                          i32.and
                          i32.eqz
                          if ;; label = @12
                            local.get $36
                            local.get $8
                            local.get $0
                            call $21
                            drop
                          end
                          local.get $7
                          i32.const 4
                          i32.add
                          local.set $7
                          local.get $5
                          local.get $1
                          i32.lt_u
                          br_if 0 (;@11;)
                          br 3 (;@8;)
                        end
                      end
                    else
                      block ;; label = @10
                        i32.const 0
                        local.set $1
                        br 2 (;@8;)
                      end
                    end
                  end
                  local.get $0
                  i32.const 32
                  local.get $10
                  local.get $1
                  local.get $12
                  i32.const 8192
                  i32.xor
                  call $25
                  local.get $10
                  local.get $1
                  i32.le_s
                  if ;; label = @8
                    local.get $1
                    local.set $10
                  end
                  local.get $11
                  local.set $1
                  br 3 (;@4;)
                end
                local.get $12
                i32.const -65537
                i32.and
                local.set $1
                local.get $5
                i32.const -1
                i32.gt_s
                if ;; label = @7
                  local.get $1
                  local.set $12
                end
                local.get $5
                local.get $16
                i64.load
                i64.const 0
                i64.ne
                local.tee $9
                i32.or
                if (result i32) ;; label = @7
                  block (result i32) ;; label = @8
                    local.get $7
                    local.set $1
                    local.get $5
                    local.get $9
                    i32.const 1
                    i32.and
                    i32.const 1
                    i32.xor
                    local.get $38
                    local.get $7
                    i32.sub
                    i32.add
                    local.tee $7
                    i32.gt_s
                    if ;; label = @9
                      local.get $5
                      local.set $7
                    end
                    local.get $21
                  end
                else
                  block (result i32) ;; label = @8
                    local.get $21
                    local.set $1
                    i32.const 0
                    local.set $7
                    local.get $21
                  end
                end
                local.set $5
              end
              local.get $0
              i32.const 32
              local.get $10
              local.get $7
              local.get $5
              local.get $1
              i32.sub
              local.tee $9
              i32.lt_s
              if (result i32) ;; label = @6
                local.get $9
                local.tee $7
              else
                local.get $7
              end
              local.get $6
              i32.add
              local.tee $5
              i32.lt_s
              if (result i32) ;; label = @6
                local.get $5
                local.tee $10
              else
                local.get $10
              end
              local.get $5
              local.get $12
              call $25
              local.get $0
              i32.load
              i32.const 32
              i32.and
              i32.eqz
              if ;; label = @6
                local.get $8
                local.get $6
                local.get $0
                call $21
                drop
              end
              local.get $0
              i32.const 48
              local.get $10
              local.get $5
              local.get $12
              i32.const 65536
              i32.xor
              call $25
              local.get $0
              i32.const 48
              local.get $7
              local.get $9
              i32.const 0
              call $25
              local.get $0
              i32.load
              i32.const 32
              i32.and
              i32.eqz
              if ;; label = @6
                local.get $1
                local.get $9
                local.get $0
                call $21
                drop
              end
              local.get $0
              i32.const 32
              local.get $10
              local.get $5
              local.get $12
              i32.const 8192
              i32.xor
              call $25
              local.get $11
              local.set $1
              br 1 (;@4;)
            end
          end
          br 1 (;@2;)
        end
        local.get $0
        i32.eqz
        if ;; label = @3
          local.get $17
          if ;; label = @4
            block ;; label = @5
              i32.const 1
              local.set $0
              loop $label$308 ;; label = @6
                local.get $4
                local.get $0
                i32.const 2
                i32.shl
                i32.add
                i32.load
                local.tee $1
                if ;; label = @7
                  block ;; label = @8
                    local.get $3
                    local.get $0
                    i32.const 3
                    i32.shl
                    i32.add
                    local.get $1
                    local.get $2
                    call $22
                    local.get $0
                    i32.const 1
                    i32.add
                    local.tee $0
                    i32.const 10
                    i32.lt_s
                    br_if 2 (;@6;)
                    i32.const 1
                    local.set $15
                    br 6 (;@2;)
                  end
                end
              end
              loop $label$310 ;; label = @6
                local.get $4
                local.get $0
                i32.const 2
                i32.shl
                i32.add
                i32.load
                if ;; label = @7
                  block ;; label = @8
                    i32.const -1
                    local.set $15
                    br 6 (;@2;)
                  end
                end
                local.get $0
                i32.const 1
                i32.add
                local.tee $0
                i32.const 10
                i32.lt_s
                br_if 0 (;@6;)
                i32.const 1
                local.set $15
              end
            end
          else
            i32.const 0
            local.set $15
          end
        end
      end
      local.get $23
      global.set $global$1
      local.get $15
    end
  )
  (func $20 (;33;) (type $2) (param $0 i32) (result i32)
    i32.const 0
  )
  (func $21 (;34;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    (local $3 i32) (local $4 i32) (local $5 i32) (local $6 i32)
    block $label$1 (result i32) ;; label = @1
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          local.get $2
          i32.const 16
          i32.add
          local.tee $4
          i32.load
          local.tee $3
          br_if 0 (;@3;)
          local.get $2
          call $30
          if ;; label = @4
            i32.const 0
            local.set $3
          else
            block ;; label = @5
              local.get $4
              i32.load
              local.set $3
              br 2 (;@3;)
            end
          end
          br 1 (;@2;)
        end
        local.get $3
        local.get $2
        i32.const 20
        i32.add
        local.tee $5
        i32.load
        local.tee $4
        i32.sub
        local.get $1
        i32.lt_u
        if ;; label = @3
          block ;; label = @4
            local.get $2
            local.get $0
            local.get $1
            local.get $2
            i32.load offset=36
            i32.const 3
            i32.and
            i32.const 2
            i32.add
            call_indirect (type $0)
            local.set $3
            br 2 (;@2;)
          end
        end
        block $label$7 (result i32) ;; label = @3
          local.get $2
          i32.load8_s offset=75
          i32.const -1
          i32.gt_s
          if (result i32) ;; label = @4
            block (result i32) ;; label = @5
              local.get $1
              local.set $3
              loop $label$9 ;; label = @6
                i32.const 0
                local.get $3
                i32.eqz
                br_if 3 (;@3;)
                drop
                local.get $0
                local.get $3
                i32.const -1
                i32.add
                local.tee $6
                i32.add
                i32.load8_s
                i32.const 10
                i32.ne
                if ;; label = @7
                  block ;; label = @8
                    local.get $6
                    local.set $3
                    br 2 (;@6;)
                  end
                end
              end
              local.get $2
              local.get $0
              local.get $3
              local.get $2
              i32.load offset=36
              i32.const 3
              i32.and
              i32.const 2
              i32.add
              call_indirect (type $0)
              local.get $3
              i32.lt_u
              br_if 3 (;@2;)
              local.get $5
              i32.load
              local.set $4
              local.get $1
              local.get $3
              i32.sub
              local.set $1
              local.get $0
              local.get $3
              i32.add
              local.set $0
              local.get $3
            end
          else
            i32.const 0
          end
        end
        local.set $2
        local.get $4
        local.get $0
        local.get $1
        call $47
        drop
        local.get $5
        local.get $5
        i32.load
        local.get $1
        i32.add
        i32.store
        local.get $2
        local.get $1
        i32.add
        local.set $3
      end
      local.get $3
    end
  )
  (func $22 (;35;) (type $8) (param $0 i32) (param $1 i32) (param $2 i32)
    (local $3 i32) (local $4 i64) (local $5 f64)
    block $label$1 ;; label = @1
      local.get $1
      i32.const 20
      i32.le_u
      if ;; label = @2
        block $label$3 ;; label = @3
          block $label$4 ;; label = @4
            block $label$5 ;; label = @5
              block $label$6 ;; label = @6
                block $label$7 ;; label = @7
                  block $label$8 ;; label = @8
                    block $label$9 ;; label = @9
                      block $label$10 ;; label = @10
                        block $label$11 ;; label = @11
                          block $label$12 ;; label = @12
                            block $label$13 ;; label = @13
                              local.get $1
                              i32.const 9
                              i32.sub
                              br_table 0 (;@13;) 1 (;@12;) 2 (;@11;) 3 (;@10;) 4 (;@9;) 5 (;@8;) 6 (;@7;) 7 (;@6;) 8 (;@5;) 9 (;@4;) 10 (;@3;)
                            end
                            local.get $2
                            i32.load
                            i32.const 3
                            i32.add
                            i32.const -4
                            i32.and
                            local.tee $1
                            i32.load
                            local.set $3
                            local.get $2
                            local.get $1
                            i32.const 4
                            i32.add
                            i32.store
                            local.get $0
                            local.get $3
                            i32.store
                            br 11 (;@1;)
                          end
                          local.get $2
                          i32.load
                          i32.const 3
                          i32.add
                          i32.const -4
                          i32.and
                          local.tee $1
                          i32.load
                          local.set $3
                          local.get $2
                          local.get $1
                          i32.const 4
                          i32.add
                          i32.store
                          local.get $0
                          local.get $3
                          i64.extend_i32_s
                          i64.store
                          br 10 (;@1;)
                        end
                        local.get $2
                        i32.load
                        i32.const 3
                        i32.add
                        i32.const -4
                        i32.and
                        local.tee $1
                        i32.load
                        local.set $3
                        local.get $2
                        local.get $1
                        i32.const 4
                        i32.add
                        i32.store
                        local.get $0
                        local.get $3
                        i64.extend_i32_u
                        i64.store
                        br 9 (;@1;)
                      end
                      local.get $2
                      i32.load
                      i32.const 7
                      i32.add
                      i32.const -8
                      i32.and
                      local.tee $1
                      i64.load
                      local.set $4
                      local.get $2
                      local.get $1
                      i32.const 8
                      i32.add
                      i32.store
                      local.get $0
                      local.get $4
                      i64.store
                      br 8 (;@1;)
                    end
                    local.get $2
                    i32.load
                    i32.const 3
                    i32.add
                    i32.const -4
                    i32.and
                    local.tee $1
                    i32.load
                    local.set $3
                    local.get $2
                    local.get $1
                    i32.const 4
                    i32.add
                    i32.store
                    local.get $0
                    local.get $3
                    i32.const 65535
                    i32.and
                    i32.const 16
                    i32.shl
                    i32.const 16
                    i32.shr_s
                    i64.extend_i32_s
                    i64.store
                    br 7 (;@1;)
                  end
                  local.get $2
                  i32.load
                  i32.const 3
                  i32.add
                  i32.const -4
                  i32.and
                  local.tee $1
                  i32.load
                  local.set $3
                  local.get $2
                  local.get $1
                  i32.const 4
                  i32.add
                  i32.store
                  local.get $0
                  local.get $3
                  i32.const 65535
                  i32.and
                  i64.extend_i32_u
                  i64.store
                  br 6 (;@1;)
                end
                local.get $2
                i32.load
                i32.const 3
                i32.add
                i32.const -4
                i32.and
                local.tee $1
                i32.load
                local.set $3
                local.get $2
                local.get $1
                i32.const 4
                i32.add
                i32.store
                local.get $0
                local.get $3
                i32.const 255
                i32.and
                i32.const 24
                i32.shl
                i32.const 24
                i32.shr_s
                i64.extend_i32_s
                i64.store
                br 5 (;@1;)
              end
              local.get $2
              i32.load
              i32.const 3
              i32.add
              i32.const -4
              i32.and
              local.tee $1
              i32.load
              local.set $3
              local.get $2
              local.get $1
              i32.const 4
              i32.add
              i32.store
              local.get $0
              local.get $3
              i32.const 255
              i32.and
              i64.extend_i32_u
              i64.store
              br 4 (;@1;)
            end
            local.get $2
            i32.load
            i32.const 7
            i32.add
            i32.const -8
            i32.and
            local.tee $1
            f64.load
            local.set $5
            local.get $2
            local.get $1
            i32.const 8
            i32.add
            i32.store
            local.get $0
            local.get $5
            f64.store
            br 3 (;@1;)
          end
          local.get $2
          i32.load
          i32.const 7
          i32.add
          i32.const -8
          i32.and
          local.tee $1
          f64.load
          local.set $5
          local.get $2
          local.get $1
          i32.const 8
          i32.add
          i32.store
          local.get $0
          local.get $5
          f64.store
        end
      end
    end
  )
  (func $23 (;36;) (type $9) (param $0 i64) (param $1 i32) (result i32)
    (local $2 i32) (local $3 i32) (local $4 i64)
    block $label$1 (result i32) ;; label = @1
      local.get $0
      i32.wrap_i64
      local.set $2
      local.get $0
      i64.const 4294967295
      i64.gt_u
      if ;; label = @2
        block ;; label = @3
          loop $label$3 ;; label = @4
            local.get $1
            i32.const -1
            i32.add
            local.tee $1
            local.get $0
            i64.const 10
            i64.rem_u
            i64.const 48
            i64.or
            i64.store8
            local.get $0
            i64.const 10
            i64.div_u
            local.set $4
            local.get $0
            i64.const 42949672959
            i64.gt_u
            if ;; label = @5
              block ;; label = @6
                local.get $4
                local.set $0
                br 2 (;@4;)
              end
            end
          end
          local.get $4
          i32.wrap_i64
          local.set $2
        end
      end
      local.get $2
      if ;; label = @2
        loop $label$6 ;; label = @3
          local.get $1
          i32.const -1
          i32.add
          local.tee $1
          local.get $2
          i32.const 10
          i32.rem_u
          i32.const 48
          i32.or
          i32.store8
          local.get $2
          i32.const 10
          i32.div_u
          local.set $3
          local.get $2
          i32.const 10
          i32.ge_u
          if ;; label = @4
            block ;; label = @5
              local.get $3
              local.set $2
              br 2 (;@3;)
            end
          end
        end
      end
      local.get $1
    end
  )
  (func $24 (;37;) (type $2) (param $0 i32) (result i32)
    (local $1 i32) (local $2 i32)
    block $label$1 (result i32) ;; label = @1
      i32.const 0
      local.set $1
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          block $label$4 ;; label = @4
            loop $label$5 ;; label = @5
              local.get $1
              i32.const 2233
              i32.add
              i32.load8_u
              local.get $0
              i32.eq
              br_if 1 (;@4;)
              local.get $1
              i32.const 1
              i32.add
              local.tee $1
              i32.const 87
              i32.ne
              br_if 0 (;@5;)
              i32.const 87
              local.set $1
              i32.const 2321
              local.set $0
              br 2 (;@3;)
            end
          end
          local.get $1
          if ;; label = @4
            block ;; label = @5
              i32.const 2321
              local.set $0
              br 2 (;@3;)
            end
          else
            i32.const 2321
            local.set $0
          end
          br 1 (;@2;)
        end
        loop $label$8 ;; label = @3
          local.get $0
          local.set $2
          loop $label$9 ;; label = @4
            local.get $2
            i32.const 1
            i32.add
            local.set $0
            local.get $2
            i32.load8_s
            if ;; label = @5
              block ;; label = @6
                local.get $0
                local.set $2
                br 2 (;@4;)
              end
            end
          end
          local.get $1
          i32.const -1
          i32.add
          local.tee $1
          br_if 0 (;@3;)
        end
      end
      local.get $0
    end
  )
  (func $25 (;38;) (type $10) (param $0 i32) (param $1 i32) (param $2 i32) (param $3 i32) (param $4 i32)
    (local $5 i32) (local $6 i32) (local $7 i32)
    block $label$1 ;; label = @1
      global.get $global$1
      local.set $7
      global.get $global$1
      i32.const 256
      i32.add
      global.set $global$1
      local.get $7
      local.set $6
      block $label$2 ;; label = @2
        local.get $2
        local.get $3
        i32.gt_s
        local.get $4
        i32.const 73728
        i32.and
        i32.eqz
        i32.and
        if ;; label = @3
          block ;; label = @4
            local.get $6
            local.get $1
            local.get $2
            local.get $3
            i32.sub
            local.tee $5
            i32.const 256
            i32.gt_u
            if (result i32) ;; label = @5
              i32.const 256
            else
              local.get $5
            end
            call $46
            drop
            local.get $0
            i32.load
            local.tee $1
            i32.const 32
            i32.and
            i32.eqz
            local.set $4
            local.get $5
            i32.const 255
            i32.gt_u
            if ;; label = @5
              block ;; label = @6
                loop $label$7 ;; label = @7
                  local.get $4
                  if ;; label = @8
                    block ;; label = @9
                      local.get $6
                      i32.const 256
                      local.get $0
                      call $21
                      drop
                      local.get $0
                      i32.load
                      local.set $1
                    end
                  end
                  local.get $1
                  i32.const 32
                  i32.and
                  i32.eqz
                  local.set $4
                  local.get $5
                  i32.const -256
                  i32.add
                  local.tee $5
                  i32.const 255
                  i32.gt_u
                  br_if 0 (;@7;)
                end
                local.get $4
                i32.eqz
                br_if 4 (;@2;)
                local.get $2
                local.get $3
                i32.sub
                i32.const 255
                i32.and
                local.set $5
              end
            else
              local.get $4
              i32.eqz
              br_if 3 (;@2;)
            end
            local.get $6
            local.get $5
            local.get $0
            call $21
            drop
          end
        end
      end
      local.get $7
      global.set $global$1
    end
  )
  (func $26 (;39;) (type $6) (param $0 i32) (param $1 i32) (result i32)
    local.get $0
    if (result i32) ;; label = @1
      local.get $0
      local.get $1
      i32.const 0
      call $29
    else
      i32.const 0
    end
  )
  (func $27 (;40;) (type $11) (param $0 f64) (param $1 i32) (result f64)
    local.get $0
    local.get $1
    call $28
  )
  (func $28 (;41;) (type $11) (param $0 f64) (param $1 i32) (result f64)
    (local $2 i64) (local $3 i64)
    block $label$1 (result f64) ;; label = @1
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          block $label$4 ;; label = @4
            block $label$5 ;; label = @5
              local.get $0
              i64.reinterpret_f64
              local.tee $2
              i64.const 52
              i64.shr_u
              local.tee $3
              i32.wrap_i64
              i32.const 65535
              i32.and
              i32.const 2047
              i32.and
              i32.const 16
              i32.shl
              i32.const 16
              i32.shr_s
              i32.const 0
              i32.sub
              br_table 0 (;@5;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 2 (;@3;) 1 (;@4;) 2 (;@3;)
            end
            local.get $1
            local.get $0
            f64.const 0x0p+0 (;=0;)
            f64.ne
            if (result i32) ;; label = @5
              block (result i32) ;; label = @6
                local.get $0
                f64.const 0x1p+64 (;=18446744073709552000;)
                f64.mul
                local.get $1
                call $28
                local.set $0
                local.get $1
                i32.load
                i32.const -64
                i32.add
              end
            else
              i32.const 0
            end
            i32.store
            br 2 (;@2;)
          end
          br 1 (;@2;)
        end
        local.get $1
        local.get $3
        i32.wrap_i64
        i32.const 2047
        i32.and
        i32.const -1022
        i32.add
        i32.store
        local.get $2
        i64.const -9218868437227405313
        i64.and
        i64.const 4602678819172646912
        i64.or
        f64.reinterpret_i64
        local.set $0
      end
      local.get $0
    end
  )
  (func $29 (;42;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    block $label$1 (result i32) ;; label = @1
      local.get $0
      if (result i32) ;; label = @2
        block (result i32) ;; label = @3
          local.get $1
          i32.const 128
          i32.lt_u
          if ;; label = @4
            block ;; label = @5
              local.get $0
              local.get $1
              i32.store8
              i32.const 1
              br 4 (;@1;)
            end
          end
          local.get $1
          i32.const 2048
          i32.lt_u
          if ;; label = @4
            block ;; label = @5
              local.get $0
              local.get $1
              i32.const 6
              i32.shr_u
              i32.const 192
              i32.or
              i32.store8
              local.get $0
              local.get $1
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8 offset=1
              i32.const 2
              br 4 (;@1;)
            end
          end
          local.get $1
          i32.const 55296
          i32.lt_u
          local.get $1
          i32.const -8192
          i32.and
          i32.const 57344
          i32.eq
          i32.or
          if ;; label = @4
            block ;; label = @5
              local.get $0
              local.get $1
              i32.const 12
              i32.shr_u
              i32.const 224
              i32.or
              i32.store8
              local.get $0
              local.get $1
              i32.const 6
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8 offset=1
              local.get $0
              local.get $1
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8 offset=2
              i32.const 3
              br 4 (;@1;)
            end
          end
          local.get $1
          i32.const -65536
          i32.add
          i32.const 1048576
          i32.lt_u
          if (result i32) ;; label = @4
            block (result i32) ;; label = @5
              local.get $0
              local.get $1
              i32.const 18
              i32.shr_u
              i32.const 240
              i32.or
              i32.store8
              local.get $0
              local.get $1
              i32.const 12
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8 offset=1
              local.get $0
              local.get $1
              i32.const 6
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8 offset=2
              local.get $0
              local.get $1
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8 offset=3
              i32.const 4
            end
          else
            block (result i32) ;; label = @5
              call $12
              i32.const 84
              i32.store
              i32.const -1
            end
          end
        end
      else
        i32.const 1
      end
    end
  )
  (func $30 (;43;) (type $2) (param $0 i32) (result i32)
    (local $1 i32) (local $2 i32)
    block $label$1 (result i32) ;; label = @1
      local.get $0
      i32.const 74
      i32.add
      local.tee $2
      i32.load8_s
      local.set $1
      local.get $2
      local.get $1
      i32.const 255
      i32.add
      local.get $1
      i32.or
      i32.store8
      local.get $0
      i32.load
      local.tee $1
      i32.const 8
      i32.and
      if (result i32) ;; label = @2
        block (result i32) ;; label = @3
          local.get $0
          local.get $1
          i32.const 32
          i32.or
          i32.store
          i32.const -1
        end
      else
        block (result i32) ;; label = @3
          local.get $0
          i32.const 0
          i32.store offset=8
          local.get $0
          i32.const 0
          i32.store offset=4
          local.get $0
          local.get $0
          i32.load offset=44
          local.tee $1
          i32.store offset=28
          local.get $0
          local.get $1
          i32.store offset=20
          local.get $0
          local.get $1
          local.get $0
          i32.load offset=48
          i32.add
          i32.store offset=16
          i32.const 0
        end
      end
      local.tee $0
    end
  )
  (func $31 (;44;) (type $2) (param $0 i32) (result i32)
    (local $1 i32) (local $2 i32) (local $3 i32)
    block $label$1 (result i32) ;; label = @1
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          local.get $0
          local.tee $2
          i32.const 3
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get $2
          local.set $1
          loop $label$4 ;; label = @4
            local.get $0
            i32.load8_s
            i32.eqz
            if ;; label = @5
              block ;; label = @6
                local.get $1
                local.set $0
                br 4 (;@2;)
              end
            end
            local.get $0
            i32.const 1
            i32.add
            local.tee $0
            local.tee $1
            i32.const 3
            i32.and
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
        end
        loop $label$6 ;; label = @3
          local.get $0
          i32.const 4
          i32.add
          local.set $1
          local.get $0
          i32.load
          local.tee $3
          i32.const -2139062144
          i32.and
          i32.const -2139062144
          i32.xor
          local.get $3
          i32.const -16843009
          i32.add
          i32.and
          i32.eqz
          if ;; label = @4
            block ;; label = @5
              local.get $1
              local.set $0
              br 2 (;@3;)
            end
          end
        end
        local.get $3
        i32.const 255
        i32.and
        i32.const 24
        i32.shl
        i32.const 24
        i32.shr_s
        if ;; label = @3
          loop $label$9 ;; label = @4
            local.get $0
            i32.const 1
            i32.add
            local.tee $0
            i32.load8_s
            br_if 0 (;@4;)
          end
        end
      end
      local.get $0
      local.get $2
      i32.sub
    end
  )
  (func $32 (;45;) (type $6) (param $0 i32) (param $1 i32) (result i32)
    (local $2 i32) (local $3 i32) (local $4 i32) (local $5 i32) (local $6 i32) (local $7 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $3
      global.get $global$1
      i32.const 16
      i32.add
      global.set $global$1
      local.get $3
      local.tee $4
      local.get $1
      i32.const 255
      i32.and
      local.tee $7
      i32.store8
      block $label$2 ;; label = @2
        block $label$3 ;; label = @3
          local.get $0
          i32.const 16
          i32.add
          local.tee $2
          i32.load
          local.tee $5
          br_if 0 (;@3;)
          local.get $0
          call $30
          if ;; label = @4
            i32.const -1
            local.set $1
          else
            block ;; label = @5
              local.get $2
              i32.load
              local.set $5
              br 2 (;@3;)
            end
          end
          br 1 (;@2;)
        end
        local.get $0
        i32.const 20
        i32.add
        local.tee $2
        i32.load
        local.tee $6
        local.get $5
        i32.lt_u
        if ;; label = @3
          local.get $1
          i32.const 255
          i32.and
          local.tee $1
          local.get $0
          i32.load8_s offset=75
          i32.ne
          if ;; label = @4
            block ;; label = @5
              local.get $2
              local.get $6
              i32.const 1
              i32.add
              i32.store
              local.get $6
              local.get $7
              i32.store8
              br 3 (;@2;)
            end
          end
        end
        local.get $0
        local.get $4
        i32.const 1
        local.get $0
        i32.load offset=36
        i32.const 3
        i32.and
        i32.const 2
        i32.add
        call_indirect (type $0)
        i32.const 1
        i32.eq
        if (result i32) ;; label = @3
          local.get $4
          i32.load8_u
        else
          i32.const -1
        end
        local.set $1
      end
      local.get $3
      global.set $global$1
      local.get $1
    end
  )
  (func $33 (;46;) (type $12) (param $0 i32) (param $1 i32) (param $2 i32) (param $3 i32) (result i32)
    (local $4 i32) (local $5 i32)
    block $label$1 (result i32) ;; label = @1
      local.get $2
      local.get $1
      i32.mul
      local.set $4
      local.get $3
      i32.load offset=76
      i32.const -1
      i32.gt_s
      if ;; label = @2
        block ;; label = @3
          local.get $3
          call $20
          i32.eqz
          local.set $5
          local.get $0
          local.get $4
          local.get $3
          call $21
          local.set $0
          local.get $5
          i32.eqz
          if ;; label = @4
            local.get $3
            call $13
          end
        end
      else
        local.get $0
        local.get $4
        local.get $3
        call $21
        local.set $0
      end
      local.get $0
      local.get $4
      i32.ne
      if ;; label = @2
        local.get $0
        local.get $1
        i32.div_u
        local.set $2
      end
      local.get $2
    end
  )
  (func $34 (;47;) (type $6) (param $0 i32) (param $1 i32) (result i32)
    (local $2 i32) (local $3 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $2
      global.get $global$1
      i32.const 16
      i32.add
      global.set $global$1
      local.get $2
      local.tee $3
      local.get $1
      i32.store
      i32.const 1280
      i32.load
      local.get $0
      local.get $3
      call $18
      local.set $0
      local.get $2
      global.set $global$1
      local.get $0
    end
  )
  (func $35 (;48;) (type $2) (param $0 i32) (result i32)
    (local $1 i32) (local $2 i32) (local $3 i32)
    block $label$1 (result i32) ;; label = @1
      i32.const 1280
      i32.load
      local.tee $1
      i32.load offset=76
      i32.const -1
      i32.gt_s
      if (result i32) ;; label = @2
        local.get $1
        call $20
      else
        i32.const 0
      end
      local.set $2
      block $label$4 (result i32) ;; label = @2
        local.get $0
        local.get $1
        call $36
        i32.const 0
        i32.lt_s
        if (result i32) ;; label = @3
          i32.const 1
        else
          block (result i32) ;; label = @4
            local.get $1
            i32.load8_s offset=75
            i32.const 10
            i32.ne
            if ;; label = @5
              local.get $1
              i32.const 20
              i32.add
              local.tee $3
              i32.load
              local.tee $0
              local.get $1
              i32.load offset=16
              i32.lt_u
              if ;; label = @6
                block ;; label = @7
                  local.get $3
                  local.get $0
                  i32.const 1
                  i32.add
                  i32.store
                  local.get $0
                  i32.const 10
                  i32.store8
                  i32.const 0
                  br 5 (;@2;)
                end
              end
            end
            local.get $1
            i32.const 10
            call $32
            i32.const 0
            i32.lt_s
          end
        end
      end
      local.set $0
      local.get $2
      if ;; label = @2
        local.get $1
        call $13
      end
      local.get $0
      i32.const 31
      i32.shl
      i32.const 31
      i32.shr_s
    end
  )
  (func $36 (;49;) (type $6) (param $0 i32) (param $1 i32) (result i32)
    local.get $0
    local.get $0
    call $31
    i32.const 1
    local.get $1
    call $33
    i32.const -1
    i32.add
  )
  (func $37 (;50;) (type $2) (param $0 i32) (result i32)
    (local $1 i32) (local $2 i32) (local $3 i32) (local $4 i32) (local $5 i32) (local $6 i32) (local $7 i32) (local $8 i32) (local $9 i32) (local $10 i32) (local $11 i32) (local $12 i32) (local $13 i32) (local $14 i32) (local $15 i32) (local $16 i32) (local $17 i32) (local $18 i32) (local $19 i32) (local $20 i32) (local $21 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$1
      local.set $14
      global.get $global$1
      i32.const 16
      i32.add
      global.set $global$1
      local.get $14
      local.set $18
      block $label$2 ;; label = @2
        local.get $0
        i32.const 245
        i32.lt_u
        if ;; label = @3
          block ;; label = @4
            local.get $0
            i32.const 11
            i32.add
            i32.const -8
            i32.and
            local.set $3
            i32.const 4176
            i32.load
            local.tee $8
            local.get $0
            i32.const 11
            i32.lt_u
            if (result i32) ;; label = @5
              i32.const 16
              local.tee $3
            else
              local.get $3
            end
            i32.const 3
            i32.shr_u
            local.tee $2
            i32.shr_u
            local.tee $0
            i32.const 3
            i32.and
            if ;; label = @5
              block ;; label = @6
                local.get $0
                i32.const 1
                i32.and
                i32.const 1
                i32.xor
                local.get $2
                i32.add
                local.tee $5
                i32.const 1
                i32.shl
                i32.const 2
                i32.shl
                i32.const 4216
                i32.add
                local.tee $2
                i32.const 8
                i32.add
                local.tee $3
                i32.load
                local.tee $7
                i32.const 8
                i32.add
                local.tee $1
                i32.load
                local.set $4
                local.get $2
                local.get $4
                i32.eq
                if ;; label = @7
                  i32.const 4176
                  local.get $8
                  i32.const 1
                  local.get $5
                  i32.shl
                  i32.const -1
                  i32.xor
                  i32.and
                  i32.store
                else
                  block ;; label = @8
                    local.get $4
                    i32.const 4192
                    i32.load
                    i32.lt_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $4
                    i32.const 12
                    i32.add
                    local.tee $0
                    i32.load
                    local.get $7
                    i32.eq
                    if ;; label = @9
                      block ;; label = @10
                        local.get $0
                        local.get $2
                        i32.store
                        local.get $3
                        local.get $4
                        i32.store
                      end
                    else
                      call $fimport$8
                    end
                  end
                end
                local.get $7
                local.get $5
                i32.const 3
                i32.shl
                local.tee $0
                i32.const 3
                i32.or
                i32.store offset=4
                local.get $7
                local.get $0
                i32.add
                i32.const 4
                i32.add
                local.tee $0
                local.get $0
                i32.load
                i32.const 1
                i32.or
                i32.store
                local.get $14
                global.set $global$1
                local.get $1
                return
              end
            end
            local.get $3
            i32.const 4184
            i32.load
            local.tee $16
            i32.gt_u
            if ;; label = @5
              block ;; label = @6
                local.get $0
                if ;; label = @7
                  block ;; label = @8
                    local.get $0
                    local.get $2
                    i32.shl
                    i32.const 2
                    local.get $2
                    i32.shl
                    local.tee $0
                    i32.const 0
                    local.get $0
                    i32.sub
                    i32.or
                    i32.and
                    local.tee $0
                    i32.const 0
                    local.get $0
                    i32.sub
                    i32.and
                    i32.const -1
                    i32.add
                    local.tee $0
                    i32.const 12
                    i32.shr_u
                    i32.const 16
                    i32.and
                    local.set $5
                    local.get $0
                    local.get $5
                    i32.shr_u
                    local.tee $2
                    i32.const 5
                    i32.shr_u
                    i32.const 8
                    i32.and
                    local.tee $0
                    local.get $5
                    i32.or
                    local.get $2
                    local.get $0
                    i32.shr_u
                    local.tee $2
                    i32.const 2
                    i32.shr_u
                    i32.const 4
                    i32.and
                    local.tee $0
                    i32.or
                    local.get $2
                    local.get $0
                    i32.shr_u
                    local.tee $2
                    i32.const 1
                    i32.shr_u
                    i32.const 2
                    i32.and
                    local.tee $0
                    i32.or
                    local.get $2
                    local.get $0
                    i32.shr_u
                    local.tee $2
                    i32.const 1
                    i32.shr_u
                    i32.const 1
                    i32.and
                    local.tee $0
                    i32.or
                    local.get $2
                    local.get $0
                    i32.shr_u
                    i32.add
                    local.tee $11
                    i32.const 1
                    i32.shl
                    i32.const 2
                    i32.shl
                    i32.const 4216
                    i32.add
                    local.tee $4
                    i32.const 8
                    i32.add
                    local.tee $2
                    i32.load
                    local.tee $9
                    i32.const 8
                    i32.add
                    local.tee $5
                    i32.load
                    local.set $12
                    local.get $4
                    local.get $12
                    i32.eq
                    if ;; label = @9
                      i32.const 4176
                      local.get $8
                      i32.const 1
                      local.get $11
                      i32.shl
                      i32.const -1
                      i32.xor
                      i32.and
                      local.tee $7
                      i32.store
                    else
                      block ;; label = @10
                        local.get $12
                        i32.const 4192
                        i32.load
                        i32.lt_u
                        if ;; label = @11
                          call $fimport$8
                        end
                        local.get $12
                        i32.const 12
                        i32.add
                        local.tee $0
                        i32.load
                        local.get $9
                        i32.eq
                        if ;; label = @11
                          block ;; label = @12
                            local.get $0
                            local.get $4
                            i32.store
                            local.get $2
                            local.get $12
                            i32.store
                            local.get $8
                            local.set $7
                          end
                        else
                          call $fimport$8
                        end
                      end
                    end
                    local.get $9
                    local.get $3
                    i32.const 3
                    i32.or
                    i32.store offset=4
                    local.get $9
                    local.get $3
                    i32.add
                    local.tee $4
                    local.get $11
                    i32.const 3
                    i32.shl
                    local.get $3
                    i32.sub
                    local.tee $11
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get $4
                    local.get $11
                    i32.add
                    local.get $11
                    i32.store
                    local.get $16
                    if ;; label = @9
                      block ;; label = @10
                        i32.const 4196
                        i32.load
                        local.set $9
                        local.get $16
                        i32.const 3
                        i32.shr_u
                        local.tee $0
                        i32.const 1
                        i32.shl
                        i32.const 2
                        i32.shl
                        i32.const 4216
                        i32.add
                        local.set $2
                        local.get $7
                        i32.const 1
                        local.get $0
                        i32.shl
                        local.tee $0
                        i32.and
                        if ;; label = @11
                          local.get $2
                          i32.const 8
                          i32.add
                          local.tee $3
                          i32.load
                          local.tee $0
                          i32.const 4192
                          i32.load
                          i32.lt_u
                          if ;; label = @12
                            call $fimport$8
                          else
                            block ;; label = @13
                              local.get $3
                              local.set $6
                              local.get $0
                              local.set $1
                            end
                          end
                        else
                          block ;; label = @12
                            i32.const 4176
                            local.get $7
                            local.get $0
                            i32.or
                            i32.store
                            local.get $2
                            i32.const 8
                            i32.add
                            local.set $6
                            local.get $2
                            local.set $1
                          end
                        end
                        local.get $6
                        local.get $9
                        i32.store
                        local.get $1
                        local.get $9
                        i32.store offset=12
                        local.get $9
                        local.get $1
                        i32.store offset=8
                        local.get $9
                        local.get $2
                        i32.store offset=12
                      end
                    end
                    i32.const 4184
                    local.get $11
                    i32.store
                    i32.const 4196
                    local.get $4
                    i32.store
                    local.get $14
                    global.set $global$1
                    local.get $5
                    return
                  end
                end
                i32.const 4180
                i32.load
                local.tee $6
                if ;; label = @7
                  block ;; label = @8
                    local.get $6
                    i32.const 0
                    local.get $6
                    i32.sub
                    i32.and
                    i32.const -1
                    i32.add
                    local.tee $0
                    i32.const 12
                    i32.shr_u
                    i32.const 16
                    i32.and
                    local.set $2
                    local.get $0
                    local.get $2
                    i32.shr_u
                    local.tee $1
                    i32.const 5
                    i32.shr_u
                    i32.const 8
                    i32.and
                    local.tee $0
                    local.get $2
                    i32.or
                    local.get $1
                    local.get $0
                    i32.shr_u
                    local.tee $1
                    i32.const 2
                    i32.shr_u
                    i32.const 4
                    i32.and
                    local.tee $0
                    i32.or
                    local.get $1
                    local.get $0
                    i32.shr_u
                    local.tee $1
                    i32.const 1
                    i32.shr_u
                    i32.const 2
                    i32.and
                    local.tee $0
                    i32.or
                    local.get $1
                    local.get $0
                    i32.shr_u
                    local.tee $1
                    i32.const 1
                    i32.shr_u
                    i32.const 1
                    i32.and
                    local.tee $0
                    i32.or
                    local.get $1
                    local.get $0
                    i32.shr_u
                    i32.add
                    i32.const 2
                    i32.shl
                    i32.const 4480
                    i32.add
                    i32.load
                    local.tee $2
                    i32.load offset=4
                    i32.const -8
                    i32.and
                    local.get $3
                    i32.sub
                    local.set $9
                    local.get $2
                    local.set $1
                    loop $label$25 ;; label = @9
                      block $label$26 ;; label = @10
                        local.get $1
                        i32.load offset=16
                        local.tee $0
                        i32.eqz
                        if ;; label = @11
                          local.get $1
                          i32.load offset=20
                          local.tee $0
                          i32.eqz
                          br_if 1 (;@10;)
                        end
                        local.get $0
                        i32.load offset=4
                        i32.const -8
                        i32.and
                        local.get $3
                        i32.sub
                        local.tee $1
                        local.get $9
                        i32.lt_u
                        local.tee $7
                        if ;; label = @11
                          local.get $1
                          local.set $9
                        end
                        local.get $0
                        local.set $1
                        local.get $7
                        if ;; label = @11
                          local.get $0
                          local.set $2
                        end
                        br 1 (;@9;)
                      end
                    end
                    local.get $2
                    i32.const 4192
                    i32.load
                    local.tee $12
                    i32.lt_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $2
                    local.get $2
                    local.get $3
                    i32.add
                    local.tee $13
                    i32.ge_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $2
                    i32.load offset=24
                    local.set $15
                    block $label$32 ;; label = @9
                      local.get $2
                      i32.load offset=12
                      local.tee $0
                      local.get $2
                      i32.eq
                      if ;; label = @10
                        block ;; label = @11
                          local.get $2
                          i32.const 20
                          i32.add
                          local.tee $1
                          i32.load
                          local.tee $0
                          i32.eqz
                          if ;; label = @12
                            local.get $2
                            i32.const 16
                            i32.add
                            local.tee $1
                            i32.load
                            local.tee $0
                            i32.eqz
                            if ;; label = @13
                              block ;; label = @14
                                i32.const 0
                                local.set $4
                                br 5 (;@9;)
                              end
                            end
                          end
                          loop $label$36 ;; label = @12
                            local.get $0
                            i32.const 20
                            i32.add
                            local.tee $11
                            i32.load
                            local.tee $7
                            if ;; label = @13
                              block ;; label = @14
                                local.get $7
                                local.set $0
                                local.get $11
                                local.set $1
                                br 2 (;@12;)
                              end
                            end
                            local.get $0
                            i32.const 16
                            i32.add
                            local.tee $11
                            i32.load
                            local.tee $7
                            if ;; label = @13
                              block ;; label = @14
                                local.get $7
                                local.set $0
                                local.get $11
                                local.set $1
                                br 2 (;@12;)
                              end
                            end
                          end
                          local.get $1
                          local.get $12
                          i32.lt_u
                          if ;; label = @12
                            call $fimport$8
                          else
                            block ;; label = @13
                              local.get $1
                              i32.const 0
                              i32.store
                              local.get $0
                              local.set $4
                            end
                          end
                        end
                      else
                        block ;; label = @11
                          local.get $2
                          i32.load offset=8
                          local.tee $11
                          local.get $12
                          i32.lt_u
                          if ;; label = @12
                            call $fimport$8
                          end
                          local.get $11
                          i32.const 12
                          i32.add
                          local.tee $7
                          i32.load
                          local.get $2
                          i32.ne
                          if ;; label = @12
                            call $fimport$8
                          end
                          local.get $0
                          i32.const 8
                          i32.add
                          local.tee $1
                          i32.load
                          local.get $2
                          i32.eq
                          if ;; label = @12
                            block ;; label = @13
                              local.get $7
                              local.get $0
                              i32.store
                              local.get $1
                              local.get $11
                              i32.store
                              local.get $0
                              local.set $4
                            end
                          else
                            call $fimport$8
                          end
                        end
                      end
                    end
                    block $label$46 ;; label = @9
                      local.get $15
                      if ;; label = @10
                        block ;; label = @11
                          local.get $2
                          local.get $2
                          i32.load offset=28
                          local.tee $1
                          i32.const 2
                          i32.shl
                          i32.const 4480
                          i32.add
                          local.tee $0
                          i32.load
                          i32.eq
                          if ;; label = @12
                            block ;; label = @13
                              local.get $0
                              local.get $4
                              i32.store
                              local.get $4
                              i32.eqz
                              if ;; label = @14
                                block ;; label = @15
                                  i32.const 4180
                                  local.get $6
                                  i32.const 1
                                  local.get $1
                                  i32.shl
                                  i32.const -1
                                  i32.xor
                                  i32.and
                                  i32.store
                                  br 6 (;@9;)
                                end
                              end
                            end
                          else
                            block ;; label = @13
                              local.get $15
                              i32.const 4192
                              i32.load
                              i32.lt_u
                              if ;; label = @14
                                call $fimport$8
                              end
                              local.get $15
                              i32.const 16
                              i32.add
                              local.tee $0
                              i32.load
                              local.get $2
                              i32.eq
                              if ;; label = @14
                                local.get $0
                                local.get $4
                                i32.store
                              else
                                local.get $15
                                local.get $4
                                i32.store offset=20
                              end
                              local.get $4
                              i32.eqz
                              br_if 4 (;@9;)
                            end
                          end
                          local.get $4
                          i32.const 4192
                          i32.load
                          local.tee $0
                          i32.lt_u
                          if ;; label = @12
                            call $fimport$8
                          end
                          local.get $4
                          local.get $15
                          i32.store offset=24
                          local.get $2
                          i32.load offset=16
                          local.tee $1
                          if ;; label = @12
                            local.get $1
                            local.get $0
                            i32.lt_u
                            if ;; label = @13
                              call $fimport$8
                            else
                              block ;; label = @14
                                local.get $4
                                local.get $1
                                i32.store offset=16
                                local.get $1
                                local.get $4
                                i32.store offset=24
                              end
                            end
                          end
                          local.get $2
                          i32.load offset=20
                          local.tee $0
                          if ;; label = @12
                            local.get $0
                            i32.const 4192
                            i32.load
                            i32.lt_u
                            if ;; label = @13
                              call $fimport$8
                            else
                              block ;; label = @14
                                local.get $4
                                local.get $0
                                i32.store offset=20
                                local.get $0
                                local.get $4
                                i32.store offset=24
                              end
                            end
                          end
                        end
                      end
                    end
                    local.get $9
                    i32.const 16
                    i32.lt_u
                    if ;; label = @9
                      block ;; label = @10
                        local.get $2
                        local.get $9
                        local.get $3
                        i32.add
                        local.tee $0
                        i32.const 3
                        i32.or
                        i32.store offset=4
                        local.get $2
                        local.get $0
                        i32.add
                        i32.const 4
                        i32.add
                        local.tee $0
                        local.get $0
                        i32.load
                        i32.const 1
                        i32.or
                        i32.store
                      end
                    else
                      block ;; label = @10
                        local.get $2
                        local.get $3
                        i32.const 3
                        i32.or
                        i32.store offset=4
                        local.get $13
                        local.get $9
                        i32.const 1
                        i32.or
                        i32.store offset=4
                        local.get $13
                        local.get $9
                        i32.add
                        local.get $9
                        i32.store
                        local.get $16
                        if ;; label = @11
                          block ;; label = @12
                            i32.const 4196
                            i32.load
                            local.set $7
                            local.get $16
                            i32.const 3
                            i32.shr_u
                            local.tee $0
                            i32.const 1
                            i32.shl
                            i32.const 2
                            i32.shl
                            i32.const 4216
                            i32.add
                            local.set $3
                            local.get $8
                            i32.const 1
                            local.get $0
                            i32.shl
                            local.tee $0
                            i32.and
                            if ;; label = @13
                              local.get $3
                              i32.const 8
                              i32.add
                              local.tee $1
                              i32.load
                              local.tee $0
                              i32.const 4192
                              i32.load
                              i32.lt_u
                              if ;; label = @14
                                call $fimport$8
                              else
                                block ;; label = @15
                                  local.get $1
                                  local.set $10
                                  local.get $0
                                  local.set $5
                                end
                              end
                            else
                              block ;; label = @14
                                i32.const 4176
                                local.get $8
                                local.get $0
                                i32.or
                                i32.store
                                local.get $3
                                i32.const 8
                                i32.add
                                local.set $10
                                local.get $3
                                local.set $5
                              end
                            end
                            local.get $10
                            local.get $7
                            i32.store
                            local.get $5
                            local.get $7
                            i32.store offset=12
                            local.get $7
                            local.get $5
                            i32.store offset=8
                            local.get $7
                            local.get $3
                            i32.store offset=12
                          end
                        end
                        i32.const 4184
                        local.get $9
                        i32.store
                        i32.const 4196
                        local.get $13
                        i32.store
                      end
                    end
                    local.get $14
                    global.set $global$1
                    local.get $2
                    i32.const 8
                    i32.add
                    return
                  end
                else
                  local.get $3
                  local.set $0
                end
              end
            else
              local.get $3
              local.set $0
            end
          end
        else
          local.get $0
          i32.const -65
          i32.gt_u
          if ;; label = @4
            i32.const -1
            local.set $0
          else
            block ;; label = @5
              local.get $0
              i32.const 11
              i32.add
              local.tee $0
              i32.const -8
              i32.and
              local.set $7
              i32.const 4180
              i32.load
              local.tee $5
              if ;; label = @6
                block ;; label = @7
                  local.get $0
                  i32.const 8
                  i32.shr_u
                  local.tee $0
                  if (result i32) ;; label = @8
                    local.get $7
                    i32.const 16777215
                    i32.gt_u
                    if (result i32) ;; label = @9
                      i32.const 31
                    else
                      local.get $7
                      i32.const 14
                      local.get $0
                      local.get $0
                      i32.const 1048320
                      i32.add
                      i32.const 16
                      i32.shr_u
                      i32.const 8
                      i32.and
                      local.tee $3
                      i32.shl
                      local.tee $1
                      i32.const 520192
                      i32.add
                      i32.const 16
                      i32.shr_u
                      i32.const 4
                      i32.and
                      local.tee $0
                      local.get $3
                      i32.or
                      local.get $1
                      local.get $0
                      i32.shl
                      local.tee $1
                      i32.const 245760
                      i32.add
                      i32.const 16
                      i32.shr_u
                      i32.const 2
                      i32.and
                      local.tee $0
                      i32.or
                      i32.sub
                      local.get $1
                      local.get $0
                      i32.shl
                      i32.const 15
                      i32.shr_u
                      i32.add
                      local.tee $0
                      i32.const 7
                      i32.add
                      i32.shr_u
                      i32.const 1
                      i32.and
                      local.get $0
                      i32.const 1
                      i32.shl
                      i32.or
                    end
                  else
                    i32.const 0
                  end
                  local.set $17
                  i32.const 0
                  local.get $7
                  i32.sub
                  local.set $3
                  block $label$78 ;; label = @8
                    block $label$79 ;; label = @9
                      block $label$80 ;; label = @10
                        local.get $17
                        i32.const 2
                        i32.shl
                        i32.const 4480
                        i32.add
                        i32.load
                        local.tee $1
                        if ;; label = @11
                          block ;; label = @12
                            i32.const 25
                            local.get $17
                            i32.const 1
                            i32.shr_u
                            i32.sub
                            local.set $0
                            i32.const 0
                            local.set $4
                            local.get $7
                            local.get $17
                            i32.const 31
                            i32.eq
                            if (result i32) ;; label = @13
                              i32.const 0
                            else
                              local.get $0
                            end
                            i32.shl
                            local.set $10
                            i32.const 0
                            local.set $0
                            loop $label$84 ;; label = @13
                              local.get $1
                              i32.load offset=4
                              i32.const -8
                              i32.and
                              local.get $7
                              i32.sub
                              local.tee $6
                              local.get $3
                              i32.lt_u
                              if ;; label = @14
                                local.get $6
                                if ;; label = @15
                                  block ;; label = @16
                                    local.get $6
                                    local.set $3
                                    local.get $1
                                    local.set $0
                                  end
                                else
                                  block ;; label = @16
                                    i32.const 0
                                    local.set $3
                                    local.get $1
                                    local.set $0
                                    br 7 (;@9;)
                                  end
                                end
                              end
                              local.get $1
                              i32.load offset=20
                              local.tee $19
                              i32.eqz
                              local.get $19
                              local.get $1
                              i32.const 16
                              i32.add
                              local.get $10
                              i32.const 31
                              i32.shr_u
                              i32.const 2
                              i32.shl
                              i32.add
                              i32.load
                              local.tee $6
                              i32.eq
                              i32.or
                              if (result i32) ;; label = @14
                                local.get $4
                              else
                                local.get $19
                              end
                              local.set $1
                              local.get $10
                              local.get $6
                              i32.eqz
                              local.tee $4
                              i32.const 1
                              i32.and
                              i32.const 1
                              i32.xor
                              i32.shl
                              local.set $10
                              local.get $4
                              if ;; label = @14
                                block ;; label = @15
                                  local.get $1
                                  local.set $4
                                  local.get $0
                                  local.set $1
                                  br 5 (;@10;)
                                end
                              else
                                block ;; label = @15
                                  local.get $1
                                  local.set $4
                                  local.get $6
                                  local.set $1
                                  br 2 (;@13;)
                                end
                              end
                            end
                          end
                        else
                          block ;; label = @12
                            i32.const 0
                            local.set $4
                            i32.const 0
                            local.set $1
                          end
                        end
                      end
                      local.get $4
                      i32.eqz
                      local.get $1
                      i32.eqz
                      i32.and
                      if (result i32) ;; label = @10
                        block (result i32) ;; label = @11
                          local.get $5
                          i32.const 2
                          local.get $17
                          i32.shl
                          local.tee $0
                          i32.const 0
                          local.get $0
                          i32.sub
                          i32.or
                          i32.and
                          local.tee $0
                          i32.eqz
                          if ;; label = @12
                            block ;; label = @13
                              local.get $7
                              local.set $0
                              br 11 (;@2;)
                            end
                          end
                          local.get $0
                          i32.const 0
                          local.get $0
                          i32.sub
                          i32.and
                          i32.const -1
                          i32.add
                          local.tee $0
                          i32.const 12
                          i32.shr_u
                          i32.const 16
                          i32.and
                          local.set $10
                          local.get $0
                          local.get $10
                          i32.shr_u
                          local.tee $4
                          i32.const 5
                          i32.shr_u
                          i32.const 8
                          i32.and
                          local.tee $0
                          local.get $10
                          i32.or
                          local.get $4
                          local.get $0
                          i32.shr_u
                          local.tee $4
                          i32.const 2
                          i32.shr_u
                          i32.const 4
                          i32.and
                          local.tee $0
                          i32.or
                          local.get $4
                          local.get $0
                          i32.shr_u
                          local.tee $4
                          i32.const 1
                          i32.shr_u
                          i32.const 2
                          i32.and
                          local.tee $0
                          i32.or
                          local.get $4
                          local.get $0
                          i32.shr_u
                          local.tee $4
                          i32.const 1
                          i32.shr_u
                          i32.const 1
                          i32.and
                          local.tee $0
                          i32.or
                          local.get $4
                          local.get $0
                          i32.shr_u
                          i32.add
                          i32.const 2
                          i32.shl
                          i32.const 4480
                          i32.add
                          i32.load
                        end
                      else
                        local.get $4
                      end
                      local.tee $0
                      br_if 0 (;@9;)
                      local.get $1
                      local.set $4
                      br 1 (;@8;)
                    end
                    loop $label$96 ;; label = @9
                      local.get $0
                      i32.load offset=4
                      i32.const -8
                      i32.and
                      local.get $7
                      i32.sub
                      local.tee $4
                      local.get $3
                      i32.lt_u
                      local.tee $10
                      if ;; label = @10
                        local.get $4
                        local.set $3
                      end
                      local.get $10
                      if ;; label = @10
                        local.get $0
                        local.set $1
                      end
                      local.get $0
                      i32.load offset=16
                      local.tee $4
                      if ;; label = @10
                        block ;; label = @11
                          local.get $4
                          local.set $0
                          br 2 (;@9;)
                        end
                      end
                      local.get $0
                      i32.load offset=20
                      local.tee $0
                      br_if 0 (;@9;)
                      local.get $1
                      local.set $4
                    end
                  end
                  local.get $4
                  if ;; label = @8
                    local.get $3
                    i32.const 4184
                    i32.load
                    local.get $7
                    i32.sub
                    i32.lt_u
                    if ;; label = @9
                      block ;; label = @10
                        local.get $4
                        i32.const 4192
                        i32.load
                        local.tee $12
                        i32.lt_u
                        if ;; label = @11
                          call $fimport$8
                        end
                        local.get $4
                        local.get $4
                        local.get $7
                        i32.add
                        local.tee $6
                        i32.ge_u
                        if ;; label = @11
                          call $fimport$8
                        end
                        local.get $4
                        i32.load offset=24
                        local.set $10
                        block $label$104 ;; label = @11
                          local.get $4
                          i32.load offset=12
                          local.tee $0
                          local.get $4
                          i32.eq
                          if ;; label = @12
                            block ;; label = @13
                              local.get $4
                              i32.const 20
                              i32.add
                              local.tee $1
                              i32.load
                              local.tee $0
                              i32.eqz
                              if ;; label = @14
                                local.get $4
                                i32.const 16
                                i32.add
                                local.tee $1
                                i32.load
                                local.tee $0
                                i32.eqz
                                if ;; label = @15
                                  block ;; label = @16
                                    i32.const 0
                                    local.set $13
                                    br 5 (;@11;)
                                  end
                                end
                              end
                              loop $label$108 ;; label = @14
                                local.get $0
                                i32.const 20
                                i32.add
                                local.tee $9
                                i32.load
                                local.tee $11
                                if ;; label = @15
                                  block ;; label = @16
                                    local.get $11
                                    local.set $0
                                    local.get $9
                                    local.set $1
                                    br 2 (;@14;)
                                  end
                                end
                                local.get $0
                                i32.const 16
                                i32.add
                                local.tee $9
                                i32.load
                                local.tee $11
                                if ;; label = @15
                                  block ;; label = @16
                                    local.get $11
                                    local.set $0
                                    local.get $9
                                    local.set $1
                                    br 2 (;@14;)
                                  end
                                end
                              end
                              local.get $1
                              local.get $12
                              i32.lt_u
                              if ;; label = @14
                                call $fimport$8
                              else
                                block ;; label = @15
                                  local.get $1
                                  i32.const 0
                                  i32.store
                                  local.get $0
                                  local.set $13
                                end
                              end
                            end
                          else
                            block ;; label = @13
                              local.get $4
                              i32.load offset=8
                              local.tee $9
                              local.get $12
                              i32.lt_u
                              if ;; label = @14
                                call $fimport$8
                              end
                              local.get $9
                              i32.const 12
                              i32.add
                              local.tee $11
                              i32.load
                              local.get $4
                              i32.ne
                              if ;; label = @14
                                call $fimport$8
                              end
                              local.get $0
                              i32.const 8
                              i32.add
                              local.tee $1
                              i32.load
                              local.get $4
                              i32.eq
                              if ;; label = @14
                                block ;; label = @15
                                  local.get $11
                                  local.get $0
                                  i32.store
                                  local.get $1
                                  local.get $9
                                  i32.store
                                  local.get $0
                                  local.set $13
                                end
                              else
                                call $fimport$8
                              end
                            end
                          end
                        end
                        block $label$118 ;; label = @11
                          local.get $10
                          if ;; label = @12
                            block ;; label = @13
                              local.get $4
                              local.get $4
                              i32.load offset=28
                              local.tee $1
                              i32.const 2
                              i32.shl
                              i32.const 4480
                              i32.add
                              local.tee $0
                              i32.load
                              i32.eq
                              if ;; label = @14
                                block ;; label = @15
                                  local.get $0
                                  local.get $13
                                  i32.store
                                  local.get $13
                                  i32.eqz
                                  if ;; label = @16
                                    block ;; label = @17
                                      i32.const 4180
                                      local.get $5
                                      i32.const 1
                                      local.get $1
                                      i32.shl
                                      i32.const -1
                                      i32.xor
                                      i32.and
                                      local.tee $2
                                      i32.store
                                      br 6 (;@11;)
                                    end
                                  end
                                end
                              else
                                block ;; label = @15
                                  local.get $10
                                  i32.const 4192
                                  i32.load
                                  i32.lt_u
                                  if ;; label = @16
                                    call $fimport$8
                                  end
                                  local.get $10
                                  i32.const 16
                                  i32.add
                                  local.tee $0
                                  i32.load
                                  local.get $4
                                  i32.eq
                                  if ;; label = @16
                                    local.get $0
                                    local.get $13
                                    i32.store
                                  else
                                    local.get $10
                                    local.get $13
                                    i32.store offset=20
                                  end
                                  local.get $13
                                  i32.eqz
                                  if ;; label = @16
                                    block ;; label = @17
                                      local.get $5
                                      local.set $2
                                      br 6 (;@11;)
                                    end
                                  end
                                end
                              end
                              local.get $13
                              i32.const 4192
                              i32.load
                              local.tee $0
                              i32.lt_u
                              if ;; label = @14
                                call $fimport$8
                              end
                              local.get $13
                              local.get $10
                              i32.store offset=24
                              local.get $4
                              i32.load offset=16
                              local.tee $1
                              if ;; label = @14
                                local.get $1
                                local.get $0
                                i32.lt_u
                                if ;; label = @15
                                  call $fimport$8
                                else
                                  block ;; label = @16
                                    local.get $13
                                    local.get $1
                                    i32.store offset=16
                                    local.get $1
                                    local.get $13
                                    i32.store offset=24
                                  end
                                end
                              end
                              local.get $4
                              i32.load offset=20
                              local.tee $0
                              if ;; label = @14
                                local.get $0
                                i32.const 4192
                                i32.load
                                i32.lt_u
                                if ;; label = @15
                                  call $fimport$8
                                else
                                  block ;; label = @16
                                    local.get $13
                                    local.get $0
                                    i32.store offset=20
                                    local.get $0
                                    local.get $13
                                    i32.store offset=24
                                    local.get $5
                                    local.set $2
                                  end
                                end
                              else
                                local.get $5
                                local.set $2
                              end
                            end
                          else
                            local.get $5
                            local.set $2
                          end
                        end
                        block $label$136 ;; label = @11
                          local.get $3
                          i32.const 16
                          i32.lt_u
                          if ;; label = @12
                            block ;; label = @13
                              local.get $4
                              local.get $3
                              local.get $7
                              i32.add
                              local.tee $0
                              i32.const 3
                              i32.or
                              i32.store offset=4
                              local.get $4
                              local.get $0
                              i32.add
                              i32.const 4
                              i32.add
                              local.tee $0
                              local.get $0
                              i32.load
                              i32.const 1
                              i32.or
                              i32.store
                            end
                          else
                            block ;; label = @13
                              local.get $4
                              local.get $7
                              i32.const 3
                              i32.or
                              i32.store offset=4
                              local.get $6
                              local.get $3
                              i32.const 1
                              i32.or
                              i32.store offset=4
                              local.get $6
                              local.get $3
                              i32.add
                              local.get $3
                              i32.store
                              local.get $3
                              i32.const 3
                              i32.shr_u
                              local.set $0
                              local.get $3
                              i32.const 256
                              i32.lt_u
                              if ;; label = @14
                                block ;; label = @15
                                  local.get $0
                                  i32.const 1
                                  i32.shl
                                  i32.const 2
                                  i32.shl
                                  i32.const 4216
                                  i32.add
                                  local.set $3
                                  i32.const 4176
                                  i32.load
                                  local.tee $1
                                  i32.const 1
                                  local.get $0
                                  i32.shl
                                  local.tee $0
                                  i32.and
                                  if ;; label = @16
                                    local.get $3
                                    i32.const 8
                                    i32.add
                                    local.tee $1
                                    i32.load
                                    local.tee $0
                                    i32.const 4192
                                    i32.load
                                    i32.lt_u
                                    if ;; label = @17
                                      call $fimport$8
                                    else
                                      block ;; label = @18
                                        local.get $1
                                        local.set $16
                                        local.get $0
                                        local.set $8
                                      end
                                    end
                                  else
                                    block ;; label = @17
                                      i32.const 4176
                                      local.get $1
                                      local.get $0
                                      i32.or
                                      i32.store
                                      local.get $3
                                      i32.const 8
                                      i32.add
                                      local.set $16
                                      local.get $3
                                      local.set $8
                                    end
                                  end
                                  local.get $16
                                  local.get $6
                                  i32.store
                                  local.get $8
                                  local.get $6
                                  i32.store offset=12
                                  local.get $6
                                  local.get $8
                                  i32.store offset=8
                                  local.get $6
                                  local.get $3
                                  i32.store offset=12
                                  br 4 (;@11;)
                                end
                              end
                              local.get $3
                              i32.const 8
                              i32.shr_u
                              local.tee $0
                              if (result i32) ;; label = @14
                                local.get $3
                                i32.const 16777215
                                i32.gt_u
                                if (result i32) ;; label = @15
                                  i32.const 31
                                else
                                  local.get $3
                                  i32.const 14
                                  local.get $0
                                  local.get $0
                                  i32.const 1048320
                                  i32.add
                                  i32.const 16
                                  i32.shr_u
                                  i32.const 8
                                  i32.and
                                  local.tee $5
                                  i32.shl
                                  local.tee $1
                                  i32.const 520192
                                  i32.add
                                  i32.const 16
                                  i32.shr_u
                                  i32.const 4
                                  i32.and
                                  local.tee $0
                                  local.get $5
                                  i32.or
                                  local.get $1
                                  local.get $0
                                  i32.shl
                                  local.tee $1
                                  i32.const 245760
                                  i32.add
                                  i32.const 16
                                  i32.shr_u
                                  i32.const 2
                                  i32.and
                                  local.tee $0
                                  i32.or
                                  i32.sub
                                  local.get $1
                                  local.get $0
                                  i32.shl
                                  i32.const 15
                                  i32.shr_u
                                  i32.add
                                  local.tee $0
                                  i32.const 7
                                  i32.add
                                  i32.shr_u
                                  i32.const 1
                                  i32.and
                                  local.get $0
                                  i32.const 1
                                  i32.shl
                                  i32.or
                                end
                              else
                                i32.const 0
                              end
                              local.tee $5
                              i32.const 2
                              i32.shl
                              i32.const 4480
                              i32.add
                              local.set $1
                              local.get $6
                              local.get $5
                              i32.store offset=28
                              local.get $6
                              i32.const 16
                              i32.add
                              local.tee $0
                              i32.const 0
                              i32.store offset=4
                              local.get $0
                              i32.const 0
                              i32.store
                              local.get $2
                              i32.const 1
                              local.get $5
                              i32.shl
                              local.tee $0
                              i32.and
                              i32.eqz
                              if ;; label = @14
                                block ;; label = @15
                                  i32.const 4180
                                  local.get $2
                                  local.get $0
                                  i32.or
                                  i32.store
                                  local.get $1
                                  local.get $6
                                  i32.store
                                  local.get $6
                                  local.get $1
                                  i32.store offset=24
                                  local.get $6
                                  local.get $6
                                  i32.store offset=12
                                  local.get $6
                                  local.get $6
                                  i32.store offset=8
                                  br 4 (;@11;)
                                end
                              end
                              local.get $1
                              i32.load
                              local.set $0
                              i32.const 25
                              local.get $5
                              i32.const 1
                              i32.shr_u
                              i32.sub
                              local.set $1
                              local.get $3
                              local.get $5
                              i32.const 31
                              i32.eq
                              if (result i32) ;; label = @14
                                i32.const 0
                              else
                                local.get $1
                              end
                              i32.shl
                              local.set $5
                              block $label$151 ;; label = @14
                                block $label$152 ;; label = @15
                                  block $label$153 ;; label = @16
                                    loop $label$154 ;; label = @17
                                      local.get $0
                                      i32.load offset=4
                                      i32.const -8
                                      i32.and
                                      local.get $3
                                      i32.eq
                                      br_if 2 (;@15;)
                                      local.get $5
                                      i32.const 1
                                      i32.shl
                                      local.set $2
                                      local.get $0
                                      i32.const 16
                                      i32.add
                                      local.get $5
                                      i32.const 31
                                      i32.shr_u
                                      i32.const 2
                                      i32.shl
                                      i32.add
                                      local.tee $5
                                      i32.load
                                      local.tee $1
                                      i32.eqz
                                      br_if 1 (;@16;)
                                      local.get $2
                                      local.set $5
                                      local.get $1
                                      local.set $0
                                      br 0 (;@17;)
                                    end
                                  end
                                  local.get $5
                                  i32.const 4192
                                  i32.load
                                  i32.lt_u
                                  if ;; label = @16
                                    call $fimport$8
                                  else
                                    block ;; label = @17
                                      local.get $5
                                      local.get $6
                                      i32.store
                                      local.get $6
                                      local.get $0
                                      i32.store offset=24
                                      local.get $6
                                      local.get $6
                                      i32.store offset=12
                                      local.get $6
                                      local.get $6
                                      i32.store offset=8
                                      br 6 (;@11;)
                                    end
                                  end
                                  br 1 (;@14;)
                                end
                                local.get $0
                                i32.const 8
                                i32.add
                                local.tee $3
                                i32.load
                                local.tee $2
                                i32.const 4192
                                i32.load
                                local.tee $1
                                i32.ge_u
                                local.get $0
                                local.get $1
                                i32.ge_u
                                i32.and
                                if ;; label = @15
                                  block ;; label = @16
                                    local.get $2
                                    local.get $6
                                    i32.store offset=12
                                    local.get $3
                                    local.get $6
                                    i32.store
                                    local.get $6
                                    local.get $2
                                    i32.store offset=8
                                    local.get $6
                                    local.get $0
                                    i32.store offset=12
                                    local.get $6
                                    i32.const 0
                                    i32.store offset=24
                                  end
                                else
                                  call $fimport$8
                                end
                              end
                            end
                          end
                        end
                        local.get $14
                        global.set $global$1
                        local.get $4
                        i32.const 8
                        i32.add
                        return
                      end
                    else
                      local.get $7
                      local.set $0
                    end
                  else
                    local.get $7
                    local.set $0
                  end
                end
              else
                local.get $7
                local.set $0
              end
            end
          end
        end
      end
      i32.const 4184
      i32.load
      local.tee $1
      local.get $0
      i32.ge_u
      if ;; label = @2
        block ;; label = @3
          i32.const 4196
          i32.load
          local.set $2
          local.get $1
          local.get $0
          i32.sub
          local.tee $3
          i32.const 15
          i32.gt_u
          if ;; label = @4
            block ;; label = @5
              i32.const 4196
              local.get $2
              local.get $0
              i32.add
              local.tee $1
              i32.store
              i32.const 4184
              local.get $3
              i32.store
              local.get $1
              local.get $3
              i32.const 1
              i32.or
              i32.store offset=4
              local.get $1
              local.get $3
              i32.add
              local.get $3
              i32.store
              local.get $2
              local.get $0
              i32.const 3
              i32.or
              i32.store offset=4
            end
          else
            block ;; label = @5
              i32.const 4184
              i32.const 0
              i32.store
              i32.const 4196
              i32.const 0
              i32.store
              local.get $2
              local.get $1
              i32.const 3
              i32.or
              i32.store offset=4
              local.get $2
              local.get $1
              i32.add
              i32.const 4
              i32.add
              local.tee $0
              local.get $0
              i32.load
              i32.const 1
              i32.or
              i32.store
            end
          end
          local.get $14
          global.set $global$1
          local.get $2
          i32.const 8
          i32.add
          return
        end
      end
      i32.const 4188
      i32.load
      local.tee $10
      local.get $0
      i32.gt_u
      if ;; label = @2
        block ;; label = @3
          i32.const 4188
          local.get $10
          local.get $0
          i32.sub
          local.tee $3
          i32.store
          i32.const 4200
          i32.const 4200
          i32.load
          local.tee $2
          local.get $0
          i32.add
          local.tee $1
          i32.store
          local.get $1
          local.get $3
          i32.const 1
          i32.or
          i32.store offset=4
          local.get $2
          local.get $0
          i32.const 3
          i32.or
          i32.store offset=4
          local.get $14
          global.set $global$1
          local.get $2
          i32.const 8
          i32.add
          return
        end
      end
      i32.const 4648
      i32.load
      if (result i32) ;; label = @2
        i32.const 4656
        i32.load
      else
        block (result i32) ;; label = @3
          i32.const 4656
          i32.const 4096
          i32.store
          i32.const 4652
          i32.const 4096
          i32.store
          i32.const 4660
          i32.const -1
          i32.store
          i32.const 4664
          i32.const -1
          i32.store
          i32.const 4668
          i32.const 0
          i32.store
          i32.const 4620
          i32.const 0
          i32.store
          local.get $18
          local.get $18
          i32.const -16
          i32.and
          i32.const 1431655768
          i32.xor
          local.tee $1
          i32.store
          i32.const 4648
          local.get $1
          i32.store
          i32.const 4096
        end
      end
      local.tee $1
      local.get $0
      i32.const 47
      i32.add
      local.tee $13
      i32.add
      local.tee $8
      i32.const 0
      local.get $1
      i32.sub
      local.tee $4
      i32.and
      local.tee $6
      local.get $0
      i32.le_u
      if ;; label = @2
        block ;; label = @3
          local.get $14
          global.set $global$1
          i32.const 0
          return
        end
      end
      i32.const 4616
      i32.load
      local.tee $2
      if ;; label = @2
        i32.const 4608
        i32.load
        local.tee $3
        local.get $6
        i32.add
        local.tee $1
        local.get $3
        i32.le_u
        local.get $1
        local.get $2
        i32.gt_u
        i32.or
        if ;; label = @3
          block ;; label = @4
            local.get $14
            global.set $global$1
            i32.const 0
            return
          end
        end
      end
      local.get $0
      i32.const 48
      i32.add
      local.set $7
      block $label$171 ;; label = @2
        block $label$172 ;; label = @3
          i32.const 4620
          i32.load
          i32.const 4
          i32.and
          i32.eqz
          if ;; label = @4
            block ;; label = @5
              block $label$174 ;; label = @6
                block $label$175 ;; label = @7
                  block $label$176 ;; label = @8
                    i32.const 4200
                    i32.load
                    local.tee $3
                    i32.eqz
                    br_if 0 (;@8;)
                    i32.const 4624
                    local.set $2
                    loop $label$177 ;; label = @9
                      block $label$178 ;; label = @10
                        local.get $2
                        i32.load
                        local.tee $1
                        local.get $3
                        i32.le_u
                        if ;; label = @11
                          local.get $1
                          local.get $2
                          i32.const 4
                          i32.add
                          local.tee $5
                          i32.load
                          i32.add
                          local.get $3
                          i32.gt_u
                          br_if 1 (;@10;)
                        end
                        local.get $2
                        i32.load offset=8
                        local.tee $1
                        i32.eqz
                        br_if 2 (;@8;)
                        local.get $1
                        local.set $2
                        br 1 (;@9;)
                      end
                    end
                    local.get $8
                    local.get $10
                    i32.sub
                    local.get $4
                    i32.and
                    local.tee $3
                    i32.const 2147483647
                    i32.lt_u
                    if ;; label = @9
                      local.get $3
                      call $45
                      local.tee $1
                      local.get $2
                      i32.load
                      local.get $5
                      i32.load
                      i32.add
                      i32.eq
                      if ;; label = @10
                        local.get $1
                        i32.const -1
                        i32.ne
                        br_if 7 (;@3;)
                      else
                        block ;; label = @11
                          local.get $1
                          local.set $2
                          local.get $3
                          local.set $1
                          br 4 (;@7;)
                        end
                      end
                    end
                    br 2 (;@6;)
                  end
                  i32.const 0
                  call $45
                  local.tee $1
                  i32.const -1
                  i32.ne
                  if ;; label = @8
                    block ;; label = @9
                      i32.const 4652
                      i32.load
                      local.tee $2
                      i32.const -1
                      i32.add
                      local.tee $5
                      local.get $1
                      local.tee $3
                      i32.add
                      i32.const 0
                      local.get $2
                      i32.sub
                      i32.and
                      local.get $3
                      i32.sub
                      local.set $2
                      local.get $5
                      local.get $3
                      i32.and
                      if (result i32) ;; label = @10
                        local.get $2
                      else
                        i32.const 0
                      end
                      local.get $6
                      i32.add
                      local.tee $3
                      i32.const 4608
                      i32.load
                      local.tee $5
                      i32.add
                      local.set $4
                      local.get $3
                      local.get $0
                      i32.gt_u
                      local.get $3
                      i32.const 2147483647
                      i32.lt_u
                      i32.and
                      if ;; label = @10
                        block ;; label = @11
                          i32.const 4616
                          i32.load
                          local.tee $2
                          if ;; label = @12
                            local.get $4
                            local.get $5
                            i32.le_u
                            local.get $4
                            local.get $2
                            i32.gt_u
                            i32.or
                            br_if 6 (;@6;)
                          end
                          local.get $3
                          call $45
                          local.tee $2
                          local.get $1
                          i32.eq
                          br_if 8 (;@3;)
                          local.get $3
                          local.set $1
                          br 4 (;@7;)
                        end
                      end
                    end
                  end
                  br 1 (;@6;)
                end
                i32.const 0
                local.get $1
                i32.sub
                local.set $5
                local.get $7
                local.get $1
                i32.gt_u
                local.get $1
                i32.const 2147483647
                i32.lt_u
                local.get $2
                i32.const -1
                i32.ne
                i32.and
                i32.and
                if ;; label = @7
                  local.get $13
                  local.get $1
                  i32.sub
                  i32.const 4656
                  i32.load
                  local.tee $3
                  i32.add
                  i32.const 0
                  local.get $3
                  i32.sub
                  i32.and
                  local.tee $3
                  i32.const 2147483647
                  i32.lt_u
                  if ;; label = @8
                    local.get $3
                    call $45
                    i32.const -1
                    i32.eq
                    if ;; label = @9
                      block ;; label = @10
                        local.get $5
                        call $45
                        drop
                        br 4 (;@6;)
                      end
                    else
                      local.get $3
                      local.get $1
                      i32.add
                      local.set $3
                    end
                  else
                    local.get $1
                    local.set $3
                  end
                else
                  local.get $1
                  local.set $3
                end
                local.get $2
                i32.const -1
                i32.ne
                if ;; label = @7
                  block ;; label = @8
                    local.get $2
                    local.set $1
                    br 5 (;@3;)
                  end
                end
              end
              i32.const 4620
              i32.const 4620
              i32.load
              i32.const 4
              i32.or
              i32.store
            end
          end
          local.get $6
          i32.const 2147483647
          i32.lt_u
          if ;; label = @4
            local.get $6
            call $45
            local.tee $1
            i32.const 0
            call $45
            local.tee $3
            i32.lt_u
            local.get $1
            i32.const -1
            i32.ne
            local.get $3
            i32.const -1
            i32.ne
            i32.and
            i32.and
            if ;; label = @5
              local.get $3
              local.get $1
              i32.sub
              local.tee $3
              local.get $0
              i32.const 40
              i32.add
              i32.gt_u
              br_if 2 (;@3;)
            end
          end
          br 1 (;@2;)
        end
        i32.const 4608
        i32.const 4608
        i32.load
        local.get $3
        i32.add
        local.tee $2
        i32.store
        local.get $2
        i32.const 4612
        i32.load
        i32.gt_u
        if ;; label = @3
          i32.const 4612
          local.get $2
          i32.store
        end
        block $label$198 ;; label = @3
          i32.const 4200
          i32.load
          local.tee $8
          if ;; label = @4
            block ;; label = @5
              i32.const 4624
              local.set $2
              block $label$200 ;; label = @6
                block $label$201 ;; label = @7
                  loop $label$202 ;; label = @8
                    local.get $1
                    local.get $2
                    i32.load
                    local.tee $4
                    local.get $2
                    i32.const 4
                    i32.add
                    local.tee $7
                    i32.load
                    local.tee $5
                    i32.add
                    i32.eq
                    br_if 1 (;@7;)
                    local.get $2
                    i32.load offset=8
                    local.tee $2
                    br_if 0 (;@8;)
                  end
                  br 1 (;@6;)
                end
                local.get $2
                i32.load offset=12
                i32.const 8
                i32.and
                i32.eqz
                if ;; label = @7
                  local.get $8
                  local.get $1
                  i32.lt_u
                  local.get $8
                  local.get $4
                  i32.ge_u
                  i32.and
                  if ;; label = @8
                    block ;; label = @9
                      local.get $7
                      local.get $5
                      local.get $3
                      i32.add
                      i32.store
                      i32.const 4188
                      i32.load
                      local.set $5
                      i32.const 0
                      local.get $8
                      i32.const 8
                      i32.add
                      local.tee $2
                      i32.sub
                      i32.const 7
                      i32.and
                      local.set $1
                      i32.const 4200
                      local.get $8
                      local.get $2
                      i32.const 7
                      i32.and
                      if (result i32) ;; label = @10
                        local.get $1
                      else
                        i32.const 0
                        local.tee $1
                      end
                      i32.add
                      local.tee $2
                      i32.store
                      i32.const 4188
                      local.get $3
                      local.get $1
                      i32.sub
                      local.get $5
                      i32.add
                      local.tee $1
                      i32.store
                      local.get $2
                      local.get $1
                      i32.const 1
                      i32.or
                      i32.store offset=4
                      local.get $2
                      local.get $1
                      i32.add
                      i32.const 40
                      i32.store offset=4
                      i32.const 4204
                      i32.const 4664
                      i32.load
                      i32.store
                      br 6 (;@3;)
                    end
                  end
                end
              end
              local.get $1
              i32.const 4192
              i32.load
              local.tee $2
              i32.lt_u
              if ;; label = @6
                block ;; label = @7
                  i32.const 4192
                  local.get $1
                  i32.store
                  local.get $1
                  local.set $2
                end
              end
              local.get $1
              local.get $3
              i32.add
              local.set $10
              i32.const 4624
              local.set $5
              block $label$208 ;; label = @6
                block $label$209 ;; label = @7
                  loop $label$210 ;; label = @8
                    local.get $5
                    i32.load
                    local.get $10
                    i32.eq
                    br_if 1 (;@7;)
                    local.get $5
                    i32.load offset=8
                    local.tee $5
                    br_if 0 (;@8;)
                    i32.const 4624
                    local.set $5
                  end
                  br 1 (;@6;)
                end
                local.get $5
                i32.load offset=12
                i32.const 8
                i32.and
                if ;; label = @7
                  i32.const 4624
                  local.set $5
                else
                  block ;; label = @8
                    local.get $5
                    local.get $1
                    i32.store
                    local.get $5
                    i32.const 4
                    i32.add
                    local.tee $5
                    local.get $5
                    i32.load
                    local.get $3
                    i32.add
                    i32.store
                    i32.const 0
                    local.get $1
                    i32.const 8
                    i32.add
                    local.tee $4
                    i32.sub
                    i32.const 7
                    i32.and
                    local.set $7
                    i32.const 0
                    local.get $10
                    i32.const 8
                    i32.add
                    local.tee $5
                    i32.sub
                    i32.const 7
                    i32.and
                    local.set $3
                    local.get $1
                    local.get $4
                    i32.const 7
                    i32.and
                    if (result i32) ;; label = @9
                      local.get $7
                    else
                      i32.const 0
                    end
                    i32.add
                    local.tee $13
                    local.get $0
                    i32.add
                    local.set $6
                    local.get $10
                    local.get $5
                    i32.const 7
                    i32.and
                    if (result i32) ;; label = @9
                      local.get $3
                    else
                      i32.const 0
                    end
                    i32.add
                    local.tee $4
                    local.get $13
                    i32.sub
                    local.get $0
                    i32.sub
                    local.set $7
                    local.get $13
                    local.get $0
                    i32.const 3
                    i32.or
                    i32.store offset=4
                    block $label$217 ;; label = @9
                      local.get $4
                      local.get $8
                      i32.eq
                      if ;; label = @10
                        block ;; label = @11
                          i32.const 4188
                          i32.const 4188
                          i32.load
                          local.get $7
                          i32.add
                          local.tee $0
                          i32.store
                          i32.const 4200
                          local.get $6
                          i32.store
                          local.get $6
                          local.get $0
                          i32.const 1
                          i32.or
                          i32.store offset=4
                        end
                      else
                        block ;; label = @11
                          local.get $4
                          i32.const 4196
                          i32.load
                          i32.eq
                          if ;; label = @12
                            block ;; label = @13
                              i32.const 4184
                              i32.const 4184
                              i32.load
                              local.get $7
                              i32.add
                              local.tee $0
                              i32.store
                              i32.const 4196
                              local.get $6
                              i32.store
                              local.get $6
                              local.get $0
                              i32.const 1
                              i32.or
                              i32.store offset=4
                              local.get $6
                              local.get $0
                              i32.add
                              local.get $0
                              i32.store
                              br 4 (;@9;)
                            end
                          end
                          local.get $4
                          i32.load offset=4
                          local.tee $0
                          i32.const 3
                          i32.and
                          i32.const 1
                          i32.eq
                          if (result i32) ;; label = @12
                            block (result i32) ;; label = @13
                              local.get $0
                              i32.const -8
                              i32.and
                              local.set $11
                              local.get $0
                              i32.const 3
                              i32.shr_u
                              local.set $1
                              block $label$222 ;; label = @14
                                local.get $0
                                i32.const 256
                                i32.lt_u
                                if ;; label = @15
                                  block ;; label = @16
                                    local.get $4
                                    i32.load offset=12
                                    local.set $5
                                    block $label$224 ;; label = @17
                                      local.get $4
                                      i32.load offset=8
                                      local.tee $3
                                      local.get $1
                                      i32.const 1
                                      i32.shl
                                      i32.const 2
                                      i32.shl
                                      i32.const 4216
                                      i32.add
                                      local.tee $0
                                      i32.ne
                                      if ;; label = @18
                                        block ;; label = @19
                                          local.get $3
                                          local.get $2
                                          i32.lt_u
                                          if ;; label = @20
                                            call $fimport$8
                                          end
                                          local.get $3
                                          i32.load offset=12
                                          local.get $4
                                          i32.eq
                                          br_if 2 (;@17;)
                                          call $fimport$8
                                        end
                                      end
                                    end
                                    local.get $5
                                    local.get $3
                                    i32.eq
                                    if ;; label = @17
                                      block ;; label = @18
                                        i32.const 4176
                                        i32.const 4176
                                        i32.load
                                        i32.const 1
                                        local.get $1
                                        i32.shl
                                        i32.const -1
                                        i32.xor
                                        i32.and
                                        i32.store
                                        br 4 (;@14;)
                                      end
                                    end
                                    block $label$228 ;; label = @17
                                      local.get $5
                                      local.get $0
                                      i32.eq
                                      if ;; label = @18
                                        local.get $5
                                        i32.const 8
                                        i32.add
                                        local.set $20
                                      else
                                        block ;; label = @19
                                          local.get $5
                                          local.get $2
                                          i32.lt_u
                                          if ;; label = @20
                                            call $fimport$8
                                          end
                                          local.get $5
                                          i32.const 8
                                          i32.add
                                          local.tee $0
                                          i32.load
                                          local.get $4
                                          i32.eq
                                          if ;; label = @20
                                            block ;; label = @21
                                              local.get $0
                                              local.set $20
                                              br 4 (;@17;)
                                            end
                                          end
                                          call $fimport$8
                                        end
                                      end
                                    end
                                    local.get $3
                                    local.get $5
                                    i32.store offset=12
                                    local.get $20
                                    local.get $3
                                    i32.store
                                  end
                                else
                                  block ;; label = @16
                                    local.get $4
                                    i32.load offset=24
                                    local.set $8
                                    block $label$234 ;; label = @17
                                      local.get $4
                                      i32.load offset=12
                                      local.tee $0
                                      local.get $4
                                      i32.eq
                                      if ;; label = @18
                                        block ;; label = @19
                                          local.get $4
                                          i32.const 16
                                          i32.add
                                          local.tee $3
                                          i32.const 4
                                          i32.add
                                          local.tee $1
                                          i32.load
                                          local.tee $0
                                          i32.eqz
                                          if ;; label = @20
                                            local.get $3
                                            i32.load
                                            local.tee $0
                                            if ;; label = @21
                                              local.get $3
                                              local.set $1
                                            else
                                              block ;; label = @22
                                                i32.const 0
                                                local.set $12
                                                br 5 (;@17;)
                                              end
                                            end
                                          end
                                          loop $label$239 ;; label = @20
                                            local.get $0
                                            i32.const 20
                                            i32.add
                                            local.tee $5
                                            i32.load
                                            local.tee $3
                                            if ;; label = @21
                                              block ;; label = @22
                                                local.get $3
                                                local.set $0
                                                local.get $5
                                                local.set $1
                                                br 2 (;@20;)
                                              end
                                            end
                                            local.get $0
                                            i32.const 16
                                            i32.add
                                            local.tee $5
                                            i32.load
                                            local.tee $3
                                            if ;; label = @21
                                              block ;; label = @22
                                                local.get $3
                                                local.set $0
                                                local.get $5
                                                local.set $1
                                                br 2 (;@20;)
                                              end
                                            end
                                          end
                                          local.get $1
                                          local.get $2
                                          i32.lt_u
                                          if ;; label = @20
                                            call $fimport$8
                                          else
                                            block ;; label = @21
                                              local.get $1
                                              i32.const 0
                                              i32.store
                                              local.get $0
                                              local.set $12
                                            end
                                          end
                                        end
                                      else
                                        block ;; label = @19
                                          local.get $4
                                          i32.load offset=8
                                          local.tee $5
                                          local.get $2
                                          i32.lt_u
                                          if ;; label = @20
                                            call $fimport$8
                                          end
                                          local.get $5
                                          i32.const 12
                                          i32.add
                                          local.tee $3
                                          i32.load
                                          local.get $4
                                          i32.ne
                                          if ;; label = @20
                                            call $fimport$8
                                          end
                                          local.get $0
                                          i32.const 8
                                          i32.add
                                          local.tee $1
                                          i32.load
                                          local.get $4
                                          i32.eq
                                          if ;; label = @20
                                            block ;; label = @21
                                              local.get $3
                                              local.get $0
                                              i32.store
                                              local.get $1
                                              local.get $5
                                              i32.store
                                              local.get $0
                                              local.set $12
                                            end
                                          else
                                            call $fimport$8
                                          end
                                        end
                                      end
                                    end
                                    local.get $8
                                    i32.eqz
                                    br_if 2 (;@14;)
                                    block $label$249 ;; label = @17
                                      local.get $4
                                      local.get $4
                                      i32.load offset=28
                                      local.tee $1
                                      i32.const 2
                                      i32.shl
                                      i32.const 4480
                                      i32.add
                                      local.tee $0
                                      i32.load
                                      i32.eq
                                      if ;; label = @18
                                        block ;; label = @19
                                          local.get $0
                                          local.get $12
                                          i32.store
                                          local.get $12
                                          br_if 2 (;@17;)
                                          i32.const 4180
                                          i32.const 4180
                                          i32.load
                                          i32.const 1
                                          local.get $1
                                          i32.shl
                                          i32.const -1
                                          i32.xor
                                          i32.and
                                          i32.store
                                          br 5 (;@14;)
                                        end
                                      else
                                        block ;; label = @19
                                          local.get $8
                                          i32.const 4192
                                          i32.load
                                          i32.lt_u
                                          if ;; label = @20
                                            call $fimport$8
                                          end
                                          local.get $8
                                          i32.const 16
                                          i32.add
                                          local.tee $0
                                          i32.load
                                          local.get $4
                                          i32.eq
                                          if ;; label = @20
                                            local.get $0
                                            local.get $12
                                            i32.store
                                          else
                                            local.get $8
                                            local.get $12
                                            i32.store offset=20
                                          end
                                          local.get $12
                                          i32.eqz
                                          br_if 5 (;@14;)
                                        end
                                      end
                                    end
                                    local.get $12
                                    i32.const 4192
                                    i32.load
                                    local.tee $1
                                    i32.lt_u
                                    if ;; label = @17
                                      call $fimport$8
                                    end
                                    local.get $12
                                    local.get $8
                                    i32.store offset=24
                                    local.get $4
                                    i32.const 16
                                    i32.add
                                    local.tee $0
                                    i32.load
                                    local.tee $3
                                    if ;; label = @17
                                      local.get $3
                                      local.get $1
                                      i32.lt_u
                                      if ;; label = @18
                                        call $fimport$8
                                      else
                                        block ;; label = @19
                                          local.get $12
                                          local.get $3
                                          i32.store offset=16
                                          local.get $3
                                          local.get $12
                                          i32.store offset=24
                                        end
                                      end
                                    end
                                    local.get $0
                                    i32.load offset=4
                                    local.tee $0
                                    i32.eqz
                                    br_if 2 (;@14;)
                                    local.get $0
                                    i32.const 4192
                                    i32.load
                                    i32.lt_u
                                    if ;; label = @17
                                      call $fimport$8
                                    else
                                      block ;; label = @18
                                        local.get $12
                                        local.get $0
                                        i32.store offset=20
                                        local.get $0
                                        local.get $12
                                        i32.store offset=24
                                      end
                                    end
                                  end
                                end
                              end
                              local.get $11
                              local.get $7
                              i32.add
                              local.set $7
                              local.get $4
                              local.get $11
                              i32.add
                            end
                          else
                            local.get $4
                          end
                          local.tee $0
                          i32.const 4
                          i32.add
                          local.tee $0
                          local.get $0
                          i32.load
                          i32.const -2
                          i32.and
                          i32.store
                          local.get $6
                          local.get $7
                          i32.const 1
                          i32.or
                          i32.store offset=4
                          local.get $6
                          local.get $7
                          i32.add
                          local.get $7
                          i32.store
                          local.get $7
                          i32.const 3
                          i32.shr_u
                          local.set $0
                          local.get $7
                          i32.const 256
                          i32.lt_u
                          if ;; label = @12
                            block ;; label = @13
                              local.get $0
                              i32.const 1
                              i32.shl
                              i32.const 2
                              i32.shl
                              i32.const 4216
                              i32.add
                              local.set $3
                              block $label$263 ;; label = @14
                                i32.const 4176
                                i32.load
                                local.tee $1
                                i32.const 1
                                local.get $0
                                i32.shl
                                local.tee $0
                                i32.and
                                if ;; label = @15
                                  block ;; label = @16
                                    local.get $3
                                    i32.const 8
                                    i32.add
                                    local.tee $1
                                    i32.load
                                    local.tee $0
                                    i32.const 4192
                                    i32.load
                                    i32.ge_u
                                    if ;; label = @17
                                      block ;; label = @18
                                        local.get $1
                                        local.set $21
                                        local.get $0
                                        local.set $9
                                        br 4 (;@14;)
                                      end
                                    end
                                    call $fimport$8
                                  end
                                else
                                  block ;; label = @16
                                    i32.const 4176
                                    local.get $1
                                    local.get $0
                                    i32.or
                                    i32.store
                                    local.get $3
                                    i32.const 8
                                    i32.add
                                    local.set $21
                                    local.get $3
                                    local.set $9
                                  end
                                end
                              end
                              local.get $21
                              local.get $6
                              i32.store
                              local.get $9
                              local.get $6
                              i32.store offset=12
                              local.get $6
                              local.get $9
                              i32.store offset=8
                              local.get $6
                              local.get $3
                              i32.store offset=12
                              br 4 (;@9;)
                            end
                          end
                          block $label$267 (result i32) ;; label = @12
                            local.get $7
                            i32.const 8
                            i32.shr_u
                            local.tee $0
                            if (result i32) ;; label = @13
                              block (result i32) ;; label = @14
                                i32.const 31
                                local.get $7
                                i32.const 16777215
                                i32.gt_u
                                br_if 2 (;@12;)
                                drop
                                local.get $7
                                i32.const 14
                                local.get $0
                                local.get $0
                                i32.const 1048320
                                i32.add
                                i32.const 16
                                i32.shr_u
                                i32.const 8
                                i32.and
                                local.tee $3
                                i32.shl
                                local.tee $1
                                i32.const 520192
                                i32.add
                                i32.const 16
                                i32.shr_u
                                i32.const 4
                                i32.and
                                local.tee $0
                                local.get $3
                                i32.or
                                local.get $1
                                local.get $0
                                i32.shl
                                local.tee $1
                                i32.const 245760
                                i32.add
                                i32.const 16
                                i32.shr_u
                                i32.const 2
                                i32.and
                                local.tee $0
                                i32.or
                                i32.sub
                                local.get $1
                                local.get $0
                                i32.shl
                                i32.const 15
                                i32.shr_u
                                i32.add
                                local.tee $0
                                i32.const 7
                                i32.add
                                i32.shr_u
                                i32.const 1
                                i32.and
                                local.get $0
                                i32.const 1
                                i32.shl
                                i32.or
                              end
                            else
                              i32.const 0
                            end
                          end
                          local.tee $2
                          i32.const 2
                          i32.shl
                          i32.const 4480
                          i32.add
                          local.set $3
                          local.get $6
                          local.get $2
                          i32.store offset=28
                          local.get $6
                          i32.const 16
                          i32.add
                          local.tee $0
                          i32.const 0
                          i32.store offset=4
                          local.get $0
                          i32.const 0
                          i32.store
                          i32.const 4180
                          i32.load
                          local.tee $1
                          i32.const 1
                          local.get $2
                          i32.shl
                          local.tee $0
                          i32.and
                          i32.eqz
                          if ;; label = @12
                            block ;; label = @13
                              i32.const 4180
                              local.get $1
                              local.get $0
                              i32.or
                              i32.store
                              local.get $3
                              local.get $6
                              i32.store
                              local.get $6
                              local.get $3
                              i32.store offset=24
                              local.get $6
                              local.get $6
                              i32.store offset=12
                              local.get $6
                              local.get $6
                              i32.store offset=8
                              br 4 (;@9;)
                            end
                          end
                          local.get $3
                          i32.load
                          local.set $0
                          i32.const 25
                          local.get $2
                          i32.const 1
                          i32.shr_u
                          i32.sub
                          local.set $1
                          local.get $7
                          local.get $2
                          i32.const 31
                          i32.eq
                          if (result i32) ;; label = @12
                            i32.const 0
                          else
                            local.get $1
                          end
                          i32.shl
                          local.set $2
                          block $label$273 ;; label = @12
                            block $label$274 ;; label = @13
                              block $label$275 ;; label = @14
                                loop $label$276 ;; label = @15
                                  local.get $0
                                  i32.load offset=4
                                  i32.const -8
                                  i32.and
                                  local.get $7
                                  i32.eq
                                  br_if 2 (;@13;)
                                  local.get $2
                                  i32.const 1
                                  i32.shl
                                  local.set $3
                                  local.get $0
                                  i32.const 16
                                  i32.add
                                  local.get $2
                                  i32.const 31
                                  i32.shr_u
                                  i32.const 2
                                  i32.shl
                                  i32.add
                                  local.tee $2
                                  i32.load
                                  local.tee $1
                                  i32.eqz
                                  br_if 1 (;@14;)
                                  local.get $3
                                  local.set $2
                                  local.get $1
                                  local.set $0
                                  br 0 (;@15;)
                                end
                              end
                              local.get $2
                              i32.const 4192
                              i32.load
                              i32.lt_u
                              if ;; label = @14
                                call $fimport$8
                              else
                                block ;; label = @15
                                  local.get $2
                                  local.get $6
                                  i32.store
                                  local.get $6
                                  local.get $0
                                  i32.store offset=24
                                  local.get $6
                                  local.get $6
                                  i32.store offset=12
                                  local.get $6
                                  local.get $6
                                  i32.store offset=8
                                  br 6 (;@9;)
                                end
                              end
                              br 1 (;@12;)
                            end
                            local.get $0
                            i32.const 8
                            i32.add
                            local.tee $3
                            i32.load
                            local.tee $2
                            i32.const 4192
                            i32.load
                            local.tee $1
                            i32.ge_u
                            local.get $0
                            local.get $1
                            i32.ge_u
                            i32.and
                            if ;; label = @13
                              block ;; label = @14
                                local.get $2
                                local.get $6
                                i32.store offset=12
                                local.get $3
                                local.get $6
                                i32.store
                                local.get $6
                                local.get $2
                                i32.store offset=8
                                local.get $6
                                local.get $0
                                i32.store offset=12
                                local.get $6
                                i32.const 0
                                i32.store offset=24
                              end
                            else
                              call $fimport$8
                            end
                          end
                        end
                      end
                    end
                    local.get $14
                    global.set $global$1
                    local.get $13
                    i32.const 8
                    i32.add
                    return
                  end
                end
              end
              loop $label$281 ;; label = @6
                block $label$282 ;; label = @7
                  local.get $5
                  i32.load
                  local.tee $2
                  local.get $8
                  i32.le_u
                  if ;; label = @8
                    local.get $2
                    local.get $5
                    i32.load offset=4
                    i32.add
                    local.tee $13
                    local.get $8
                    i32.gt_u
                    br_if 1 (;@7;)
                  end
                  local.get $5
                  i32.load offset=8
                  local.set $5
                  br 1 (;@6;)
                end
              end
              i32.const 0
              local.get $13
              i32.const -47
              i32.add
              local.tee $7
              i32.const 8
              i32.add
              local.tee $5
              i32.sub
              i32.const 7
              i32.and
              local.set $2
              local.get $7
              local.get $5
              i32.const 7
              i32.and
              if (result i32) ;; label = @6
                local.get $2
              else
                i32.const 0
              end
              i32.add
              local.tee $2
              local.get $8
              i32.const 16
              i32.add
              local.tee $12
              i32.lt_u
              if (result i32) ;; label = @6
                local.get $8
              else
                local.get $2
              end
              local.tee $7
              i32.const 8
              i32.add
              local.set $10
              local.get $7
              i32.const 24
              i32.add
              local.set $5
              local.get $3
              i32.const -40
              i32.add
              local.set $9
              i32.const 0
              local.get $1
              i32.const 8
              i32.add
              local.tee $4
              i32.sub
              i32.const 7
              i32.and
              local.set $2
              i32.const 4200
              local.get $1
              local.get $4
              i32.const 7
              i32.and
              if (result i32) ;; label = @6
                local.get $2
              else
                i32.const 0
                local.tee $2
              end
              i32.add
              local.tee $4
              i32.store
              i32.const 4188
              local.get $9
              local.get $2
              i32.sub
              local.tee $2
              i32.store
              local.get $4
              local.get $2
              i32.const 1
              i32.or
              i32.store offset=4
              local.get $4
              local.get $2
              i32.add
              i32.const 40
              i32.store offset=4
              i32.const 4204
              i32.const 4664
              i32.load
              i32.store
              local.get $7
              i32.const 4
              i32.add
              local.tee $2
              i32.const 27
              i32.store
              local.get $10
              i32.const 4624
              i64.load align=4
              i64.store align=4
              local.get $10
              i32.const 4632
              i64.load align=4
              i64.store offset=8 align=4
              i32.const 4624
              local.get $1
              i32.store
              i32.const 4628
              local.get $3
              i32.store
              i32.const 4636
              i32.const 0
              i32.store
              i32.const 4632
              local.get $10
              i32.store
              local.get $5
              local.set $1
              loop $label$290 ;; label = @6
                local.get $1
                i32.const 4
                i32.add
                local.tee $1
                i32.const 7
                i32.store
                local.get $1
                i32.const 4
                i32.add
                local.get $13
                i32.lt_u
                br_if 0 (;@6;)
              end
              local.get $7
              local.get $8
              i32.ne
              if ;; label = @6
                block ;; label = @7
                  local.get $2
                  local.get $2
                  i32.load
                  i32.const -2
                  i32.and
                  i32.store
                  local.get $8
                  local.get $7
                  local.get $8
                  i32.sub
                  local.tee $4
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  local.get $7
                  local.get $4
                  i32.store
                  local.get $4
                  i32.const 3
                  i32.shr_u
                  local.set $1
                  local.get $4
                  i32.const 256
                  i32.lt_u
                  if ;; label = @8
                    block ;; label = @9
                      local.get $1
                      i32.const 1
                      i32.shl
                      i32.const 2
                      i32.shl
                      i32.const 4216
                      i32.add
                      local.set $2
                      i32.const 4176
                      i32.load
                      local.tee $3
                      i32.const 1
                      local.get $1
                      i32.shl
                      local.tee $1
                      i32.and
                      if ;; label = @10
                        local.get $2
                        i32.const 8
                        i32.add
                        local.tee $3
                        i32.load
                        local.tee $1
                        i32.const 4192
                        i32.load
                        i32.lt_u
                        if ;; label = @11
                          call $fimport$8
                        else
                          block ;; label = @12
                            local.get $3
                            local.set $15
                            local.get $1
                            local.set $11
                          end
                        end
                      else
                        block ;; label = @11
                          i32.const 4176
                          local.get $3
                          local.get $1
                          i32.or
                          i32.store
                          local.get $2
                          i32.const 8
                          i32.add
                          local.set $15
                          local.get $2
                          local.set $11
                        end
                      end
                      local.get $15
                      local.get $8
                      i32.store
                      local.get $11
                      local.get $8
                      i32.store offset=12
                      local.get $8
                      local.get $11
                      i32.store offset=8
                      local.get $8
                      local.get $2
                      i32.store offset=12
                      br 6 (;@3;)
                    end
                  end
                  local.get $4
                  i32.const 8
                  i32.shr_u
                  local.tee $1
                  if (result i32) ;; label = @8
                    local.get $4
                    i32.const 16777215
                    i32.gt_u
                    if (result i32) ;; label = @9
                      i32.const 31
                    else
                      local.get $4
                      i32.const 14
                      local.get $1
                      local.get $1
                      i32.const 1048320
                      i32.add
                      i32.const 16
                      i32.shr_u
                      i32.const 8
                      i32.and
                      local.tee $2
                      i32.shl
                      local.tee $3
                      i32.const 520192
                      i32.add
                      i32.const 16
                      i32.shr_u
                      i32.const 4
                      i32.and
                      local.tee $1
                      local.get $2
                      i32.or
                      local.get $3
                      local.get $1
                      i32.shl
                      local.tee $3
                      i32.const 245760
                      i32.add
                      i32.const 16
                      i32.shr_u
                      i32.const 2
                      i32.and
                      local.tee $1
                      i32.or
                      i32.sub
                      local.get $3
                      local.get $1
                      i32.shl
                      i32.const 15
                      i32.shr_u
                      i32.add
                      local.tee $1
                      i32.const 7
                      i32.add
                      i32.shr_u
                      i32.const 1
                      i32.and
                      local.get $1
                      i32.const 1
                      i32.shl
                      i32.or
                    end
                  else
                    i32.const 0
                  end
                  local.tee $5
                  i32.const 2
                  i32.shl
                  i32.const 4480
                  i32.add
                  local.set $2
                  local.get $8
                  local.get $5
                  i32.store offset=28
                  local.get $8
                  i32.const 0
                  i32.store offset=20
                  local.get $12
                  i32.const 0
                  i32.store
                  i32.const 4180
                  i32.load
                  local.tee $3
                  i32.const 1
                  local.get $5
                  i32.shl
                  local.tee $1
                  i32.and
                  i32.eqz
                  if ;; label = @8
                    block ;; label = @9
                      i32.const 4180
                      local.get $3
                      local.get $1
                      i32.or
                      i32.store
                      local.get $2
                      local.get $8
                      i32.store
                      local.get $8
                      local.get $2
                      i32.store offset=24
                      local.get $8
                      local.get $8
                      i32.store offset=12
                      local.get $8
                      local.get $8
                      i32.store offset=8
                      br 6 (;@3;)
                    end
                  end
                  local.get $2
                  i32.load
                  local.set $1
                  i32.const 25
                  local.get $5
                  i32.const 1
                  i32.shr_u
                  i32.sub
                  local.set $3
                  local.get $4
                  local.get $5
                  i32.const 31
                  i32.eq
                  if (result i32) ;; label = @8
                    i32.const 0
                  else
                    local.get $3
                  end
                  i32.shl
                  local.set $5
                  block $label$304 ;; label = @8
                    block $label$305 ;; label = @9
                      block $label$306 ;; label = @10
                        loop $label$307 ;; label = @11
                          local.get $1
                          i32.load offset=4
                          i32.const -8
                          i32.and
                          local.get $4
                          i32.eq
                          br_if 2 (;@9;)
                          local.get $5
                          i32.const 1
                          i32.shl
                          local.set $2
                          local.get $1
                          i32.const 16
                          i32.add
                          local.get $5
                          i32.const 31
                          i32.shr_u
                          i32.const 2
                          i32.shl
                          i32.add
                          local.tee $5
                          i32.load
                          local.tee $3
                          i32.eqz
                          br_if 1 (;@10;)
                          local.get $2
                          local.set $5
                          local.get $3
                          local.set $1
                          br 0 (;@11;)
                        end
                      end
                      local.get $5
                      i32.const 4192
                      i32.load
                      i32.lt_u
                      if ;; label = @10
                        call $fimport$8
                      else
                        block ;; label = @11
                          local.get $5
                          local.get $8
                          i32.store
                          local.get $8
                          local.get $1
                          i32.store offset=24
                          local.get $8
                          local.get $8
                          i32.store offset=12
                          local.get $8
                          local.get $8
                          i32.store offset=8
                          br 8 (;@3;)
                        end
                      end
                      br 1 (;@8;)
                    end
                    local.get $1
                    i32.const 8
                    i32.add
                    local.tee $2
                    i32.load
                    local.tee $5
                    i32.const 4192
                    i32.load
                    local.tee $3
                    i32.ge_u
                    local.get $1
                    local.get $3
                    i32.ge_u
                    i32.and
                    if ;; label = @9
                      block ;; label = @10
                        local.get $5
                        local.get $8
                        i32.store offset=12
                        local.get $2
                        local.get $8
                        i32.store
                        local.get $8
                        local.get $5
                        i32.store offset=8
                        local.get $8
                        local.get $1
                        i32.store offset=12
                        local.get $8
                        i32.const 0
                        i32.store offset=24
                      end
                    else
                      call $fimport$8
                    end
                  end
                end
              end
            end
          else
            block ;; label = @5
              i32.const 4192
              i32.load
              local.tee $2
              i32.eqz
              local.get $1
              local.get $2
              i32.lt_u
              i32.or
              if ;; label = @6
                i32.const 4192
                local.get $1
                i32.store
              end
              i32.const 4624
              local.get $1
              i32.store
              i32.const 4628
              local.get $3
              i32.store
              i32.const 4636
              i32.const 0
              i32.store
              i32.const 4212
              i32.const 4648
              i32.load
              i32.store
              i32.const 4208
              i32.const -1
              i32.store
              i32.const 0
              local.set $2
              loop $label$314 ;; label = @6
                local.get $2
                i32.const 1
                i32.shl
                i32.const 2
                i32.shl
                i32.const 4216
                i32.add
                local.tee $5
                local.get $5
                i32.store offset=12
                local.get $5
                local.get $5
                i32.store offset=8
                local.get $2
                i32.const 1
                i32.add
                local.tee $2
                i32.const 32
                i32.ne
                br_if 0 (;@6;)
              end
              local.get $3
              i32.const -40
              i32.add
              local.set $5
              i32.const 0
              local.get $1
              i32.const 8
              i32.add
              local.tee $2
              i32.sub
              i32.const 7
              i32.and
              local.set $3
              i32.const 4200
              local.get $1
              local.get $2
              i32.const 7
              i32.and
              if (result i32) ;; label = @6
                local.get $3
              else
                i32.const 0
              end
              local.tee $1
              i32.add
              local.tee $3
              i32.store
              i32.const 4188
              local.get $5
              local.get $1
              i32.sub
              local.tee $1
              i32.store
              local.get $3
              local.get $1
              i32.const 1
              i32.or
              i32.store offset=4
              local.get $3
              local.get $1
              i32.add
              i32.const 40
              i32.store offset=4
              i32.const 4204
              i32.const 4664
              i32.load
              i32.store
            end
          end
        end
        i32.const 4188
        i32.load
        local.tee $1
        local.get $0
        i32.gt_u
        if ;; label = @3
          block ;; label = @4
            i32.const 4188
            local.get $1
            local.get $0
            i32.sub
            local.tee $3
            i32.store
            i32.const 4200
            i32.const 4200
            i32.load
            local.tee $2
            local.get $0
            i32.add
            local.tee $1
            i32.store
            local.get $1
            local.get $3
            i32.const 1
            i32.or
            i32.store offset=4
            local.get $2
            local.get $0
            i32.const 3
            i32.or
            i32.store offset=4
            local.get $14
            global.set $global$1
            local.get $2
            i32.const 8
            i32.add
            return
          end
        end
      end
      call $12
      i32.const 12
      i32.store
      local.get $14
      global.set $global$1
      i32.const 0
    end
  )
  (func $38 (;51;) (type $3) (param $0 i32)
    (local $1 i32) (local $2 i32) (local $3 i32) (local $4 i32) (local $5 i32) (local $6 i32) (local $7 i32) (local $8 i32) (local $9 i32) (local $10 i32) (local $11 i32) (local $12 i32) (local $13 i32) (local $14 i32) (local $15 i32)
    block $label$1 ;; label = @1
      local.get $0
      i32.eqz
      if ;; label = @2
        return
      end
      local.get $0
      i32.const -8
      i32.add
      local.tee $1
      i32.const 4192
      i32.load
      local.tee $11
      i32.lt_u
      if ;; label = @2
        call $fimport$8
      end
      local.get $0
      i32.const -4
      i32.add
      i32.load
      local.tee $0
      i32.const 3
      i32.and
      local.tee $8
      i32.const 1
      i32.eq
      if ;; label = @2
        call $fimport$8
      end
      local.get $1
      local.get $0
      i32.const -8
      i32.and
      local.tee $4
      i32.add
      local.set $6
      block $label$5 ;; label = @2
        local.get $0
        i32.const 1
        i32.and
        if ;; label = @3
          block ;; label = @4
            local.get $1
            local.set $3
            local.get $4
            local.set $2
          end
        else
          block ;; label = @4
            local.get $8
            i32.eqz
            if ;; label = @5
              return
            end
            local.get $1
            i32.const 0
            local.get $1
            i32.load
            local.tee $8
            i32.sub
            i32.add
            local.tee $0
            local.get $11
            i32.lt_u
            if ;; label = @5
              call $fimport$8
            end
            local.get $8
            local.get $4
            i32.add
            local.set $1
            local.get $0
            i32.const 4196
            i32.load
            i32.eq
            if ;; label = @5
              block ;; label = @6
                local.get $6
                i32.const 4
                i32.add
                local.tee $2
                i32.load
                local.tee $3
                i32.const 3
                i32.and
                i32.const 3
                i32.ne
                if ;; label = @7
                  block ;; label = @8
                    local.get $0
                    local.set $3
                    local.get $1
                    local.set $2
                    br 6 (;@2;)
                  end
                end
                i32.const 4184
                local.get $1
                i32.store
                local.get $2
                local.get $3
                i32.const -2
                i32.and
                i32.store
                local.get $0
                local.get $1
                i32.const 1
                i32.or
                i32.store offset=4
                local.get $0
                local.get $1
                i32.add
                local.get $1
                i32.store
                return
              end
            end
            local.get $8
            i32.const 3
            i32.shr_u
            local.set $10
            local.get $8
            i32.const 256
            i32.lt_u
            if ;; label = @5
              block ;; label = @6
                local.get $0
                i32.load offset=12
                local.set $3
                local.get $0
                i32.load offset=8
                local.tee $4
                local.get $10
                i32.const 1
                i32.shl
                i32.const 2
                i32.shl
                i32.const 4216
                i32.add
                local.tee $2
                i32.ne
                if ;; label = @7
                  block ;; label = @8
                    local.get $4
                    local.get $11
                    i32.lt_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $4
                    i32.load offset=12
                    local.get $0
                    i32.ne
                    if ;; label = @9
                      call $fimport$8
                    end
                  end
                end
                local.get $3
                local.get $4
                i32.eq
                if ;; label = @7
                  block ;; label = @8
                    i32.const 4176
                    i32.const 4176
                    i32.load
                    i32.const 1
                    local.get $10
                    i32.shl
                    i32.const -1
                    i32.xor
                    i32.and
                    i32.store
                    local.get $0
                    local.set $3
                    local.get $1
                    local.set $2
                    br 6 (;@2;)
                  end
                end
                local.get $3
                local.get $2
                i32.eq
                if ;; label = @7
                  local.get $3
                  i32.const 8
                  i32.add
                  local.set $5
                else
                  block ;; label = @8
                    local.get $3
                    local.get $11
                    i32.lt_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $3
                    i32.const 8
                    i32.add
                    local.tee $2
                    i32.load
                    local.get $0
                    i32.eq
                    if ;; label = @9
                      local.get $2
                      local.set $5
                    else
                      call $fimport$8
                    end
                  end
                end
                local.get $4
                local.get $3
                i32.store offset=12
                local.get $5
                local.get $4
                i32.store
                local.get $0
                local.set $3
                local.get $1
                local.set $2
                br 4 (;@2;)
              end
            end
            local.get $0
            i32.load offset=24
            local.set $12
            block $label$22 ;; label = @5
              local.get $0
              i32.load offset=12
              local.tee $4
              local.get $0
              i32.eq
              if ;; label = @6
                block ;; label = @7
                  local.get $0
                  i32.const 16
                  i32.add
                  local.tee $5
                  i32.const 4
                  i32.add
                  local.tee $8
                  i32.load
                  local.tee $4
                  if ;; label = @8
                    local.get $8
                    local.set $5
                  else
                    local.get $5
                    i32.load
                    local.tee $4
                    i32.eqz
                    if ;; label = @9
                      block ;; label = @10
                        i32.const 0
                        local.set $7
                        br 5 (;@5;)
                      end
                    end
                  end
                  loop $label$27 ;; label = @8
                    local.get $4
                    i32.const 20
                    i32.add
                    local.tee $8
                    i32.load
                    local.tee $10
                    if ;; label = @9
                      block ;; label = @10
                        local.get $10
                        local.set $4
                        local.get $8
                        local.set $5
                        br 2 (;@8;)
                      end
                    end
                    local.get $4
                    i32.const 16
                    i32.add
                    local.tee $8
                    i32.load
                    local.tee $10
                    if ;; label = @9
                      block ;; label = @10
                        local.get $10
                        local.set $4
                        local.get $8
                        local.set $5
                        br 2 (;@8;)
                      end
                    end
                  end
                  local.get $5
                  local.get $11
                  i32.lt_u
                  if ;; label = @8
                    call $fimport$8
                  else
                    block ;; label = @9
                      local.get $5
                      i32.const 0
                      i32.store
                      local.get $4
                      local.set $7
                    end
                  end
                end
              else
                block ;; label = @7
                  local.get $0
                  i32.load offset=8
                  local.tee $5
                  local.get $11
                  i32.lt_u
                  if ;; label = @8
                    call $fimport$8
                  end
                  local.get $5
                  i32.const 12
                  i32.add
                  local.tee $8
                  i32.load
                  local.get $0
                  i32.ne
                  if ;; label = @8
                    call $fimport$8
                  end
                  local.get $4
                  i32.const 8
                  i32.add
                  local.tee $10
                  i32.load
                  local.get $0
                  i32.eq
                  if ;; label = @8
                    block ;; label = @9
                      local.get $8
                      local.get $4
                      i32.store
                      local.get $10
                      local.get $5
                      i32.store
                      local.get $4
                      local.set $7
                    end
                  else
                    call $fimport$8
                  end
                end
              end
            end
            local.get $12
            if ;; label = @5
              block ;; label = @6
                local.get $0
                local.get $0
                i32.load offset=28
                local.tee $4
                i32.const 2
                i32.shl
                i32.const 4480
                i32.add
                local.tee $5
                i32.load
                i32.eq
                if ;; label = @7
                  block ;; label = @8
                    local.get $5
                    local.get $7
                    i32.store
                    local.get $7
                    i32.eqz
                    if ;; label = @9
                      block ;; label = @10
                        i32.const 4180
                        i32.const 4180
                        i32.load
                        i32.const 1
                        local.get $4
                        i32.shl
                        i32.const -1
                        i32.xor
                        i32.and
                        i32.store
                        local.get $0
                        local.set $3
                        local.get $1
                        local.set $2
                        br 8 (;@2;)
                      end
                    end
                  end
                else
                  block ;; label = @8
                    local.get $12
                    i32.const 4192
                    i32.load
                    i32.lt_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $12
                    i32.const 16
                    i32.add
                    local.tee $4
                    i32.load
                    local.get $0
                    i32.eq
                    if ;; label = @9
                      local.get $4
                      local.get $7
                      i32.store
                    else
                      local.get $12
                      local.get $7
                      i32.store offset=20
                    end
                    local.get $7
                    i32.eqz
                    if ;; label = @9
                      block ;; label = @10
                        local.get $0
                        local.set $3
                        local.get $1
                        local.set $2
                        br 8 (;@2;)
                      end
                    end
                  end
                end
                local.get $7
                i32.const 4192
                i32.load
                local.tee $5
                i32.lt_u
                if ;; label = @7
                  call $fimport$8
                end
                local.get $7
                local.get $12
                i32.store offset=24
                local.get $0
                i32.const 16
                i32.add
                local.tee $8
                i32.load
                local.tee $4
                if ;; label = @7
                  local.get $4
                  local.get $5
                  i32.lt_u
                  if ;; label = @8
                    call $fimport$8
                  else
                    block ;; label = @9
                      local.get $7
                      local.get $4
                      i32.store offset=16
                      local.get $4
                      local.get $7
                      i32.store offset=24
                    end
                  end
                end
                local.get $8
                i32.load offset=4
                local.tee $4
                if ;; label = @7
                  local.get $4
                  i32.const 4192
                  i32.load
                  i32.lt_u
                  if ;; label = @8
                    call $fimport$8
                  else
                    block ;; label = @9
                      local.get $7
                      local.get $4
                      i32.store offset=20
                      local.get $4
                      local.get $7
                      i32.store offset=24
                      local.get $0
                      local.set $3
                      local.get $1
                      local.set $2
                    end
                  end
                else
                  block ;; label = @8
                    local.get $0
                    local.set $3
                    local.get $1
                    local.set $2
                  end
                end
              end
            else
              block ;; label = @6
                local.get $0
                local.set $3
                local.get $1
                local.set $2
              end
            end
          end
        end
      end
      local.get $3
      local.get $6
      i32.ge_u
      if ;; label = @2
        call $fimport$8
      end
      local.get $6
      i32.const 4
      i32.add
      local.tee $1
      i32.load
      local.tee $0
      i32.const 1
      i32.and
      i32.eqz
      if ;; label = @2
        call $fimport$8
      end
      local.get $0
      i32.const 2
      i32.and
      if ;; label = @2
        block ;; label = @3
          local.get $1
          local.get $0
          i32.const -2
          i32.and
          i32.store
          local.get $3
          local.get $2
          i32.const 1
          i32.or
          i32.store offset=4
          local.get $3
          local.get $2
          i32.add
          local.get $2
          i32.store
        end
      else
        block ;; label = @3
          local.get $6
          i32.const 4200
          i32.load
          i32.eq
          if ;; label = @4
            block ;; label = @5
              i32.const 4188
              i32.const 4188
              i32.load
              local.get $2
              i32.add
              local.tee $0
              i32.store
              i32.const 4200
              local.get $3
              i32.store
              local.get $3
              local.get $0
              i32.const 1
              i32.or
              i32.store offset=4
              local.get $3
              i32.const 4196
              i32.load
              i32.ne
              if ;; label = @6
                return
              end
              i32.const 4196
              i32.const 0
              i32.store
              i32.const 4184
              i32.const 0
              i32.store
              return
            end
          end
          local.get $6
          i32.const 4196
          i32.load
          i32.eq
          if ;; label = @4
            block ;; label = @5
              i32.const 4184
              i32.const 4184
              i32.load
              local.get $2
              i32.add
              local.tee $0
              i32.store
              i32.const 4196
              local.get $3
              i32.store
              local.get $3
              local.get $0
              i32.const 1
              i32.or
              i32.store offset=4
              local.get $3
              local.get $0
              i32.add
              local.get $0
              i32.store
              return
            end
          end
          local.get $0
          i32.const -8
          i32.and
          local.get $2
          i32.add
          local.set $5
          local.get $0
          i32.const 3
          i32.shr_u
          local.set $4
          block $label$61 ;; label = @4
            local.get $0
            i32.const 256
            i32.lt_u
            if ;; label = @5
              block ;; label = @6
                local.get $6
                i32.load offset=12
                local.set $2
                local.get $6
                i32.load offset=8
                local.tee $1
                local.get $4
                i32.const 1
                i32.shl
                i32.const 2
                i32.shl
                i32.const 4216
                i32.add
                local.tee $0
                i32.ne
                if ;; label = @7
                  block ;; label = @8
                    local.get $1
                    i32.const 4192
                    i32.load
                    i32.lt_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $1
                    i32.load offset=12
                    local.get $6
                    i32.ne
                    if ;; label = @9
                      call $fimport$8
                    end
                  end
                end
                local.get $2
                local.get $1
                i32.eq
                if ;; label = @7
                  block ;; label = @8
                    i32.const 4176
                    i32.const 4176
                    i32.load
                    i32.const 1
                    local.get $4
                    i32.shl
                    i32.const -1
                    i32.xor
                    i32.and
                    i32.store
                    br 4 (;@4;)
                  end
                end
                local.get $2
                local.get $0
                i32.eq
                if ;; label = @7
                  local.get $2
                  i32.const 8
                  i32.add
                  local.set $14
                else
                  block ;; label = @8
                    local.get $2
                    i32.const 4192
                    i32.load
                    i32.lt_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $2
                    i32.const 8
                    i32.add
                    local.tee $0
                    i32.load
                    local.get $6
                    i32.eq
                    if ;; label = @9
                      local.get $0
                      local.set $14
                    else
                      call $fimport$8
                    end
                  end
                end
                local.get $1
                local.get $2
                i32.store offset=12
                local.get $14
                local.get $1
                i32.store
              end
            else
              block ;; label = @6
                local.get $6
                i32.load offset=24
                local.set $7
                block $label$73 ;; label = @7
                  local.get $6
                  i32.load offset=12
                  local.tee $0
                  local.get $6
                  i32.eq
                  if ;; label = @8
                    block ;; label = @9
                      local.get $6
                      i32.const 16
                      i32.add
                      local.tee $2
                      i32.const 4
                      i32.add
                      local.tee $1
                      i32.load
                      local.tee $0
                      if ;; label = @10
                        local.get $1
                        local.set $2
                      else
                        local.get $2
                        i32.load
                        local.tee $0
                        i32.eqz
                        if ;; label = @11
                          block ;; label = @12
                            i32.const 0
                            local.set $9
                            br 5 (;@7;)
                          end
                        end
                      end
                      loop $label$78 ;; label = @10
                        local.get $0
                        i32.const 20
                        i32.add
                        local.tee $1
                        i32.load
                        local.tee $4
                        if ;; label = @11
                          block ;; label = @12
                            local.get $4
                            local.set $0
                            local.get $1
                            local.set $2
                            br 2 (;@10;)
                          end
                        end
                        local.get $0
                        i32.const 16
                        i32.add
                        local.tee $1
                        i32.load
                        local.tee $4
                        if ;; label = @11
                          block ;; label = @12
                            local.get $4
                            local.set $0
                            local.get $1
                            local.set $2
                            br 2 (;@10;)
                          end
                        end
                      end
                      local.get $2
                      i32.const 4192
                      i32.load
                      i32.lt_u
                      if ;; label = @10
                        call $fimport$8
                      else
                        block ;; label = @11
                          local.get $2
                          i32.const 0
                          i32.store
                          local.get $0
                          local.set $9
                        end
                      end
                    end
                  else
                    block ;; label = @9
                      local.get $6
                      i32.load offset=8
                      local.tee $2
                      i32.const 4192
                      i32.load
                      i32.lt_u
                      if ;; label = @10
                        call $fimport$8
                      end
                      local.get $2
                      i32.const 12
                      i32.add
                      local.tee $1
                      i32.load
                      local.get $6
                      i32.ne
                      if ;; label = @10
                        call $fimport$8
                      end
                      local.get $0
                      i32.const 8
                      i32.add
                      local.tee $4
                      i32.load
                      local.get $6
                      i32.eq
                      if ;; label = @10
                        block ;; label = @11
                          local.get $1
                          local.get $0
                          i32.store
                          local.get $4
                          local.get $2
                          i32.store
                          local.get $0
                          local.set $9
                        end
                      else
                        call $fimport$8
                      end
                    end
                  end
                end
                local.get $7
                if ;; label = @7
                  block ;; label = @8
                    local.get $6
                    local.get $6
                    i32.load offset=28
                    local.tee $0
                    i32.const 2
                    i32.shl
                    i32.const 4480
                    i32.add
                    local.tee $2
                    i32.load
                    i32.eq
                    if ;; label = @9
                      block ;; label = @10
                        local.get $2
                        local.get $9
                        i32.store
                        local.get $9
                        i32.eqz
                        if ;; label = @11
                          block ;; label = @12
                            i32.const 4180
                            i32.const 4180
                            i32.load
                            i32.const 1
                            local.get $0
                            i32.shl
                            i32.const -1
                            i32.xor
                            i32.and
                            i32.store
                            br 8 (;@4;)
                          end
                        end
                      end
                    else
                      block ;; label = @10
                        local.get $7
                        i32.const 4192
                        i32.load
                        i32.lt_u
                        if ;; label = @11
                          call $fimport$8
                        end
                        local.get $7
                        i32.const 16
                        i32.add
                        local.tee $0
                        i32.load
                        local.get $6
                        i32.eq
                        if ;; label = @11
                          local.get $0
                          local.get $9
                          i32.store
                        else
                          local.get $7
                          local.get $9
                          i32.store offset=20
                        end
                        local.get $9
                        i32.eqz
                        br_if 6 (;@4;)
                      end
                    end
                    local.get $9
                    i32.const 4192
                    i32.load
                    local.tee $2
                    i32.lt_u
                    if ;; label = @9
                      call $fimport$8
                    end
                    local.get $9
                    local.get $7
                    i32.store offset=24
                    local.get $6
                    i32.const 16
                    i32.add
                    local.tee $1
                    i32.load
                    local.tee $0
                    if ;; label = @9
                      local.get $0
                      local.get $2
                      i32.lt_u
                      if ;; label = @10
                        call $fimport$8
                      else
                        block ;; label = @11
                          local.get $9
                          local.get $0
                          i32.store offset=16
                          local.get $0
                          local.get $9
                          i32.store offset=24
                        end
                      end
                    end
                    local.get $1
                    i32.load offset=4
                    local.tee $0
                    if ;; label = @9
                      local.get $0
                      i32.const 4192
                      i32.load
                      i32.lt_u
                      if ;; label = @10
                        call $fimport$8
                      else
                        block ;; label = @11
                          local.get $9
                          local.get $0
                          i32.store offset=20
                          local.get $0
                          local.get $9
                          i32.store offset=24
                        end
                      end
                    end
                  end
                end
              end
            end
          end
          local.get $3
          local.get $5
          i32.const 1
          i32.or
          i32.store offset=4
          local.get $3
          local.get $5
          i32.add
          local.get $5
          i32.store
          local.get $3
          i32.const 4196
          i32.load
          i32.eq
          if ;; label = @4
            block ;; label = @5
              i32.const 4184
              local.get $5
              i32.store
              return
            end
          else
            local.get $5
            local.set $2
          end
        end
      end
      local.get $2
      i32.const 3
      i32.shr_u
      local.set $1
      local.get $2
      i32.const 256
      i32.lt_u
      if ;; label = @2
        block ;; label = @3
          local.get $1
          i32.const 1
          i32.shl
          i32.const 2
          i32.shl
          i32.const 4216
          i32.add
          local.set $0
          i32.const 4176
          i32.load
          local.tee $2
          i32.const 1
          local.get $1
          i32.shl
          local.tee $1
          i32.and
          if ;; label = @4
            local.get $0
            i32.const 8
            i32.add
            local.tee $2
            i32.load
            local.tee $1
            i32.const 4192
            i32.load
            i32.lt_u
            if ;; label = @5
              call $fimport$8
            else
              block ;; label = @6
                local.get $2
                local.set $15
                local.get $1
                local.set $13
              end
            end
          else
            block ;; label = @5
              i32.const 4176
              local.get $2
              local.get $1
              i32.or
              i32.store
              local.get $0
              i32.const 8
              i32.add
              local.set $15
              local.get $0
              local.set $13
            end
          end
          local.get $15
          local.get $3
          i32.store
          local.get $13
          local.get $3
          i32.store offset=12
          local.get $3
          local.get $13
          i32.store offset=8
          local.get $3
          local.get $0
          i32.store offset=12
          return
        end
      end
      local.get $2
      i32.const 8
      i32.shr_u
      local.tee $0
      if (result i32) ;; label = @2
        local.get $2
        i32.const 16777215
        i32.gt_u
        if (result i32) ;; label = @3
          i32.const 31
        else
          local.get $2
          i32.const 14
          local.get $0
          local.get $0
          i32.const 1048320
          i32.add
          i32.const 16
          i32.shr_u
          i32.const 8
          i32.and
          local.tee $0
          i32.shl
          local.tee $1
          i32.const 520192
          i32.add
          i32.const 16
          i32.shr_u
          i32.const 4
          i32.and
          local.tee $4
          local.get $0
          i32.or
          local.get $1
          local.get $4
          i32.shl
          local.tee $0
          i32.const 245760
          i32.add
          i32.const 16
          i32.shr_u
          i32.const 2
          i32.and
          local.tee $1
          i32.or
          i32.sub
          local.get $0
          local.get $1
          i32.shl
          i32.const 15
          i32.shr_u
          i32.add
          local.tee $0
          i32.const 7
          i32.add
          i32.shr_u
          i32.const 1
          i32.and
          local.get $0
          i32.const 1
          i32.shl
          i32.or
        end
      else
        i32.const 0
      end
      local.tee $1
      i32.const 2
      i32.shl
      i32.const 4480
      i32.add
      local.set $0
      local.get $3
      local.get $1
      i32.store offset=28
      local.get $3
      i32.const 0
      i32.store offset=20
      local.get $3
      i32.const 0
      i32.store offset=16
      block $label$113 ;; label = @2
        i32.const 4180
        i32.load
        local.tee $4
        i32.const 1
        local.get $1
        i32.shl
        local.tee $5
        i32.and
        if ;; label = @3
          block ;; label = @4
            local.get $0
            i32.load
            local.set $0
            i32.const 25
            local.get $1
            i32.const 1
            i32.shr_u
            i32.sub
            local.set $4
            local.get $2
            local.get $1
            i32.const 31
            i32.eq
            if (result i32) ;; label = @5
              i32.const 0
            else
              local.get $4
            end
            i32.shl
            local.set $1
            block $label$117 ;; label = @5
              block $label$118 ;; label = @6
                block $label$119 ;; label = @7
                  loop $label$120 ;; label = @8
                    local.get $0
                    i32.load offset=4
                    i32.const -8
                    i32.and
                    local.get $2
                    i32.eq
                    br_if 2 (;@6;)
                    local.get $1
                    i32.const 1
                    i32.shl
                    local.set $4
                    local.get $0
                    i32.const 16
                    i32.add
                    local.get $1
                    i32.const 31
                    i32.shr_u
                    i32.const 2
                    i32.shl
                    i32.add
                    local.tee $1
                    i32.load
                    local.tee $5
                    i32.eqz
                    br_if 1 (;@7;)
                    local.get $4
                    local.set $1
                    local.get $5
                    local.set $0
                    br 0 (;@8;)
                  end
                end
                local.get $1
                i32.const 4192
                i32.load
                i32.lt_u
                if ;; label = @7
                  call $fimport$8
                else
                  block ;; label = @8
                    local.get $1
                    local.get $3
                    i32.store
                    local.get $3
                    local.get $0
                    i32.store offset=24
                    local.get $3
                    local.get $3
                    i32.store offset=12
                    local.get $3
                    local.get $3
                    i32.store offset=8
                    br 6 (;@2;)
                  end
                end
                br 1 (;@5;)
              end
              local.get $0
              i32.const 8
              i32.add
              local.tee $1
              i32.load
              local.tee $2
              i32.const 4192
              i32.load
              local.tee $4
              i32.ge_u
              local.get $0
              local.get $4
              i32.ge_u
              i32.and
              if ;; label = @6
                block ;; label = @7
                  local.get $2
                  local.get $3
                  i32.store offset=12
                  local.get $1
                  local.get $3
                  i32.store
                  local.get $3
                  local.get $2
                  i32.store offset=8
                  local.get $3
                  local.get $0
                  i32.store offset=12
                  local.get $3
                  i32.const 0
                  i32.store offset=24
                end
              else
                call $fimport$8
              end
            end
          end
        else
          block ;; label = @4
            i32.const 4180
            local.get $4
            local.get $5
            i32.or
            i32.store
            local.get $0
            local.get $3
            i32.store
            local.get $3
            local.get $0
            i32.store offset=24
            local.get $3
            local.get $3
            i32.store offset=12
            local.get $3
            local.get $3
            i32.store offset=8
          end
        end
      end
      i32.const 4208
      i32.const 4208
      i32.load
      i32.const -1
      i32.add
      local.tee $0
      i32.store
      local.get $0
      if ;; label = @2
        return
      else
        i32.const 4632
        local.set $0
      end
      loop $label$128 ;; label = @2
        local.get $0
        i32.load
        local.tee $2
        i32.const 8
        i32.add
        local.set $0
        local.get $2
        br_if 0 (;@2;)
      end
      i32.const 4208
      i32.const -1
      i32.store
    end
  )
  (func $39 (;52;) (type $2) (param $0 i32) (result i32)
    (local $1 i32)
    block $label$1 (result i32) ;; label = @1
      local.get $0
      i32.eqz
      if ;; label = @2
        i32.const 1
        local.set $0
      end
      loop $label$3 ;; label = @2
        block $label$4 ;; label = @3
          local.get $0
          call $37
          local.tee $1
          if ;; label = @4
            block ;; label = @5
              local.get $1
              local.set $0
              br 2 (;@3;)
            end
          end
          call $43
          local.tee $1
          if ;; label = @4
            block ;; label = @5
              local.get $1
              i32.const 0
              i32.and
              i32.const 8
              i32.add
              call_indirect (type $1)
              br 3 (;@2;)
            end
          else
            i32.const 0
            local.set $0
          end
        end
      end
      local.get $0
    end
  )
  (func $40 (;53;) (type $2) (param $0 i32) (result i32)
    local.get $0
    call $39
  )
  (func $41 (;54;) (type $3) (param $0 i32)
    local.get $0
    call $38
  )
  (func $42 (;55;) (type $3) (param $0 i32)
    local.get $0
    call $41
  )
  (func $43 (;56;) (type $4) (result i32)
    (local $0 i32)
    block $label$1 (result i32) ;; label = @1
      i32.const 4672
      i32.const 4672
      i32.load
      local.tee $0
      i32.const 0
      i32.add
      i32.store
      local.get $0
    end
  )
  (func $44 (;57;) (type $1)
    nop
  )
  (func $45 (;58;) (type $2) (param $0 i32) (result i32)
    (local $1 i32) (local $2 i32)
    block $label$1 (result i32) ;; label = @1
      global.get $global$0
      i32.load
      local.tee $2
      local.get $0
      i32.const 15
      i32.add
      i32.const -16
      i32.and
      local.tee $0
      i32.add
      local.set $1
      local.get $0
      i32.const 0
      i32.gt_s
      local.get $1
      local.get $2
      i32.lt_s
      i32.and
      local.get $1
      i32.const 0
      i32.lt_s
      i32.or
      if ;; label = @2
        block ;; label = @3
          call $fimport$6
          drop
          i32.const 12
          call $fimport$11
          i32.const -1
          return
        end
      end
      global.get $global$0
      local.get $1
      i32.store
      local.get $1
      call $fimport$5
      i32.gt_s
      if ;; label = @2
        call $fimport$4
        i32.eqz
        if ;; label = @3
          block ;; label = @4
            i32.const 12
            call $fimport$11
            global.get $global$0
            local.get $2
            i32.store
            i32.const -1
            return
          end
        end
      end
      local.get $2
    end
  )
  (func $46 (;59;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    (local $3 i32) (local $4 i32) (local $5 i32)
    block $label$1 (result i32) ;; label = @1
      local.get $0
      local.get $2
      i32.add
      local.set $4
      local.get $2
      i32.const 20
      i32.ge_s
      if ;; label = @2
        block ;; label = @3
          local.get $1
          i32.const 255
          i32.and
          local.set $1
          local.get $0
          i32.const 3
          i32.and
          local.tee $3
          if ;; label = @4
            block ;; label = @5
              local.get $0
              i32.const 4
              i32.add
              local.get $3
              i32.sub
              local.set $3
              loop $label$4 ;; label = @6
                local.get $0
                local.get $3
                i32.lt_s
                if ;; label = @7
                  block ;; label = @8
                    local.get $0
                    local.get $1
                    i32.store8
                    local.get $0
                    i32.const 1
                    i32.add
                    local.set $0
                    br 2 (;@6;)
                  end
                end
              end
            end
          end
          local.get $1
          local.get $1
          i32.const 8
          i32.shl
          i32.or
          local.get $1
          i32.const 16
          i32.shl
          i32.or
          local.get $1
          i32.const 24
          i32.shl
          i32.or
          local.set $3
          local.get $4
          i32.const -4
          i32.and
          local.set $5
          loop $label$6 ;; label = @4
            local.get $0
            local.get $5
            i32.lt_s
            if ;; label = @5
              block ;; label = @6
                local.get $0
                local.get $3
                i32.store
                local.get $0
                i32.const 4
                i32.add
                local.set $0
                br 2 (;@4;)
              end
            end
          end
        end
      end
      loop $label$8 ;; label = @2
        local.get $0
        local.get $4
        i32.lt_s
        if ;; label = @3
          block ;; label = @4
            local.get $0
            local.get $1
            i32.store8
            local.get $0
            i32.const 1
            i32.add
            local.set $0
            br 2 (;@2;)
          end
        end
      end
      local.get $0
      local.get $2
      i32.sub
    end
  )
  (func $47 (;60;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    (local $3 i32)
    block $label$1 (result i32) ;; label = @1
      local.get $2
      i32.const 4096
      i32.ge_s
      if ;; label = @2
        local.get $0
        local.get $1
        local.get $2
        call $fimport$12
        return
      end
      local.get $0
      local.set $3
      local.get $0
      i32.const 3
      i32.and
      local.get $1
      i32.const 3
      i32.and
      i32.eq
      if ;; label = @2
        block ;; label = @3
          loop $label$4 ;; label = @4
            local.get $0
            i32.const 3
            i32.and
            if ;; label = @5
              block ;; label = @6
                local.get $2
                i32.eqz
                if ;; label = @7
                  local.get $3
                  return
                end
                local.get $0
                local.get $1
                i32.load8_s
                i32.store8
                local.get $0
                i32.const 1
                i32.add
                local.set $0
                local.get $1
                i32.const 1
                i32.add
                local.set $1
                local.get $2
                i32.const 1
                i32.sub
                local.set $2
                br 2 (;@4;)
              end
            end
          end
          loop $label$7 ;; label = @4
            local.get $2
            i32.const 4
            i32.ge_s
            if ;; label = @5
              block ;; label = @6
                local.get $0
                local.get $1
                i32.load
                i32.store
                local.get $0
                i32.const 4
                i32.add
                local.set $0
                local.get $1
                i32.const 4
                i32.add
                local.set $1
                local.get $2
                i32.const 4
                i32.sub
                local.set $2
                br 2 (;@4;)
              end
            end
          end
        end
      end
      loop $label$9 ;; label = @2
        local.get $2
        i32.const 0
        i32.gt_s
        if ;; label = @3
          block ;; label = @4
            local.get $0
            local.get $1
            i32.load8_s
            i32.store8
            local.get $0
            i32.const 1
            i32.add
            local.set $0
            local.get $1
            i32.const 1
            i32.add
            local.set $1
            local.get $2
            i32.const 1
            i32.sub
            local.set $2
            br 2 (;@2;)
          end
        end
      end
      local.get $3
    end
  )
  (func $48 (;61;) (type $4) (result i32)
    i32.const 0
  )
  (func $49 (;62;) (type $6) (param $0 i32) (param $1 i32) (result i32)
    local.get $1
    local.get $0
    i32.const 1
    i32.and
    i32.const 0
    i32.add
    call_indirect (type $2)
  )
  (func $50 (;63;) (type $12) (param $0 i32) (param $1 i32) (param $2 i32) (param $3 i32) (result i32)
    local.get $1
    local.get $2
    local.get $3
    local.get $0
    i32.const 3
    i32.and
    i32.const 2
    i32.add
    call_indirect (type $0)
  )
  (func $51 (;64;) (type $5) (param $0 i32) (param $1 i32)
    local.get $1
    local.get $0
    i32.const 1
    i32.and
    i32.const 6
    i32.add
    call_indirect (type $3)
  )
  (func $52 (;65;) (type $3) (param $0 i32)
    local.get $0
    i32.const 0
    i32.and
    i32.const 8
    i32.add
    call_indirect (type $1)
  )
  (func $53 (;66;) (type $2) (param $0 i32) (result i32)
    block $label$1 (result i32) ;; label = @1
      i32.const 0
      call $fimport$3
      i32.const 0
    end
  )
  (func $54 (;67;) (type $0) (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
    block $label$1 (result i32) ;; label = @1
      i32.const 1
      call $fimport$3
      i32.const 0
    end
  )
  (func $55 (;68;) (type $3) (param $0 i32)
    i32.const 2
    call $fimport$3
  )
  (func $56 (;69;) (type $1)
    i32.const 3
    call $fimport$3
  )
  (global $global$0 (;5;) (mut i32) global.get $gimport$0)
  (global $global$1 (;6;) (mut i32) global.get $gimport$1)
  (global $global$2 (;7;) (mut i32) global.get $gimport$2)
  (global $global$3 (;8;) (mut i32) i32.const 0)
  (global $global$4 (;9;) (mut i32) i32.const 0)
  (global $global$5 (;10;) (mut i32) i32.const 0)
  (export "_sbrk" (func $45))
  (export "_free" (func $38))
  (export "_main" (func $7))
  (export "_pthread_self" (func $48))
  (export "_memset" (func $46))
  (export "_malloc" (func $37))
  (export "_memcpy" (func $47))
  (export "___errno_location" (func $12))
  (export "runPostSets" (func $44))
  (export "stackAlloc" (func $0))
  (export "stackSave" (func $1))
  (export "stackRestore" (func $2))
  (export "establishStackSpace" (func $3))
  (export "setThrew" (func $4))
  (export "setTempRet0" (func $5))
  (export "getTempRet0" (func $6))
  (export "dynCall_ii" (func $49))
  (export "dynCall_iiii" (func $50))
  (export "dynCall_vi" (func $51))
  (export "dynCall_v" (func $52))
  (elem (;0;) (global.get $gimport$19) func $53 $9 $54 $14 $10 $15 $55 $16 $56)
  (data (;0;) (i32.const 1024) "&\02\00\00a\00\00\00q=\8a>\00\00\00\00c\00\00\00\8f\c2\f5=\00\00\00\00g\00\00\00\8f\c2\f5=\00\00\00\00t\00\00\00q=\8a>\00\00\00\00B\00\00\00\0a\d7\a3<\00\00\00\00D\00\00\00\0a\d7\a3<\00\00\00\00H\00\00\00\0a\d7\a3<\00\00\00\00K\00\00\00\0a\d7\a3<\00\00\00\00M\00\00\00\0a\d7\a3<\00\00\00\00N\00\00\00\0a\d7\a3<\00\00\00\00R\00\00\00\0a\d7\a3<\00\00\00\00S\00\00\00\0a\d7\a3<\00\00\00\00V\00\00\00\0a\d7\a3<\00\00\00\00W\00\00\00\0a\d7\a3<\00\00\00\00Y\00\00\00\0a\d7\a3<")
  (data (;1;) (i32.const 1220) "a\00\00\00\e9\1c\9b>\00\00\00\00c\00\00\00r\bdJ>\00\00\00\00g\00\00\00\d7IJ>\00\00\00\00t\00\00\00r_\9a>")
  (data (;2;) (i32.const 1280) "\04\05\00\00\05")
  (data (;3;) (i32.const 1296) "\01")
  (data (;4;) (i32.const 1320) "\01\00\00\00\02\00\00\00L\12\00\00\00\04")
  (data (;5;) (i32.const 1344) "\01")
  (data (;6;) (i32.const 1359) "\0a\ff\ff\ff\ff")
  (data (;7;) (i32.const 1396) "*\00\00\00error: %d\0a\00GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA\00\11\00\0a\00\11\11\11\00\00\00\00\05\00\00\00\00\00\00\09\00\00\00\00\0b")
  (data (;8;) (i32.const 1731) "\11\00\0f\0a\11\11\11\03\0a\07\00\01\13\09\0b\0b\00\00\09\06\0b\00\00\0b\00\06\11\00\00\00\11\11\11")
  (data (;9;) (i32.const 1780) "\0b")
  (data (;10;) (i32.const 1789) "\11\00\0a\0a\11\11\11\00\0a\00\00\02\00\09\0b\00\00\00\09\00\0b\00\00\0b")
  (data (;11;) (i32.const 1838) "\0c")
  (data (;12;) (i32.const 1850) "\0c\00\00\00\00\0c\00\00\00\00\09\0c\00\00\00\00\00\0c\00\00\0c")
  (data (;13;) (i32.const 1896) "\0e")
  (data (;14;) (i32.const 1908) "\0d\00\00\00\04\0d\00\00\00\00\09\0e\00\00\00\00\00\0e\00\00\0e")
  (data (;15;) (i32.const 1954) "\10")
  (data (;16;) (i32.const 1966) "\0f\00\00\00\00\0f\00\00\00\00\09\10\00\00\00\00\00\10\00\00\10\00\00\12\00\00\00\12\12\12")
  (data (;17;) (i32.const 2021) "\12\00\00\00\12\12\12\00\00\00\00\00\00\09")
  (data (;18;) (i32.const 2070) "\0b")
  (data (;19;) (i32.const 2082) "\0a\00\00\00\00\0a\00\00\00\00\09\0b\00\00\00\00\00\0b\00\00\0b")
  (data (;20;) (i32.const 2128) "\0c")
  (data (;21;) (i32.const 2140) "\0c\00\00\00\00\0c\00\00\00\00\09\0c\00\00\00\00\00\0c\00\00\0c\00\000123456789ABCDEF-+   0X0x\00(null)\00-0X+0X 0X-0x+0x 0x\00inf\00INF\00nan\00NAN\00.\00T!\22\19\0d\01\02\03\11K\1c\0c\10\04\0b\1d\12\1e'hnopqb \05\06\0f\13\14\15\1a\08\16\07($\17\18\09\0a\0e\1b\1f%#\83\82}&*+<=>?CGJMXYZ[\5c]^_`acdefgijklrstyz{|\00Illegal byte sequence\00Domain error\00Result not representable\00Not a tty\00Permission denied\00Operation not permitted\00No such file or directory\00No such process\00File exists\00Value too large for data type\00No space left on device\00Out of memory\00Resource busy\00Interrupted system call\00Resource temporarily unavailable\00Invalid seek\00Cross-device link\00Read-only file system\00Directory not empty\00Connection reset by peer\00Operation timed out\00Connection refused\00Host is down\00Host is unreachable\00Address in use\00Broken pipe\00I/O error\00No such device or address\00Block device required\00No such device\00Not a directory\00Is a directory\00Text file busy\00Exec format error\00Invalid argument\00Argument list too long\00Symbolic link loop\00Filename too long\00Too many open files in system\00No file descriptors available\00Bad file descriptor\00No child process\00Bad address\00File too large\00Too many links\00No locks available\00Resource deadlock would occur\00State not recoverable\00Previous owner died\00Operation canceled\00Function not implemented\00No message of desired type\00Identifier removed\00Device not a stream\00No data available\00Device timeout\00Out of streams resources\00Link has been severed\00Protocol error\00Bad message\00File descriptor in bad state\00Not a socket\00Destination address required\00Message too large\00Protocol wrong type for socket\00Protocol not available\00Protocol not supported\00Socket type not supported\00Not supported\00Protocol family not supported\00Address family not supported by protocol\00Address not available\00Network is down\00Network unreachable\00Connection reset by network\00Connection aborted\00No buffer space available\00Socket is connected\00Socket not connected\00Cannot send after socket shutdown\00Operation already in progress\00Operation in progress\00Stale file handle\00Remote I/O error\00Quota exceeded\00No medium found\00Wrong medium type\00No error information")
)
