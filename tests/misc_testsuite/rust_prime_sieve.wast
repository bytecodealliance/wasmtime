(module
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func (param i32 i32 i32) (result i32)))
  (type (;2;) (func (param i32 i32) (result i32)))
  (type (;3;) (func (param i32) (result i32)))
  (type (;4;) (func (param i32)))
  (type (;5;) (func (param i32 i32 i32)))
  (type (;6;) (func (param i32) (result i64)))
  (type (;7;) (func (param i32 i32 i32 i32) (result i32)))
  (type (;8;) (func))
  (type (;9;) (func (param i32 i32 i32 i32)))
  (type (;10;) (func (param i32 i32 i32 i32 i32 i32) (result i32)))
  (type (;11;) (func (param i64 i32 i32) (result i32)))
  (type (;12;) (func (param f64) (result f64)))
  (func $_ZN16prime_sieve_wasm5sieve17h5e51f1bdbe7e8205E (type 0) (param i32 i32)
    (local i32 i32 i32 i32 i32 i32 f64 i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          get_local 1
          i32.const 3
          i32.lt_u
          br_if 0 (;@3;)
          get_local 1
          i32.const -3
          i32.add
          tee_local 2
          i32.const 6
          i32.shr_u
          tee_local 3
          i32.const 1
          i32.add
          tee_local 4
          i32.const 2
          i32.shl
          tee_local 5
          i32.const 4
          call $__rust_alloc_zeroed
          tee_local 6
          i32.eqz
          br_if 2 (;@1;)
          get_local 2
          i32.const 1
          i32.shr_u
          set_local 7
          block  ;; label = @4
            block  ;; label = @5
              get_local 1
              f64.convert_u/i32
              f64.sqrt
              f64.ceil
              tee_local 8
              f64.const 0x1p+32 (;=4.29497e+09;)
              f64.lt
              get_local 8
              f64.const 0x0p+0 (;=0;)
              f64.ge
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              get_local 8
              i32.trunc_u/f64
              set_local 2
              br 1 (;@4;)
            end
            i32.const 0
            set_local 2
          end
          get_local 7
          i32.const 1
          i32.add
          set_local 9
          i32.const 0
          set_local 1
          i32.const 0
          get_local 2
          i32.const -3
          i32.add
          tee_local 10
          get_local 10
          get_local 2
          i32.gt_u
          select
          i32.const 1
          i32.shr_u
          set_local 11
          loop  ;; label = @4
            get_local 3
            get_local 1
            tee_local 10
            i32.const 5
            i32.shr_u
            tee_local 1
            i32.lt_u
            br_if 2 (;@2;)
            block  ;; label = @5
              get_local 6
              get_local 1
              i32.const 2
              i32.shl
              i32.add
              i32.load
              i32.const 1
              get_local 10
              i32.const 31
              i32.and
              i32.shl
              i32.and
              br_if 0 (;@5;)
              get_local 10
              i32.const 1
              i32.shl
              i32.const 3
              i32.add
              tee_local 12
              get_local 12
              i32.mul
              i32.const -3
              i32.add
              i32.const 1
              i32.shr_u
              tee_local 1
              get_local 7
              i32.gt_u
              br_if 0 (;@5;)
              loop  ;; label = @6
                get_local 3
                get_local 1
                i32.const 5
                i32.shr_u
                tee_local 2
                i32.lt_u
                br_if 4 (;@2;)
                get_local 6
                get_local 2
                i32.const 2
                i32.shl
                i32.add
                tee_local 2
                get_local 2
                i32.load
                i32.const 1
                get_local 1
                i32.const 31
                i32.and
                i32.shl
                i32.or
                i32.store
                get_local 1
                get_local 12
                i32.add
                tee_local 1
                get_local 7
                i32.le_u
                br_if 0 (;@6;)
              end
            end
            get_local 10
            i32.const 1
            i32.add
            set_local 1
            get_local 10
            get_local 11
            i32.ne
            br_if 0 (;@4;)
          end
          get_local 0
          get_local 4
          i32.store offset=12
          get_local 0
          get_local 6
          i32.store offset=8
          get_local 0
          get_local 9
          i32.store offset=4
          get_local 0
          i32.const -1
          i32.store
          get_local 0
          i32.const 16
          i32.add
          get_local 4
          i32.store
          return
        end
        get_local 0
        i64.const 4
        i64.store offset=8 align=4
        get_local 0
        i32.const -1
        i32.store
        get_local 0
        i32.const 16
        i32.add
        i32.const 0
        i32.store
        get_local 0
        get_local 1
        i32.const -2
        i32.add
        i32.store offset=4
        return
      end
      get_local 0
      i32.const 0
      i32.store offset=8
      get_local 6
      get_local 5
      i32.const 4
      call $__rust_dealloc
      return
    end
    get_local 5
    i32.const 4
    call $_ZN5alloc5alloc18handle_alloc_error17hc11aade6dede5d47E
    unreachable)
  (func $nth_prime (type 3) (param i32) (result i32)
    (local i32 i32 f64 f64 i32 i32 i32 i32 i32 i32 i32 i32)
    get_global 0
    i32.const 32
    i32.sub
    tee_local 1
    set_global 0
    get_local 1
    get_local 0
    i32.store
    i32.const 20
    set_local 2
    block  ;; label = @1
      get_local 0
      i32.const 6
      i32.lt_u
      br_if 0 (;@1;)
      block  ;; label = @2
        get_local 0
        f64.convert_u/i32
        tee_local 3
        call $log2
        tee_local 4
        get_local 3
        f64.mul
        get_local 4
        call $log2
        get_local 3
        f64.mul
        f64.add
        f64.ceil
        tee_local 3
        f64.const 0x1p+32 (;=4.29497e+09;)
        f64.lt
        get_local 3
        f64.const 0x0p+0 (;=0;)
        f64.ge
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        get_local 3
        i32.trunc_u/f64
        set_local 2
        br 1 (;@1;)
      end
      i32.const 0
      set_local 2
    end
    get_local 1
    get_local 2
    i32.store offset=4
    get_local 1
    i32.const 8
    i32.add
    get_local 2
    call $_ZN16prime_sieve_wasm5sieve17h5e51f1bdbe7e8205E
    block  ;; label = @1
      block  ;; label = @2
        get_local 1
        i32.load offset=16
        tee_local 5
        br_if 0 (;@2;)
        i32.const -1
        set_local 6
        br 1 (;@1;)
      end
      block  ;; label = @2
        get_local 1
        i32.load offset=8
        tee_local 2
        i32.const 2147483647
        i32.eq
        br_if 0 (;@2;)
        get_local 2
        get_local 1
        i32.load offset=12
        tee_local 7
        i32.ge_s
        br_if 0 (;@2;)
        get_local 1
        i32.const 24
        i32.add
        i32.load
        set_local 8
        get_local 1
        i32.load offset=20
        set_local 9
        get_local 2
        i32.const 1
        i32.shl
        i32.const 3
        i32.add
        set_local 10
        get_local 2
        i32.const 0
        i32.lt_s
        set_local 11
        i32.const 2
        set_local 6
        loop  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              get_local 11
              br_if 0 (;@5;)
              get_local 8
              get_local 2
              i32.const 5
              i32.shr_u
              tee_local 12
              i32.le_u
              br_if 1 (;@4;)
              get_local 5
              get_local 12
              i32.const 2
              i32.shl
              i32.add
              i32.load
              i32.const 1
              get_local 2
              i32.const 31
              i32.and
              i32.shl
              i32.and
              br_if 1 (;@4;)
              get_local 10
              set_local 6
            end
            block  ;; label = @5
              get_local 0
              i32.eqz
              br_if 0 (;@5;)
              get_local 2
              i32.const 1
              i32.add
              set_local 11
              loop  ;; label = @6
                get_local 11
                i32.const 2147483647
                i32.eq
                br_if 4 (;@2;)
                get_local 11
                get_local 7
                i32.ge_s
                br_if 4 (;@2;)
                get_local 0
                i32.const -1
                i32.add
                set_local 0
                get_local 11
                i32.const 1
                i32.shl
                i32.const 3
                i32.add
                set_local 6
                get_local 11
                set_local 2
                block  ;; label = @7
                  loop  ;; label = @8
                    get_local 2
                    i32.const 1
                    i32.add
                    set_local 10
                    block  ;; label = @9
                      get_local 11
                      i32.const 0
                      i32.ge_s
                      br_if 0 (;@9;)
                      i32.const 2
                      set_local 6
                      br 2 (;@7;)
                    end
                    block  ;; label = @9
                      get_local 8
                      get_local 2
                      i32.const 5
                      i32.shr_u
                      tee_local 12
                      i32.le_u
                      br_if 0 (;@9;)
                      get_local 5
                      get_local 12
                      i32.const 2
                      i32.shl
                      i32.add
                      i32.load
                      i32.const 1
                      get_local 2
                      i32.const 31
                      i32.and
                      i32.shl
                      i32.and
                      i32.eqz
                      br_if 2 (;@7;)
                    end
                    get_local 2
                    i32.const 2147483646
                    i32.eq
                    br_if 6 (;@2;)
                    get_local 6
                    i32.const 2
                    i32.add
                    set_local 6
                    get_local 10
                    set_local 2
                    get_local 10
                    get_local 7
                    i32.lt_s
                    br_if 0 (;@8;)
                    br 6 (;@2;)
                  end
                end
                get_local 10
                set_local 11
                get_local 0
                br_if 0 (;@6;)
              end
            end
            get_local 9
            i32.eqz
            br_if 3 (;@1;)
            get_local 5
            get_local 9
            i32.const 2
            i32.shl
            i32.const 4
            call $__rust_dealloc
            br 3 (;@1;)
          end
          get_local 2
          i32.const 2147483646
          i32.eq
          br_if 1 (;@2;)
          get_local 10
          i32.const 2
          i32.add
          set_local 10
          get_local 2
          i32.const 1
          i32.add
          tee_local 2
          get_local 7
          i32.lt_s
          br_if 0 (;@3;)
        end
      end
      get_local 1
      i32.const 4
      i32.add
      get_local 1
      call $_ZN16prime_sieve_wasm9nth_prime28_$u7b$$u7b$closure$u7d$$u7d$17h778fb3b2afcb20ecE
      unreachable
    end
    get_local 1
    i32.const 32
    i32.add
    set_global 0
    get_local 6)
  (func $_ZN16prime_sieve_wasm9nth_prime28_$u7b$$u7b$closure$u7d$$u7d$17h778fb3b2afcb20ecE (type 0) (param i32 i32)
    (local i32)
    get_global 0
    i32.const 48
    i32.sub
    tee_local 2
    set_global 0
    get_local 2
    i32.const 44
    i32.add
    i32.const 1
    i32.store
    get_local 2
    i32.const 28
    i32.add
    i32.const 2
    i32.store
    get_local 2
    i64.const 2
    i64.store offset=12 align=4
    get_local 2
    i32.const 1048580
    i32.store offset=8
    get_local 2
    get_local 1
    i32.store offset=40
    get_local 2
    i32.const 1
    i32.store offset=36
    get_local 2
    get_local 0
    i32.store offset=32
    get_local 2
    get_local 2
    i32.const 32
    i32.add
    i32.store offset=24
    get_local 2
    i32.const 8
    i32.add
    i32.const 1048608
    call $_ZN3std9panicking15begin_panic_fmt17he66f9d47f0aea72dE
    unreachable)
  (func $is_prime (type 3) (param i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    get_global 0
    i32.const 32
    i32.sub
    tee_local 1
    set_global 0
    get_local 1
    i32.const 8
    i32.add
    get_local 0
    call $_ZN16prime_sieve_wasm5sieve17h5e51f1bdbe7e8205E
    block  ;; label = @1
      block  ;; label = @2
        get_local 1
        i32.load offset=16
        tee_local 2
        br_if 0 (;@2;)
        i32.const 255
        set_local 3
        br 1 (;@1;)
      end
      get_local 1
      i32.load offset=20
      set_local 4
      block  ;; label = @2
        block  ;; label = @3
          get_local 1
          i32.load offset=8
          tee_local 3
          get_local 1
          i32.load offset=12
          tee_local 5
          i32.lt_s
          br_if 0 (;@3;)
          i32.const 0
          set_local 6
          br 1 (;@2;)
        end
        get_local 1
        i32.const 24
        i32.add
        i32.load
        set_local 7
        get_local 3
        i32.const 1
        i32.shl
        i32.const 3
        i32.add
        set_local 8
        i32.const 0
        set_local 6
        loop  ;; label = @3
          get_local 6
          set_local 9
          get_local 11
          set_local 10
          get_local 3
          tee_local 12
          i32.const 1
          i32.add
          set_local 3
          i32.const 2
          set_local 11
          i32.const 1
          set_local 6
          block  ;; label = @4
            get_local 12
            i32.const 0
            i32.lt_s
            br_if 0 (;@4;)
            block  ;; label = @5
              get_local 7
              get_local 12
              i32.const 5
              i32.shr_u
              tee_local 13
              i32.gt_u
              br_if 0 (;@5;)
              get_local 10
              set_local 11
              get_local 9
              set_local 6
              br 1 (;@4;)
            end
            get_local 10
            set_local 11
            get_local 9
            set_local 6
            get_local 2
            get_local 13
            i32.const 2
            i32.shl
            i32.add
            i32.load
            i32.const 1
            get_local 12
            i32.const 31
            i32.and
            i32.shl
            i32.and
            br_if 0 (;@4;)
            get_local 8
            set_local 11
            i32.const 1
            set_local 6
          end
          get_local 8
          i32.const 2
          i32.add
          set_local 8
          get_local 5
          get_local 3
          i32.ne
          br_if 0 (;@3;)
        end
      end
      block  ;; label = @2
        get_local 4
        i32.eqz
        br_if 0 (;@2;)
        get_local 2
        get_local 4
        i32.const 2
        i32.shl
        i32.const 4
        call $__rust_dealloc
      end
      get_local 6
      i32.const 1
      i32.eq
      get_local 11
      get_local 0
      i32.eq
      i32.and
      set_local 3
    end
    get_local 1
    i32.const 32
    i32.add
    set_global 0
    get_local 3)
  (func $assert_prime (type 4) (param i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    get_global 0
    i32.const 32
    i32.sub
    tee_local 1
    set_global 0
    get_local 1
    i32.const 8
    i32.add
    get_local 0
    call $_ZN16prime_sieve_wasm5sieve17h5e51f1bdbe7e8205E
    block  ;; label = @1
      get_local 1
      i32.load offset=16
      tee_local 2
      i32.eqz
      br_if 0 (;@1;)
      get_local 1
      i32.load offset=20
      set_local 3
      block  ;; label = @2
        block  ;; label = @3
          get_local 1
          i32.load offset=8
          tee_local 4
          get_local 1
          i32.load offset=12
          tee_local 5
          i32.lt_s
          br_if 0 (;@3;)
          i32.const 0
          set_local 6
          br 1 (;@2;)
        end
        get_local 1
        i32.const 24
        i32.add
        i32.load
        set_local 7
        get_local 4
        i32.const 1
        i32.shl
        i32.const 3
        i32.add
        set_local 8
        i32.const 0
        set_local 6
        loop  ;; label = @3
          get_local 6
          set_local 9
          get_local 11
          set_local 10
          get_local 4
          tee_local 12
          i32.const 1
          i32.add
          set_local 4
          i32.const 2
          set_local 11
          i32.const 1
          set_local 6
          block  ;; label = @4
            get_local 12
            i32.const 0
            i32.lt_s
            br_if 0 (;@4;)
            block  ;; label = @5
              get_local 7
              get_local 12
              i32.const 5
              i32.shr_u
              tee_local 13
              i32.gt_u
              br_if 0 (;@5;)
              get_local 10
              set_local 11
              get_local 9
              set_local 6
              br 1 (;@4;)
            end
            get_local 10
            set_local 11
            get_local 9
            set_local 6
            get_local 2
            get_local 13
            i32.const 2
            i32.shl
            i32.add
            i32.load
            i32.const 1
            get_local 12
            i32.const 31
            i32.and
            i32.shl
            i32.and
            br_if 0 (;@4;)
            get_local 8
            set_local 11
            i32.const 1
            set_local 6
          end
          get_local 8
          i32.const 2
          i32.add
          set_local 8
          get_local 5
          get_local 4
          i32.ne
          br_if 0 (;@3;)
        end
      end
      block  ;; label = @2
        get_local 3
        i32.eqz
        br_if 0 (;@2;)
        get_local 2
        get_local 3
        i32.const 2
        i32.shl
        i32.const 4
        call $__rust_dealloc
      end
      get_local 6
      i32.const 1
      i32.ne
      br_if 0 (;@1;)
      get_local 11
      get_local 0
      i32.ne
      br_if 0 (;@1;)
      get_local 1
      i32.const 32
      i32.add
      set_global 0
      return
    end
    i32.const 1048624
    i32.const 14
    i32.const 1048640
    call $_ZN3std9panicking11begin_panic17hc48705d5b41342a0E
    unreachable)
  (func $assert_not_prime (type 4) (param i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    get_global 0
    i32.const 32
    i32.sub
    tee_local 1
    set_global 0
    get_local 1
    i32.const 8
    i32.add
    get_local 0
    call $_ZN16prime_sieve_wasm5sieve17h5e51f1bdbe7e8205E
    block  ;; label = @1
      get_local 1
      i32.load offset=16
      tee_local 2
      i32.eqz
      br_if 0 (;@1;)
      get_local 1
      i32.load offset=20
      set_local 3
      block  ;; label = @2
        block  ;; label = @3
          get_local 1
          i32.load offset=8
          tee_local 4
          get_local 1
          i32.load offset=12
          tee_local 5
          i32.lt_s
          br_if 0 (;@3;)
          i32.const 0
          set_local 6
          br 1 (;@2;)
        end
        get_local 1
        i32.const 24
        i32.add
        i32.load
        set_local 7
        get_local 4
        i32.const 1
        i32.shl
        i32.const 3
        i32.add
        set_local 8
        i32.const 0
        set_local 6
        loop  ;; label = @3
          get_local 6
          set_local 9
          get_local 11
          set_local 10
          get_local 4
          tee_local 12
          i32.const 1
          i32.add
          set_local 4
          i32.const 2
          set_local 11
          i32.const 1
          set_local 6
          block  ;; label = @4
            get_local 12
            i32.const 0
            i32.lt_s
            br_if 0 (;@4;)
            block  ;; label = @5
              get_local 7
              get_local 12
              i32.const 5
              i32.shr_u
              tee_local 13
              i32.gt_u
              br_if 0 (;@5;)
              get_local 10
              set_local 11
              get_local 9
              set_local 6
              br 1 (;@4;)
            end
            get_local 10
            set_local 11
            get_local 9
            set_local 6
            get_local 2
            get_local 13
            i32.const 2
            i32.shl
            i32.add
            i32.load
            i32.const 1
            get_local 12
            i32.const 31
            i32.and
            i32.shl
            i32.and
            br_if 0 (;@4;)
            get_local 8
            set_local 11
            i32.const 1
            set_local 6
          end
          get_local 8
          i32.const 2
          i32.add
          set_local 8
          get_local 5
          get_local 4
          i32.ne
          br_if 0 (;@3;)
        end
      end
      block  ;; label = @2
        get_local 3
        i32.eqz
        br_if 0 (;@2;)
        get_local 2
        get_local 3
        i32.const 2
        i32.shl
        i32.const 4
        call $__rust_dealloc
      end
      block  ;; label = @2
        get_local 6
        i32.const 1
        i32.ne
        br_if 0 (;@2;)
        get_local 11
        get_local 0
        i32.eq
        br_if 1 (;@1;)
      end
      get_local 1
      i32.const 32
      i32.add
      set_global 0
      return
    end
    i32.const 1048624
    i32.const 14
    i32.const 1048656
    call $_ZN3std9panicking11begin_panic17hc48705d5b41342a0E
    unreachable)
  (func $_ZN3std9panicking11begin_panic17hc48705d5b41342a0E (type 5) (param i32 i32 i32)
    (local i32)
    get_global 0
    i32.const 16
    i32.sub
    tee_local 3
    set_global 0
    get_local 3
    get_local 1
    i32.store offset=12
    get_local 3
    get_local 0
    i32.store offset=8
    get_local 3
    i32.const 8
    i32.add
    i32.const 1048672
    i32.const 0
    get_local 2
    call $_ZN4core5panic8Location6caller17ha201287b09b4d397E
    call $_ZN3std9panicking20rust_panic_with_hook17he526fc090dcaf1f1E
    unreachable)
  (func $_ZN4core3ptr13drop_in_place17h3ede11bfa89f9051E (type 4) (param i32))
  (func $_ZN91_$LT$std..panicking..begin_panic..PanicPayload$LT$A$GT$$u20$as$u20$core..panic..BoxMeUp$GT$3get17h4739c367a12a045cE (type 0) (param i32 i32)
    block  ;; label = @1
      get_local 1
      i32.load
      br_if 0 (;@1;)
      call $_ZN3std7process5abort17hea452e383111f94bE
      unreachable
    end
    get_local 0
    i32.const 1048692
    i32.store offset=4
    get_local 0
    get_local 1
    i32.store)
  (func $_ZN91_$LT$std..panicking..begin_panic..PanicPayload$LT$A$GT$$u20$as$u20$core..panic..BoxMeUp$GT$8take_box17hde9b5c896fde66d8E (type 0) (param i32 i32)
    (local i32 i32)
    get_local 1
    i32.load
    set_local 2
    get_local 1
    i32.const 0
    i32.store
    block  ;; label = @1
      block  ;; label = @2
        get_local 2
        i32.eqz
        br_if 0 (;@2;)
        get_local 1
        i32.load offset=4
        set_local 3
        i32.const 8
        i32.const 4
        call $__rust_alloc
        tee_local 1
        i32.eqz
        br_if 1 (;@1;)
        get_local 1
        get_local 3
        i32.store offset=4
        get_local 1
        get_local 2
        i32.store
        get_local 0
        i32.const 1048692
        i32.store offset=4
        get_local 0
        get_local 1
        i32.store
        return
      end
      call $_ZN3std7process5abort17hea452e383111f94bE
      unreachable
    end
    i32.const 8
    i32.const 4
    call $_ZN5alloc5alloc18handle_alloc_error17hc11aade6dede5d47E
    unreachable)
  (func $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h9f9141c4d3b51f22E (type 6) (param i32) (result i64)
    i64.const 1229646359891580772)
  (func $__rust_alloc (type 2) (param i32 i32) (result i32)
    (local i32)
    get_local 0
    get_local 1
    call $__rdl_alloc
    set_local 2
    get_local 2
    return)
  (func $__rust_dealloc (type 5) (param i32 i32 i32)
    get_local 0
    get_local 1
    get_local 2
    call $__rdl_dealloc
    return)
  (func $__rust_realloc (type 7) (param i32 i32 i32 i32) (result i32)
    (local i32)
    get_local 0
    get_local 1
    get_local 2
    get_local 3
    call $__rdl_realloc
    set_local 4
    get_local 4
    return)
  (func $__rust_alloc_zeroed (type 2) (param i32 i32) (result i32)
    (local i32)
    get_local 0
    get_local 1
    call $__rdl_alloc_zeroed
    set_local 2
    get_local 2
    return)
  (func $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h793a77747fda2e92E (type 6) (param i32) (result i64)
    i64.const -1885627845604123106)
  (func $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h9ba061566a081877E (type 6) (param i32) (result i64)
    i64.const -620717282625089513)
  (func $_ZN4core3ptr13drop_in_place17h091c0eb23df207dbE (type 4) (param i32))
  (func $_ZN4core3ptr13drop_in_place17h0a661afa028ee03bE (type 4) (param i32)
    (local i32)
    block  ;; label = @1
      get_local 0
      i32.const 4
      i32.add
      i32.load
      tee_local 1
      i32.eqz
      br_if 0 (;@1;)
      get_local 0
      i32.load
      get_local 1
      i32.const 1
      call $__rust_dealloc
    end)
  (func $_ZN4core3ptr13drop_in_place17hfda159fcd4de37a1E (type 4) (param i32)
    (local i32)
    block  ;; label = @1
      get_local 0
      i32.load offset=4
      tee_local 1
      i32.eqz
      br_if 0 (;@1;)
      get_local 0
      i32.const 8
      i32.add
      i32.load
      tee_local 0
      i32.eqz
      br_if 0 (;@1;)
      get_local 1
      get_local 0
      i32.const 1
      call $__rust_dealloc
    end)
  (func $_ZN4core6option15Option$LT$T$GT$6unwrap17h1b13daca67935d6bE (type 2) (param i32 i32) (result i32)
    block  ;; label = @1
      get_local 0
      br_if 0 (;@1;)
      i32.const 1048748
      i32.const 43
      get_local 1
      call $_ZN4core9panicking5panic17h2574daf311dde64fE
      unreachable
    end
    get_local 0)
  (func $_ZN4core6option15Option$LT$T$GT$6unwrap17h2300eb04729b1156E (type 3) (param i32) (result i32)
    block  ;; label = @1
      get_local 0
      br_if 0 (;@1;)
      i32.const 1048748
      i32.const 43
      i32.const 1048832
      call $_ZN4core9panicking5panic17h2574daf311dde64fE
      unreachable
    end
    get_local 0)
  (func $_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17h65719fab9c99edbbE (type 2) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32)
    get_global 0
    i32.const 16
    i32.sub
    tee_local 2
    set_global 0
    get_local 0
    i32.load
    set_local 0
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  get_local 1
                  i32.const 128
                  i32.lt_u
                  br_if 0 (;@7;)
                  get_local 2
                  i32.const 0
                  i32.store offset=12
                  get_local 1
                  i32.const 2048
                  i32.lt_u
                  br_if 1 (;@6;)
                  get_local 2
                  i32.const 12
                  i32.add
                  set_local 3
                  block  ;; label = @8
                    get_local 1
                    i32.const 65536
                    i32.ge_u
                    br_if 0 (;@8;)
                    get_local 2
                    get_local 1
                    i32.const 63
                    i32.and
                    i32.const 128
                    i32.or
                    i32.store8 offset=14
                    get_local 2
                    get_local 1
                    i32.const 6
                    i32.shr_u
                    i32.const 63
                    i32.and
                    i32.const 128
                    i32.or
                    i32.store8 offset=13
                    get_local 2
                    get_local 1
                    i32.const 12
                    i32.shr_u
                    i32.const 15
                    i32.and
                    i32.const 224
                    i32.or
                    i32.store8 offset=12
                    i32.const 3
                    set_local 1
                    br 4 (;@4;)
                  end
                  get_local 2
                  get_local 1
                  i32.const 63
                  i32.and
                  i32.const 128
                  i32.or
                  i32.store8 offset=15
                  get_local 2
                  get_local 1
                  i32.const 18
                  i32.shr_u
                  i32.const 240
                  i32.or
                  i32.store8 offset=12
                  get_local 2
                  get_local 1
                  i32.const 6
                  i32.shr_u
                  i32.const 63
                  i32.and
                  i32.const 128
                  i32.or
                  i32.store8 offset=14
                  get_local 2
                  get_local 1
                  i32.const 12
                  i32.shr_u
                  i32.const 63
                  i32.and
                  i32.const 128
                  i32.or
                  i32.store8 offset=13
                  i32.const 4
                  set_local 1
                  br 3 (;@4;)
                end
                block  ;; label = @7
                  block  ;; label = @8
                    get_local 0
                    i32.load offset=8
                    tee_local 4
                    get_local 0
                    i32.const 4
                    i32.add
                    i32.load
                    i32.eq
                    br_if 0 (;@8;)
                    get_local 0
                    i32.load
                    set_local 5
                    br 1 (;@7;)
                  end
                  get_local 4
                  i32.const 1
                  i32.add
                  tee_local 5
                  get_local 4
                  i32.lt_u
                  br_if 6 (;@1;)
                  get_local 4
                  i32.const 1
                  i32.shl
                  tee_local 3
                  get_local 5
                  get_local 3
                  get_local 5
                  i32.gt_u
                  select
                  tee_local 3
                  i32.const 0
                  i32.lt_s
                  br_if 6 (;@1;)
                  block  ;; label = @8
                    block  ;; label = @9
                      get_local 4
                      br_if 0 (;@9;)
                      get_local 3
                      i32.const 1
                      call $__rust_alloc
                      set_local 5
                      br 1 (;@8;)
                    end
                    get_local 0
                    i32.load
                    get_local 4
                    i32.const 1
                    get_local 3
                    call $__rust_realloc
                    set_local 5
                  end
                  get_local 5
                  i32.eqz
                  br_if 2 (;@5;)
                  get_local 0
                  get_local 5
                  i32.store
                  get_local 0
                  i32.const 4
                  i32.add
                  get_local 3
                  i32.store
                  get_local 0
                  i32.load offset=8
                  set_local 4
                end
                get_local 5
                get_local 4
                i32.add
                get_local 1
                i32.store8
                get_local 0
                get_local 0
                i32.load offset=8
                i32.const 1
                i32.add
                i32.store offset=8
                br 3 (;@3;)
              end
              get_local 2
              get_local 1
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8 offset=13
              get_local 2
              get_local 1
              i32.const 6
              i32.shr_u
              i32.const 31
              i32.and
              i32.const 192
              i32.or
              i32.store8 offset=12
              get_local 2
              i32.const 12
              i32.add
              set_local 3
              i32.const 2
              set_local 1
              br 1 (;@4;)
            end
            get_local 3
            i32.const 1
            call $_ZN5alloc5alloc18handle_alloc_error17hc11aade6dede5d47E
            unreachable
          end
          block  ;; label = @4
            block  ;; label = @5
              get_local 0
              i32.const 4
              i32.add
              i32.load
              tee_local 5
              get_local 0
              i32.const 8
              i32.add
              i32.load
              tee_local 4
              i32.sub
              get_local 1
              i32.lt_u
              br_if 0 (;@5;)
              get_local 0
              i32.load
              set_local 5
              br 1 (;@4;)
            end
            get_local 4
            get_local 1
            i32.add
            tee_local 6
            get_local 4
            i32.lt_u
            br_if 3 (;@1;)
            get_local 5
            i32.const 1
            i32.shl
            tee_local 4
            get_local 6
            get_local 4
            get_local 6
            i32.gt_u
            select
            tee_local 4
            i32.const 0
            i32.lt_s
            br_if 3 (;@1;)
            block  ;; label = @5
              block  ;; label = @6
                get_local 5
                br_if 0 (;@6;)
                get_local 4
                i32.const 1
                call $__rust_alloc
                set_local 5
                br 1 (;@5;)
              end
              get_local 0
              i32.load
              get_local 5
              i32.const 1
              get_local 4
              call $__rust_realloc
              set_local 5
            end
            get_local 5
            i32.eqz
            br_if 2 (;@2;)
            get_local 0
            get_local 5
            i32.store
            get_local 0
            i32.const 4
            i32.add
            get_local 4
            i32.store
            get_local 0
            i32.const 8
            i32.add
            i32.load
            set_local 4
          end
          get_local 0
          i32.const 8
          i32.add
          get_local 4
          get_local 1
          i32.add
          i32.store
          get_local 5
          get_local 4
          i32.add
          get_local 3
          get_local 1
          call $memcpy
          drop
        end
        get_local 2
        i32.const 16
        i32.add
        set_global 0
        i32.const 0
        return
      end
      get_local 4
      i32.const 1
      call $_ZN5alloc5alloc18handle_alloc_error17hc11aade6dede5d47E
      unreachable
    end
    call $_ZN5alloc7raw_vec17capacity_overflow17he8b42367ee028944E
    unreachable)
  (func $_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$9write_fmt17h1e75a79353b91b58E (type 2) (param i32 i32) (result i32)
    (local i32)
    get_global 0
    i32.const 32
    i32.sub
    tee_local 2
    set_global 0
    get_local 2
    get_local 0
    i32.load
    i32.store offset=4
    get_local 2
    i32.const 8
    i32.add
    i32.const 16
    i32.add
    get_local 1
    i32.const 16
    i32.add
    i64.load align=4
    i64.store
    get_local 2
    i32.const 8
    i32.add
    i32.const 8
    i32.add
    get_local 1
    i32.const 8
    i32.add
    i64.load align=4
    i64.store
    get_local 2
    get_local 1
    i64.load align=4
    i64.store offset=8
    get_local 2
    i32.const 4
    i32.add
    i32.const 1048708
    get_local 2
    i32.const 8
    i32.add
    call $_ZN4core3fmt5write17h3d72ff271d605c43E
    set_local 1
    get_local 2
    i32.const 32
    i32.add
    set_global 0
    get_local 1)
  (func $_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$9write_str17h2561a40bc4fb773bE (type 1) (param i32 i32 i32) (result i32)
    (local i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            get_local 0
            i32.load
            tee_local 0
            i32.const 4
            i32.add
            i32.load
            tee_local 3
            get_local 0
            i32.const 8
            i32.add
            i32.load
            tee_local 4
            i32.sub
            get_local 2
            i32.lt_u
            br_if 0 (;@4;)
            get_local 0
            i32.load
            set_local 3
            br 1 (;@3;)
          end
          get_local 4
          get_local 2
          i32.add
          tee_local 5
          get_local 4
          i32.lt_u
          br_if 2 (;@1;)
          get_local 3
          i32.const 1
          i32.shl
          tee_local 4
          get_local 5
          get_local 4
          get_local 5
          i32.gt_u
          select
          tee_local 4
          i32.const 0
          i32.lt_s
          br_if 2 (;@1;)
          block  ;; label = @4
            block  ;; label = @5
              get_local 3
              br_if 0 (;@5;)
              get_local 4
              i32.const 1
              call $__rust_alloc
              set_local 3
              br 1 (;@4;)
            end
            get_local 0
            i32.load
            get_local 3
            i32.const 1
            get_local 4
            call $__rust_realloc
            set_local 3
          end
          get_local 3
          i32.eqz
          br_if 1 (;@2;)
          get_local 0
          get_local 3
          i32.store
          get_local 0
          i32.const 4
          i32.add
          get_local 4
          i32.store
          get_local 0
          i32.const 8
          i32.add
          i32.load
          set_local 4
        end
        get_local 0
        i32.const 8
        i32.add
        get_local 4
        get_local 2
        i32.add
        i32.store
        get_local 3
        get_local 4
        i32.add
        get_local 1
        get_local 2
        call $memcpy
        drop
        i32.const 0
        return
      end
      get_local 4
      i32.const 1
      call $_ZN5alloc5alloc18handle_alloc_error17hc11aade6dede5d47E
      unreachable
    end
    call $_ZN5alloc7raw_vec17capacity_overflow17he8b42367ee028944E
    unreachable)
  (func $_ZN3std9panicking15begin_panic_fmt17he66f9d47f0aea72dE (type 0) (param i32 i32)
    (local i32)
    get_global 0
    i32.const 16
    i32.sub
    tee_local 2
    set_global 0
    get_local 2
    get_local 1
    call $_ZN4core5panic8Location6caller17ha201287b09b4d397E
    i32.store offset=12
    get_local 2
    get_local 0
    i32.store offset=8
    get_local 2
    i32.const 1048732
    i32.store offset=4
    get_local 2
    i32.const 1048732
    i32.store
    get_local 2
    call $rust_begin_unwind
    unreachable)
  (func $_ZN3std7process5abort17hea452e383111f94bE (type 8)
    unreachable
    unreachable)
  (func $_ZN3std5alloc24default_alloc_error_hook17h5f799078d47b0575E (type 0) (param i32 i32))
  (func $rust_oom (type 0) (param i32 i32)
    (local i32)
    get_local 0
    get_local 1
    i32.const 0
    i32.load offset=1049280
    tee_local 2
    i32.const 6
    get_local 2
    select
    call_indirect (type 0)
    unreachable
    unreachable)
  (func $__rdl_alloc (type 2) (param i32 i32) (result i32)
    block  ;; label = @1
      i32.const 1049296
      call $_ZN8dlmalloc8dlmalloc8Dlmalloc16malloc_alignment17h35c3c2822c7fb378E
      get_local 1
      i32.ge_u
      br_if 0 (;@1;)
      i32.const 1049296
      get_local 1
      get_local 0
      call $_ZN8dlmalloc8dlmalloc8Dlmalloc8memalign17h6d9752ec0c410003E
      return
    end
    i32.const 1049296
    get_local 0
    call $_ZN8dlmalloc8dlmalloc8Dlmalloc6malloc17ha394eee15885f4c7E)
  (func $__rdl_dealloc (type 5) (param i32 i32 i32)
    i32.const 1049296
    get_local 0
    call $_ZN8dlmalloc8dlmalloc8Dlmalloc4free17hdc2dafb68e09cc01E)
  (func $__rdl_realloc (type 7) (param i32 i32 i32 i32) (result i32)
    block  ;; label = @1
      block  ;; label = @2
        i32.const 1049296
        call $_ZN8dlmalloc8dlmalloc8Dlmalloc16malloc_alignment17h35c3c2822c7fb378E
        get_local 2
        i32.ge_u
        br_if 0 (;@2;)
        block  ;; label = @3
          block  ;; label = @4
            i32.const 1049296
            call $_ZN8dlmalloc8dlmalloc8Dlmalloc16malloc_alignment17h35c3c2822c7fb378E
            get_local 2
            i32.ge_u
            br_if 0 (;@4;)
            i32.const 1049296
            get_local 2
            get_local 3
            call $_ZN8dlmalloc8dlmalloc8Dlmalloc8memalign17h6d9752ec0c410003E
            set_local 2
            br 1 (;@3;)
          end
          i32.const 1049296
          get_local 3
          call $_ZN8dlmalloc8dlmalloc8Dlmalloc6malloc17ha394eee15885f4c7E
          set_local 2
        end
        get_local 2
        br_if 1 (;@1;)
        i32.const 0
        return
      end
      i32.const 1049296
      get_local 0
      get_local 3
      call $_ZN8dlmalloc8dlmalloc8Dlmalloc7realloc17h58780d2b1a57ee25E
      return
    end
    get_local 2
    get_local 0
    get_local 3
    get_local 1
    get_local 1
    get_local 3
    i32.gt_u
    select
    call $memcpy
    set_local 2
    i32.const 1049296
    get_local 0
    call $_ZN8dlmalloc8dlmalloc8Dlmalloc4free17hdc2dafb68e09cc01E
    get_local 2)
  (func $__rdl_alloc_zeroed (type 2) (param i32 i32) (result i32)
    block  ;; label = @1
      block  ;; label = @2
        i32.const 1049296
        call $_ZN8dlmalloc8dlmalloc8Dlmalloc16malloc_alignment17h35c3c2822c7fb378E
        get_local 1
        i32.ge_u
        br_if 0 (;@2;)
        i32.const 1049296
        get_local 1
        get_local 0
        call $_ZN8dlmalloc8dlmalloc8Dlmalloc8memalign17h6d9752ec0c410003E
        set_local 1
        br 1 (;@1;)
      end
      i32.const 1049296
      get_local 0
      call $_ZN8dlmalloc8dlmalloc8Dlmalloc6malloc17ha394eee15885f4c7E
      set_local 1
    end
    block  ;; label = @1
      get_local 1
      i32.eqz
      br_if 0 (;@1;)
      i32.const 1049296
      get_local 1
      call $_ZN8dlmalloc8dlmalloc8Dlmalloc17calloc_must_clear17hc573cca86dc6fb14E
      i32.eqz
      br_if 0 (;@1;)
      get_local 1
      i32.const 0
      get_local 0
      call $memset
      drop
    end
    get_local 1)
  (func $rust_begin_unwind (type 4) (param i32)
    (local i32 i32 i32)
    get_global 0
    i32.const 16
    i32.sub
    tee_local 1
    set_global 0
    get_local 0
    call $_ZN4core5panic9PanicInfo8location17hf4c15ed78456999dE
    i32.const 1048816
    call $_ZN4core6option15Option$LT$T$GT$6unwrap17h1b13daca67935d6bE
    set_local 2
    get_local 0
    call $_ZN4core5panic9PanicInfo7message17h36a3d1b385d755a2E
    call $_ZN4core6option15Option$LT$T$GT$6unwrap17h2300eb04729b1156E
    set_local 3
    get_local 1
    i32.const 0
    i32.store offset=4
    get_local 1
    get_local 3
    i32.store
    get_local 1
    i32.const 1048848
    get_local 0
    call $_ZN4core5panic9PanicInfo7message17h36a3d1b385d755a2E
    get_local 2
    call $_ZN3std9panicking20rust_panic_with_hook17he526fc090dcaf1f1E
    unreachable)
  (func $_ZN3std9panicking20rust_panic_with_hook17he526fc090dcaf1f1E (type 9) (param i32 i32 i32 i32)
    (local i32 i32)
    get_global 0
    i32.const 32
    i32.sub
    tee_local 4
    set_global 0
    i32.const 1
    set_local 5
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            i32.const 0
            i32.load offset=1049752
            i32.const 1
            i32.eq
            br_if 0 (;@4;)
            i32.const 0
            i64.const 4294967297
            i64.store offset=1049752
            br 1 (;@3;)
          end
          i32.const 0
          i32.const 0
          i32.load offset=1049756
          i32.const 1
          i32.add
          tee_local 5
          i32.store offset=1049756
          get_local 5
          i32.const 2
          i32.gt_u
          br_if 1 (;@2;)
        end
        get_local 4
        get_local 3
        i32.store offset=28
        get_local 4
        get_local 2
        i32.store offset=24
        get_local 4
        i32.const 1048732
        i32.store offset=20
        get_local 4
        i32.const 1048732
        i32.store offset=16
        i32.const 0
        i32.load offset=1049284
        tee_local 2
        i32.const -1
        i32.le_s
        br_if 0 (;@2;)
        i32.const 0
        get_local 2
        i32.const 1
        i32.add
        tee_local 2
        i32.store offset=1049284
        block  ;; label = @3
          i32.const 0
          i32.load offset=1049292
          tee_local 3
          i32.eqz
          br_if 0 (;@3;)
          i32.const 0
          i32.load offset=1049288
          set_local 2
          get_local 4
          i32.const 8
          i32.add
          get_local 0
          get_local 1
          i32.load offset=16
          call_indirect (type 0)
          get_local 4
          get_local 4
          i64.load offset=8
          i64.store offset=16
          get_local 2
          get_local 4
          i32.const 16
          i32.add
          get_local 3
          i32.load offset=12
          call_indirect (type 0)
          i32.const 0
          i32.load offset=1049284
          set_local 2
        end
        i32.const 0
        get_local 2
        i32.const -1
        i32.add
        i32.store offset=1049284
        get_local 5
        i32.const 1
        i32.le_u
        br_if 1 (;@1;)
      end
      unreachable
      unreachable
    end
    get_local 0
    get_local 1
    call $rust_panic
    unreachable)
  (func $_ZN90_$LT$std..panicking..begin_panic_handler..PanicPayload$u20$as$u20$core..panic..BoxMeUp$GT$8take_box17h7fbe7fd4ced85654E (type 0) (param i32 i32)
    (local i32 i32 i32 i32 i32)
    get_global 0
    i32.const 64
    i32.sub
    tee_local 2
    set_global 0
    block  ;; label = @1
      get_local 1
      i32.load offset=4
      tee_local 3
      br_if 0 (;@1;)
      get_local 1
      i32.const 4
      i32.add
      set_local 3
      get_local 1
      i32.load
      set_local 4
      get_local 2
      i32.const 0
      i32.store offset=32
      get_local 2
      i64.const 1
      i64.store offset=24
      get_local 2
      get_local 2
      i32.const 24
      i32.add
      i32.store offset=36
      get_local 2
      i32.const 40
      i32.add
      i32.const 16
      i32.add
      get_local 4
      i32.const 16
      i32.add
      i64.load align=4
      i64.store
      get_local 2
      i32.const 40
      i32.add
      i32.const 8
      i32.add
      get_local 4
      i32.const 8
      i32.add
      i64.load align=4
      i64.store
      get_local 2
      get_local 4
      i64.load align=4
      i64.store offset=40
      get_local 2
      i32.const 36
      i32.add
      i32.const 1048708
      get_local 2
      i32.const 40
      i32.add
      call $_ZN4core3fmt5write17h3d72ff271d605c43E
      drop
      get_local 2
      i32.const 8
      i32.add
      i32.const 8
      i32.add
      tee_local 4
      get_local 2
      i32.load offset=32
      i32.store
      get_local 2
      get_local 2
      i64.load offset=24
      i64.store offset=8
      block  ;; label = @2
        get_local 1
        i32.load offset=4
        tee_local 5
        i32.eqz
        br_if 0 (;@2;)
        get_local 1
        i32.const 8
        i32.add
        i32.load
        tee_local 6
        i32.eqz
        br_if 0 (;@2;)
        get_local 5
        get_local 6
        i32.const 1
        call $__rust_dealloc
      end
      get_local 3
      get_local 2
      i64.load offset=8
      i64.store align=4
      get_local 3
      i32.const 8
      i32.add
      get_local 4
      i32.load
      i32.store
      get_local 3
      i32.load
      set_local 3
    end
    get_local 1
    i32.const 1
    i32.store offset=4
    get_local 1
    i32.const 12
    i32.add
    i32.load
    set_local 4
    get_local 1
    i32.const 8
    i32.add
    tee_local 1
    i32.load
    set_local 5
    get_local 1
    i64.const 0
    i64.store align=4
    block  ;; label = @1
      i32.const 12
      i32.const 4
      call $__rust_alloc
      tee_local 1
      br_if 0 (;@1;)
      i32.const 12
      i32.const 4
      call $_ZN5alloc5alloc18handle_alloc_error17hc11aade6dede5d47E
      unreachable
    end
    get_local 1
    get_local 4
    i32.store offset=8
    get_local 1
    get_local 5
    i32.store offset=4
    get_local 1
    get_local 3
    i32.store
    get_local 0
    i32.const 1048868
    i32.store offset=4
    get_local 0
    get_local 1
    i32.store
    get_local 2
    i32.const 64
    i32.add
    set_global 0)
  (func $_ZN90_$LT$std..panicking..begin_panic_handler..PanicPayload$u20$as$u20$core..panic..BoxMeUp$GT$3get17h6462774f78ab3653E (type 0) (param i32 i32)
    (local i32 i32 i32 i32)
    get_global 0
    i32.const 64
    i32.sub
    tee_local 2
    set_global 0
    get_local 1
    i32.const 4
    i32.add
    set_local 3
    block  ;; label = @1
      get_local 1
      i32.load offset=4
      br_if 0 (;@1;)
      get_local 1
      i32.load
      set_local 4
      get_local 2
      i32.const 0
      i32.store offset=32
      get_local 2
      i64.const 1
      i64.store offset=24
      get_local 2
      get_local 2
      i32.const 24
      i32.add
      i32.store offset=36
      get_local 2
      i32.const 40
      i32.add
      i32.const 16
      i32.add
      get_local 4
      i32.const 16
      i32.add
      i64.load align=4
      i64.store
      get_local 2
      i32.const 40
      i32.add
      i32.const 8
      i32.add
      get_local 4
      i32.const 8
      i32.add
      i64.load align=4
      i64.store
      get_local 2
      get_local 4
      i64.load align=4
      i64.store offset=40
      get_local 2
      i32.const 36
      i32.add
      i32.const 1048708
      get_local 2
      i32.const 40
      i32.add
      call $_ZN4core3fmt5write17h3d72ff271d605c43E
      drop
      get_local 2
      i32.const 8
      i32.add
      i32.const 8
      i32.add
      tee_local 4
      get_local 2
      i32.load offset=32
      i32.store
      get_local 2
      get_local 2
      i64.load offset=24
      i64.store offset=8
      block  ;; label = @2
        get_local 1
        i32.load offset=4
        tee_local 5
        i32.eqz
        br_if 0 (;@2;)
        get_local 1
        i32.const 8
        i32.add
        i32.load
        tee_local 1
        i32.eqz
        br_if 0 (;@2;)
        get_local 5
        get_local 1
        i32.const 1
        call $__rust_dealloc
      end
      get_local 3
      get_local 2
      i64.load offset=8
      i64.store align=4
      get_local 3
      i32.const 8
      i32.add
      get_local 4
      i32.load
      i32.store
    end
    get_local 0
    i32.const 1048868
    i32.store offset=4
    get_local 0
    get_local 3
    i32.store
    get_local 2
    i32.const 64
    i32.add
    set_global 0)
  (func $rust_panic (type 0) (param i32 i32)
    (local i32)
    get_global 0
    i32.const 16
    i32.sub
    tee_local 2
    set_global 0
    get_local 2
    get_local 1
    i32.store offset=12
    get_local 2
    get_local 0
    i32.store offset=8
    get_local 2
    i32.const 8
    i32.add
    call $__rust_start_panic
    drop
    unreachable
    unreachable)
  (func $__rust_start_panic (type 3) (param i32) (result i32)
    unreachable
    unreachable)
  (func $_ZN8dlmalloc8dlmalloc8Dlmalloc16malloc_alignment17h35c3c2822c7fb378E (type 3) (param i32) (result i32)
    i32.const 8)
  (func $_ZN8dlmalloc8dlmalloc8Dlmalloc17calloc_must_clear17hc573cca86dc6fb14E (type 2) (param i32 i32) (result i32)
    get_local 1
    i32.const -4
    i32.add
    i32.load8_u
    i32.const 3
    i32.and
    i32.const 0
    i32.ne)
  (func $_ZN8dlmalloc8dlmalloc8Dlmalloc6malloc17ha394eee15885f4c7E (type 2) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i64)
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              get_local 1
              i32.const 245
              i32.lt_u
              br_if 0 (;@5;)
              i32.const 0
              set_local 2
              get_local 1
              i32.const -65587
              i32.ge_u
              br_if 4 (;@1;)
              get_local 1
              i32.const 11
              i32.add
              tee_local 1
              i32.const -8
              i32.and
              set_local 3
              get_local 0
              i32.const 4
              i32.add
              i32.load
              tee_local 4
              i32.eqz
              br_if 1 (;@4;)
              i32.const 0
              set_local 5
              block  ;; label = @6
                get_local 1
                i32.const 8
                i32.shr_u
                tee_local 1
                i32.eqz
                br_if 0 (;@6;)
                i32.const 31
                set_local 5
                get_local 3
                i32.const 16777215
                i32.gt_u
                br_if 0 (;@6;)
                get_local 3
                i32.const 6
                get_local 1
                i32.clz
                tee_local 1
                i32.sub
                i32.const 31
                i32.and
                i32.shr_u
                i32.const 1
                i32.and
                get_local 1
                i32.const 1
                i32.shl
                i32.sub
                i32.const 62
                i32.add
                set_local 5
              end
              i32.const 0
              get_local 3
              i32.sub
              set_local 2
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    get_local 0
                    get_local 5
                    i32.const 2
                    i32.shl
                    i32.add
                    i32.const 272
                    i32.add
                    i32.load
                    tee_local 1
                    i32.eqz
                    br_if 0 (;@8;)
                    i32.const 0
                    set_local 6
                    get_local 3
                    i32.const 0
                    i32.const 25
                    get_local 5
                    i32.const 1
                    i32.shr_u
                    i32.sub
                    i32.const 31
                    i32.and
                    get_local 5
                    i32.const 31
                    i32.eq
                    select
                    i32.shl
                    set_local 7
                    i32.const 0
                    set_local 8
                    loop  ;; label = @9
                      block  ;; label = @10
                        get_local 1
                        i32.const 4
                        i32.add
                        i32.load
                        i32.const -8
                        i32.and
                        tee_local 9
                        get_local 3
                        i32.lt_u
                        br_if 0 (;@10;)
                        get_local 9
                        get_local 3
                        i32.sub
                        tee_local 9
                        get_local 2
                        i32.ge_u
                        br_if 0 (;@10;)
                        get_local 9
                        set_local 2
                        get_local 1
                        set_local 8
                        get_local 9
                        br_if 0 (;@10;)
                        i32.const 0
                        set_local 2
                        get_local 1
                        set_local 8
                        br 3 (;@7;)
                      end
                      get_local 1
                      i32.const 20
                      i32.add
                      i32.load
                      tee_local 9
                      get_local 6
                      get_local 9
                      get_local 1
                      get_local 7
                      i32.const 29
                      i32.shr_u
                      i32.const 4
                      i32.and
                      i32.add
                      i32.const 16
                      i32.add
                      i32.load
                      tee_local 1
                      i32.ne
                      select
                      get_local 6
                      get_local 9
                      select
                      set_local 6
                      get_local 7
                      i32.const 1
                      i32.shl
                      set_local 7
                      get_local 1
                      br_if 0 (;@9;)
                    end
                    block  ;; label = @9
                      get_local 6
                      i32.eqz
                      br_if 0 (;@9;)
                      get_local 6
                      set_local 1
                      br 2 (;@7;)
                    end
                    get_local 8
                    br_if 2 (;@6;)
                  end
                  i32.const 0
                  set_local 8
                  i32.const 2
                  get_local 5
                  i32.const 31
                  i32.and
                  i32.shl
                  tee_local 1
                  i32.const 0
                  get_local 1
                  i32.sub
                  i32.or
                  get_local 4
                  i32.and
                  tee_local 1
                  i32.eqz
                  br_if 3 (;@4;)
                  get_local 0
                  get_local 1
                  i32.const 0
                  get_local 1
                  i32.sub
                  i32.and
                  i32.ctz
                  i32.const 2
                  i32.shl
                  i32.add
                  i32.const 272
                  i32.add
                  i32.load
                  tee_local 1
                  i32.eqz
                  br_if 3 (;@4;)
                end
                loop  ;; label = @7
                  get_local 1
                  i32.const 4
                  i32.add
                  i32.load
                  i32.const -8
                  i32.and
                  tee_local 6
                  get_local 3
                  i32.ge_u
                  get_local 6
                  get_local 3
                  i32.sub
                  tee_local 9
                  get_local 2
                  i32.lt_u
                  i32.and
                  set_local 7
                  block  ;; label = @8
                    get_local 1
                    i32.load offset=16
                    tee_local 6
                    br_if 0 (;@8;)
                    get_local 1
                    i32.const 20
                    i32.add
                    i32.load
                    set_local 6
                  end
                  get_local 1
                  get_local 8
                  get_local 7
                  select
                  set_local 8
                  get_local 9
                  get_local 2
                  get_local 7
                  select
                  set_local 2
                  get_local 6
                  set_local 1
                  get_local 6
                  br_if 0 (;@7;)
                end
                get_local 8
                i32.eqz
                br_if 2 (;@4;)
              end
              block  ;; label = @6
                get_local 0
                i32.load offset=400
                tee_local 1
                get_local 3
                i32.lt_u
                br_if 0 (;@6;)
                get_local 2
                get_local 1
                get_local 3
                i32.sub
                i32.ge_u
                br_if 2 (;@4;)
              end
              get_local 8
              i32.load offset=24
              set_local 5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    get_local 8
                    i32.load offset=12
                    tee_local 6
                    get_local 8
                    i32.ne
                    br_if 0 (;@8;)
                    get_local 8
                    i32.const 20
                    i32.const 16
                    get_local 8
                    i32.const 20
                    i32.add
                    tee_local 6
                    i32.load
                    tee_local 7
                    select
                    i32.add
                    i32.load
                    tee_local 1
                    br_if 1 (;@7;)
                    i32.const 0
                    set_local 6
                    br 2 (;@6;)
                  end
                  get_local 8
                  i32.load offset=8
                  tee_local 1
                  get_local 6
                  i32.store offset=12
                  get_local 6
                  get_local 1
                  i32.store offset=8
                  br 1 (;@6;)
                end
                get_local 6
                get_local 8
                i32.const 16
                i32.add
                get_local 7
                select
                set_local 7
                loop  ;; label = @7
                  get_local 7
                  set_local 9
                  block  ;; label = @8
                    get_local 1
                    tee_local 6
                    i32.const 20
                    i32.add
                    tee_local 7
                    i32.load
                    tee_local 1
                    br_if 0 (;@8;)
                    get_local 6
                    i32.const 16
                    i32.add
                    set_local 7
                    get_local 6
                    i32.load offset=16
                    set_local 1
                  end
                  get_local 1
                  br_if 0 (;@7;)
                end
                get_local 9
                i32.const 0
                i32.store
              end
              block  ;; label = @6
                get_local 5
                i32.eqz
                br_if 0 (;@6;)
                block  ;; label = @7
                  block  ;; label = @8
                    get_local 0
                    get_local 8
                    i32.load offset=28
                    i32.const 2
                    i32.shl
                    i32.add
                    i32.const 272
                    i32.add
                    tee_local 1
                    i32.load
                    get_local 8
                    i32.eq
                    br_if 0 (;@8;)
                    get_local 5
                    i32.const 16
                    i32.const 20
                    get_local 5
                    i32.load offset=16
                    get_local 8
                    i32.eq
                    select
                    i32.add
                    get_local 6
                    i32.store
                    get_local 6
                    i32.eqz
                    br_if 2 (;@6;)
                    br 1 (;@7;)
                  end
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 6
                  br_if 0 (;@7;)
                  get_local 0
                  i32.const 4
                  i32.add
                  tee_local 1
                  get_local 1
                  i32.load
                  i32.const -2
                  get_local 8
                  i32.load offset=28
                  i32.rotl
                  i32.and
                  i32.store
                  br 1 (;@6;)
                end
                get_local 6
                get_local 5
                i32.store offset=24
                block  ;; label = @7
                  get_local 8
                  i32.load offset=16
                  tee_local 1
                  i32.eqz
                  br_if 0 (;@7;)
                  get_local 6
                  get_local 1
                  i32.store offset=16
                  get_local 1
                  get_local 6
                  i32.store offset=24
                end
                get_local 8
                i32.const 20
                i32.add
                i32.load
                tee_local 1
                i32.eqz
                br_if 0 (;@6;)
                get_local 6
                i32.const 20
                i32.add
                get_local 1
                i32.store
                get_local 1
                get_local 6
                i32.store offset=24
              end
              block  ;; label = @6
                block  ;; label = @7
                  get_local 2
                  i32.const 16
                  i32.lt_u
                  br_if 0 (;@7;)
                  get_local 8
                  get_local 3
                  i32.const 3
                  i32.or
                  i32.store offset=4
                  get_local 8
                  get_local 3
                  i32.add
                  tee_local 3
                  get_local 2
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  get_local 3
                  get_local 2
                  i32.add
                  get_local 2
                  i32.store
                  block  ;; label = @8
                    get_local 2
                    i32.const 256
                    i32.lt_u
                    br_if 0 (;@8;)
                    block  ;; label = @9
                      block  ;; label = @10
                        get_local 2
                        i32.const 8
                        i32.shr_u
                        tee_local 6
                        br_if 0 (;@10;)
                        i32.const 0
                        set_local 1
                        br 1 (;@9;)
                      end
                      i32.const 31
                      set_local 1
                      get_local 2
                      i32.const 16777215
                      i32.gt_u
                      br_if 0 (;@9;)
                      get_local 2
                      i32.const 6
                      get_local 6
                      i32.clz
                      tee_local 1
                      i32.sub
                      i32.const 31
                      i32.and
                      i32.shr_u
                      i32.const 1
                      i32.and
                      get_local 1
                      i32.const 1
                      i32.shl
                      i32.sub
                      i32.const 62
                      i32.add
                      set_local 1
                    end
                    get_local 3
                    i64.const 0
                    i64.store offset=16 align=4
                    get_local 3
                    get_local 1
                    i32.store offset=28
                    get_local 0
                    get_local 1
                    i32.const 2
                    i32.shl
                    i32.add
                    i32.const 272
                    i32.add
                    set_local 6
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              get_local 0
                              i32.const 4
                              i32.add
                              tee_local 7
                              i32.load
                              tee_local 9
                              i32.const 1
                              get_local 1
                              i32.const 31
                              i32.and
                              i32.shl
                              tee_local 0
                              i32.and
                              i32.eqz
                              br_if 0 (;@13;)
                              get_local 6
                              i32.load
                              tee_local 7
                              i32.const 4
                              i32.add
                              i32.load
                              i32.const -8
                              i32.and
                              get_local 2
                              i32.ne
                              br_if 1 (;@12;)
                              get_local 7
                              set_local 1
                              br 2 (;@11;)
                            end
                            get_local 7
                            get_local 9
                            get_local 0
                            i32.or
                            i32.store
                            get_local 6
                            get_local 3
                            i32.store
                            get_local 3
                            get_local 6
                            i32.store offset=24
                            br 3 (;@9;)
                          end
                          get_local 2
                          i32.const 0
                          i32.const 25
                          get_local 1
                          i32.const 1
                          i32.shr_u
                          i32.sub
                          i32.const 31
                          i32.and
                          get_local 1
                          i32.const 31
                          i32.eq
                          select
                          i32.shl
                          set_local 6
                          loop  ;; label = @12
                            get_local 7
                            get_local 6
                            i32.const 29
                            i32.shr_u
                            i32.const 4
                            i32.and
                            i32.add
                            i32.const 16
                            i32.add
                            tee_local 9
                            i32.load
                            tee_local 1
                            i32.eqz
                            br_if 2 (;@10;)
                            get_local 6
                            i32.const 1
                            i32.shl
                            set_local 6
                            get_local 1
                            set_local 7
                            get_local 1
                            i32.const 4
                            i32.add
                            i32.load
                            i32.const -8
                            i32.and
                            get_local 2
                            i32.ne
                            br_if 0 (;@12;)
                          end
                        end
                        get_local 1
                        i32.load offset=8
                        tee_local 2
                        get_local 3
                        i32.store offset=12
                        get_local 1
                        get_local 3
                        i32.store offset=8
                        get_local 3
                        i32.const 0
                        i32.store offset=24
                        get_local 3
                        get_local 1
                        i32.store offset=12
                        get_local 3
                        get_local 2
                        i32.store offset=8
                        br 4 (;@6;)
                      end
                      get_local 9
                      get_local 3
                      i32.store
                      get_local 3
                      get_local 7
                      i32.store offset=24
                    end
                    get_local 3
                    get_local 3
                    i32.store offset=12
                    get_local 3
                    get_local 3
                    i32.store offset=8
                    br 2 (;@6;)
                  end
                  get_local 0
                  get_local 2
                  i32.const 3
                  i32.shr_u
                  tee_local 2
                  i32.const 3
                  i32.shl
                  i32.add
                  i32.const 8
                  i32.add
                  set_local 1
                  block  ;; label = @8
                    block  ;; label = @9
                      get_local 0
                      i32.load
                      tee_local 6
                      i32.const 1
                      get_local 2
                      i32.const 31
                      i32.and
                      i32.shl
                      tee_local 2
                      i32.and
                      i32.eqz
                      br_if 0 (;@9;)
                      get_local 1
                      i32.load offset=8
                      set_local 2
                      br 1 (;@8;)
                    end
                    get_local 0
                    get_local 6
                    get_local 2
                    i32.or
                    i32.store
                    get_local 1
                    set_local 2
                  end
                  get_local 1
                  get_local 3
                  i32.store offset=8
                  get_local 2
                  get_local 3
                  i32.store offset=12
                  get_local 3
                  get_local 1
                  i32.store offset=12
                  get_local 3
                  get_local 2
                  i32.store offset=8
                  br 1 (;@6;)
                end
                get_local 8
                get_local 2
                get_local 3
                i32.add
                tee_local 1
                i32.const 3
                i32.or
                i32.store offset=4
                get_local 8
                get_local 1
                i32.add
                tee_local 1
                get_local 1
                i32.load offset=4
                i32.const 1
                i32.or
                i32.store offset=4
              end
              get_local 8
              i32.const 8
              i32.add
              return
            end
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  get_local 0
                  i32.load
                  tee_local 8
                  i32.const 16
                  get_local 1
                  i32.const 11
                  i32.add
                  i32.const -8
                  i32.and
                  get_local 1
                  i32.const 11
                  i32.lt_u
                  select
                  tee_local 3
                  i32.const 3
                  i32.shr_u
                  tee_local 2
                  i32.const 31
                  i32.and
                  tee_local 6
                  i32.shr_u
                  tee_local 1
                  i32.const 3
                  i32.and
                  br_if 0 (;@7;)
                  get_local 3
                  get_local 0
                  i32.load offset=400
                  i32.le_u
                  br_if 3 (;@4;)
                  get_local 1
                  br_if 1 (;@6;)
                  get_local 0
                  i32.load offset=4
                  tee_local 1
                  i32.eqz
                  br_if 3 (;@4;)
                  get_local 0
                  get_local 1
                  i32.const 0
                  get_local 1
                  i32.sub
                  i32.and
                  i32.ctz
                  i32.const 2
                  i32.shl
                  i32.add
                  i32.const 272
                  i32.add
                  i32.load
                  tee_local 6
                  i32.const 4
                  i32.add
                  i32.load
                  i32.const -8
                  i32.and
                  get_local 3
                  i32.sub
                  set_local 2
                  get_local 6
                  set_local 7
                  loop  ;; label = @8
                    block  ;; label = @9
                      get_local 6
                      i32.load offset=16
                      tee_local 1
                      br_if 0 (;@9;)
                      get_local 6
                      i32.const 20
                      i32.add
                      i32.load
                      tee_local 1
                      i32.eqz
                      br_if 4 (;@5;)
                    end
                    get_local 1
                    i32.const 4
                    i32.add
                    i32.load
                    i32.const -8
                    i32.and
                    get_local 3
                    i32.sub
                    tee_local 6
                    get_local 2
                    get_local 6
                    get_local 2
                    i32.lt_u
                    tee_local 6
                    select
                    set_local 2
                    get_local 1
                    get_local 7
                    get_local 6
                    select
                    set_local 7
                    get_local 1
                    set_local 6
                    br 0 (;@8;)
                  end
                end
                block  ;; label = @7
                  block  ;; label = @8
                    get_local 0
                    get_local 1
                    i32.const -1
                    i32.xor
                    i32.const 1
                    i32.and
                    get_local 2
                    i32.add
                    tee_local 3
                    i32.const 3
                    i32.shl
                    i32.add
                    tee_local 7
                    i32.const 16
                    i32.add
                    i32.load
                    tee_local 1
                    i32.const 8
                    i32.add
                    tee_local 2
                    i32.load
                    tee_local 6
                    get_local 7
                    i32.const 8
                    i32.add
                    tee_local 7
                    i32.eq
                    br_if 0 (;@8;)
                    get_local 6
                    get_local 7
                    i32.store offset=12
                    get_local 7
                    get_local 6
                    i32.store offset=8
                    br 1 (;@7;)
                  end
                  get_local 0
                  get_local 8
                  i32.const -2
                  get_local 3
                  i32.rotl
                  i32.and
                  i32.store
                end
                get_local 1
                get_local 3
                i32.const 3
                i32.shl
                tee_local 3
                i32.const 3
                i32.or
                i32.store offset=4
                get_local 1
                get_local 3
                i32.add
                tee_local 1
                get_local 1
                i32.load offset=4
                i32.const 1
                i32.or
                i32.store offset=4
                br 5 (;@1;)
              end
              block  ;; label = @6
                block  ;; label = @7
                  get_local 0
                  get_local 1
                  get_local 6
                  i32.shl
                  i32.const 2
                  get_local 6
                  i32.shl
                  tee_local 1
                  i32.const 0
                  get_local 1
                  i32.sub
                  i32.or
                  i32.and
                  tee_local 1
                  i32.const 0
                  get_local 1
                  i32.sub
                  i32.and
                  i32.ctz
                  tee_local 2
                  i32.const 3
                  i32.shl
                  i32.add
                  tee_local 7
                  i32.const 16
                  i32.add
                  i32.load
                  tee_local 1
                  i32.const 8
                  i32.add
                  tee_local 9
                  i32.load
                  tee_local 6
                  get_local 7
                  i32.const 8
                  i32.add
                  tee_local 7
                  i32.eq
                  br_if 0 (;@7;)
                  get_local 6
                  get_local 7
                  i32.store offset=12
                  get_local 7
                  get_local 6
                  i32.store offset=8
                  br 1 (;@6;)
                end
                get_local 0
                get_local 8
                i32.const -2
                get_local 2
                i32.rotl
                i32.and
                i32.store
              end
              get_local 1
              get_local 3
              i32.const 3
              i32.or
              i32.store offset=4
              get_local 1
              get_local 3
              i32.add
              tee_local 6
              get_local 2
              i32.const 3
              i32.shl
              tee_local 2
              get_local 3
              i32.sub
              tee_local 3
              i32.const 1
              i32.or
              i32.store offset=4
              get_local 1
              get_local 2
              i32.add
              get_local 3
              i32.store
              block  ;; label = @6
                get_local 0
                i32.load offset=400
                tee_local 1
                i32.eqz
                br_if 0 (;@6;)
                get_local 0
                get_local 1
                i32.const 3
                i32.shr_u
                tee_local 7
                i32.const 3
                i32.shl
                i32.add
                i32.const 8
                i32.add
                set_local 2
                get_local 0
                i32.load offset=408
                set_local 1
                block  ;; label = @7
                  block  ;; label = @8
                    get_local 0
                    i32.load
                    tee_local 8
                    i32.const 1
                    get_local 7
                    i32.const 31
                    i32.and
                    i32.shl
                    tee_local 7
                    i32.and
                    i32.eqz
                    br_if 0 (;@8;)
                    get_local 2
                    i32.load offset=8
                    set_local 7
                    br 1 (;@7;)
                  end
                  get_local 0
                  get_local 8
                  get_local 7
                  i32.or
                  i32.store
                  get_local 2
                  set_local 7
                end
                get_local 2
                get_local 1
                i32.store offset=8
                get_local 7
                get_local 1
                i32.store offset=12
                get_local 1
                get_local 2
                i32.store offset=12
                get_local 1
                get_local 7
                i32.store offset=8
              end
              get_local 0
              get_local 6
              i32.store offset=408
              get_local 0
              get_local 3
              i32.store offset=400
              get_local 9
              return
            end
            get_local 7
            i32.load offset=24
            set_local 5
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  get_local 7
                  i32.load offset=12
                  tee_local 6
                  get_local 7
                  i32.ne
                  br_if 0 (;@7;)
                  get_local 7
                  i32.const 20
                  i32.const 16
                  get_local 7
                  i32.const 20
                  i32.add
                  tee_local 6
                  i32.load
                  tee_local 8
                  select
                  i32.add
                  i32.load
                  tee_local 1
                  br_if 1 (;@6;)
                  i32.const 0
                  set_local 6
                  br 2 (;@5;)
                end
                get_local 7
                i32.load offset=8
                tee_local 1
                get_local 6
                i32.store offset=12
                get_local 6
                get_local 1
                i32.store offset=8
                br 1 (;@5;)
              end
              get_local 6
              get_local 7
              i32.const 16
              i32.add
              get_local 8
              select
              set_local 8
              loop  ;; label = @6
                get_local 8
                set_local 9
                block  ;; label = @7
                  get_local 1
                  tee_local 6
                  i32.const 20
                  i32.add
                  tee_local 8
                  i32.load
                  tee_local 1
                  br_if 0 (;@7;)
                  get_local 6
                  i32.const 16
                  i32.add
                  set_local 8
                  get_local 6
                  i32.load offset=16
                  set_local 1
                end
                get_local 1
                br_if 0 (;@6;)
              end
              get_local 9
              i32.const 0
              i32.store
            end
            get_local 5
            i32.eqz
            br_if 2 (;@2;)
            block  ;; label = @5
              get_local 0
              get_local 7
              i32.load offset=28
              i32.const 2
              i32.shl
              i32.add
              i32.const 272
              i32.add
              tee_local 1
              i32.load
              get_local 7
              i32.eq
              br_if 0 (;@5;)
              get_local 5
              i32.const 16
              i32.const 20
              get_local 5
              i32.load offset=16
              get_local 7
              i32.eq
              select
              i32.add
              get_local 6
              i32.store
              get_local 6
              i32.eqz
              br_if 3 (;@2;)
              br 2 (;@3;)
            end
            get_local 1
            get_local 6
            i32.store
            get_local 6
            br_if 1 (;@3;)
            get_local 0
            get_local 0
            i32.load offset=4
            i32.const -2
            get_local 7
            i32.load offset=28
            i32.rotl
            i32.and
            i32.store offset=4
            br 2 (;@2;)
          end
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      get_local 0
                      i32.load offset=400
                      tee_local 1
                      get_local 3
                      i32.ge_u
                      br_if 0 (;@9;)
                      get_local 0
                      i32.load offset=404
                      tee_local 1
                      get_local 3
                      i32.gt_u
                      br_if 3 (;@6;)
                      i32.const 0
                      set_local 2
                      get_local 3
                      i32.const 65583
                      i32.add
                      tee_local 6
                      i32.const 16
                      i32.shr_u
                      memory.grow
                      tee_local 1
                      i32.const -1
                      i32.eq
                      br_if 8 (;@1;)
                      get_local 1
                      i32.const 16
                      i32.shl
                      tee_local 8
                      i32.eqz
                      br_if 8 (;@1;)
                      get_local 0
                      get_local 0
                      i32.load offset=416
                      get_local 6
                      i32.const -65536
                      i32.and
                      tee_local 5
                      i32.add
                      tee_local 1
                      i32.store offset=416
                      get_local 0
                      get_local 0
                      i32.load offset=420
                      tee_local 6
                      get_local 1
                      get_local 6
                      get_local 1
                      i32.gt_u
                      select
                      i32.store offset=420
                      get_local 0
                      i32.load offset=412
                      tee_local 6
                      i32.eqz
                      br_if 1 (;@8;)
                      get_local 0
                      i32.const 424
                      i32.add
                      tee_local 4
                      set_local 1
                      loop  ;; label = @10
                        get_local 1
                        i32.load
                        tee_local 7
                        get_local 1
                        i32.load offset=4
                        tee_local 9
                        i32.add
                        get_local 8
                        i32.eq
                        br_if 3 (;@7;)
                        get_local 1
                        i32.load offset=8
                        tee_local 1
                        br_if 0 (;@10;)
                        br 5 (;@5;)
                      end
                    end
                    get_local 0
                    i32.load offset=408
                    set_local 2
                    block  ;; label = @9
                      block  ;; label = @10
                        get_local 1
                        get_local 3
                        i32.sub
                        tee_local 6
                        i32.const 15
                        i32.gt_u
                        br_if 0 (;@10;)
                        get_local 0
                        i32.const 0
                        i32.store offset=408
                        get_local 0
                        i32.const 0
                        i32.store offset=400
                        get_local 2
                        get_local 1
                        i32.const 3
                        i32.or
                        i32.store offset=4
                        get_local 2
                        get_local 1
                        i32.add
                        tee_local 3
                        i32.const 4
                        i32.add
                        set_local 1
                        get_local 3
                        i32.load offset=4
                        i32.const 1
                        i32.or
                        set_local 3
                        br 1 (;@9;)
                      end
                      get_local 0
                      get_local 6
                      i32.store offset=400
                      get_local 0
                      get_local 2
                      get_local 3
                      i32.add
                      tee_local 7
                      i32.store offset=408
                      get_local 7
                      get_local 6
                      i32.const 1
                      i32.or
                      i32.store offset=4
                      get_local 2
                      get_local 1
                      i32.add
                      get_local 6
                      i32.store
                      get_local 3
                      i32.const 3
                      i32.or
                      set_local 3
                      get_local 2
                      i32.const 4
                      i32.add
                      set_local 1
                    end
                    get_local 1
                    get_local 3
                    i32.store
                    get_local 2
                    i32.const 8
                    i32.add
                    return
                  end
                  block  ;; label = @8
                    block  ;; label = @9
                      get_local 0
                      i32.load offset=444
                      tee_local 1
                      i32.eqz
                      br_if 0 (;@9;)
                      get_local 1
                      get_local 8
                      i32.le_u
                      br_if 1 (;@8;)
                    end
                    get_local 0
                    get_local 8
                    i32.store offset=444
                  end
                  get_local 0
                  i32.const 4095
                  i32.store offset=448
                  get_local 0
                  get_local 8
                  i32.store offset=424
                  get_local 0
                  i32.const 436
                  i32.add
                  i32.const 0
                  i32.store
                  get_local 0
                  i32.const 428
                  i32.add
                  get_local 5
                  i32.store
                  get_local 0
                  i32.const 20
                  i32.add
                  get_local 0
                  i32.const 8
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 0
                  i32.const 28
                  i32.add
                  get_local 0
                  i32.const 16
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 36
                  i32.add
                  get_local 0
                  i32.const 24
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 44
                  i32.add
                  get_local 0
                  i32.const 32
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 52
                  i32.add
                  get_local 0
                  i32.const 40
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 60
                  i32.add
                  get_local 0
                  i32.const 48
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 68
                  i32.add
                  get_local 0
                  i32.const 56
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 76
                  i32.add
                  get_local 0
                  i32.const 64
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 84
                  i32.add
                  get_local 0
                  i32.const 72
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 80
                  i32.add
                  tee_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 92
                  i32.add
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 88
                  i32.add
                  tee_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 100
                  i32.add
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 96
                  i32.add
                  tee_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 108
                  i32.add
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 104
                  i32.add
                  tee_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 116
                  i32.add
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 112
                  i32.add
                  tee_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 124
                  i32.add
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 120
                  i32.add
                  tee_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 132
                  i32.add
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 128
                  i32.add
                  tee_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 140
                  i32.add
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 136
                  i32.add
                  tee_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 148
                  i32.add
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 156
                  i32.add
                  get_local 0
                  i32.const 144
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 164
                  i32.add
                  get_local 0
                  i32.const 152
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 172
                  i32.add
                  get_local 0
                  i32.const 160
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 180
                  i32.add
                  get_local 0
                  i32.const 168
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 188
                  i32.add
                  get_local 0
                  i32.const 176
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 196
                  i32.add
                  get_local 0
                  i32.const 184
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 204
                  i32.add
                  get_local 0
                  i32.const 192
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 212
                  i32.add
                  get_local 0
                  i32.const 200
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 220
                  i32.add
                  get_local 0
                  i32.const 208
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 228
                  i32.add
                  get_local 0
                  i32.const 216
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 236
                  i32.add
                  get_local 0
                  i32.const 224
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 244
                  i32.add
                  get_local 0
                  i32.const 232
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 252
                  i32.add
                  get_local 0
                  i32.const 240
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  i32.const 260
                  i32.add
                  get_local 0
                  i32.const 248
                  i32.add
                  tee_local 6
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store
                  get_local 0
                  i32.const 268
                  i32.add
                  get_local 0
                  i32.const 256
                  i32.add
                  tee_local 1
                  i32.store
                  get_local 1
                  get_local 6
                  i32.store
                  get_local 0
                  get_local 8
                  i32.store offset=412
                  get_local 0
                  i32.const 264
                  i32.add
                  get_local 1
                  i32.store
                  get_local 0
                  get_local 5
                  i32.const -40
                  i32.add
                  tee_local 1
                  i32.store offset=404
                  get_local 8
                  get_local 1
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  get_local 8
                  get_local 1
                  i32.add
                  i32.const 40
                  i32.store offset=4
                  get_local 0
                  i32.const 2097152
                  i32.store offset=440
                  br 3 (;@4;)
                end
                get_local 1
                i32.const 12
                i32.add
                i32.load
                br_if 1 (;@5;)
                get_local 8
                get_local 6
                i32.le_u
                br_if 1 (;@5;)
                get_local 7
                get_local 6
                i32.gt_u
                br_if 1 (;@5;)
                get_local 1
                get_local 9
                get_local 5
                i32.add
                i32.store offset=4
                get_local 0
                get_local 0
                i32.load offset=412
                tee_local 1
                i32.const 15
                i32.add
                i32.const -8
                i32.and
                tee_local 6
                i32.const -8
                i32.add
                i32.store offset=412
                get_local 0
                get_local 1
                get_local 6
                i32.sub
                get_local 0
                i32.load offset=404
                get_local 5
                i32.add
                tee_local 7
                i32.add
                i32.const 8
                i32.add
                tee_local 8
                i32.store offset=404
                get_local 6
                i32.const -4
                i32.add
                get_local 8
                i32.const 1
                i32.or
                i32.store
                get_local 1
                get_local 7
                i32.add
                i32.const 40
                i32.store offset=4
                get_local 0
                i32.const 2097152
                i32.store offset=440
                br 2 (;@4;)
              end
              get_local 0
              get_local 1
              get_local 3
              i32.sub
              tee_local 2
              i32.store offset=404
              get_local 0
              get_local 0
              i32.load offset=412
              tee_local 1
              get_local 3
              i32.add
              tee_local 6
              i32.store offset=412
              get_local 6
              get_local 2
              i32.const 1
              i32.or
              i32.store offset=4
              get_local 1
              get_local 3
              i32.const 3
              i32.or
              i32.store offset=4
              get_local 1
              i32.const 8
              i32.add
              return
            end
            get_local 0
            get_local 0
            i32.load offset=444
            tee_local 1
            get_local 8
            get_local 1
            get_local 8
            i32.lt_u
            select
            i32.store offset=444
            get_local 8
            get_local 5
            i32.add
            set_local 7
            get_local 4
            set_local 1
            block  ;; label = @5
              block  ;; label = @6
                loop  ;; label = @7
                  get_local 1
                  i32.load
                  get_local 7
                  i32.eq
                  br_if 1 (;@6;)
                  get_local 1
                  i32.load offset=8
                  tee_local 1
                  br_if 0 (;@7;)
                  br 2 (;@5;)
                end
              end
              get_local 1
              i32.const 12
              i32.add
              i32.load
              br_if 0 (;@5;)
              get_local 1
              get_local 8
              i32.store
              get_local 1
              get_local 1
              i32.load offset=4
              get_local 5
              i32.add
              i32.store offset=4
              get_local 8
              get_local 3
              i32.const 3
              i32.or
              i32.store offset=4
              get_local 8
              get_local 3
              i32.add
              set_local 1
              get_local 7
              get_local 8
              i32.sub
              get_local 3
              i32.sub
              set_local 3
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    get_local 0
                    i32.load offset=412
                    get_local 7
                    i32.eq
                    br_if 0 (;@8;)
                    get_local 0
                    i32.load offset=408
                    get_local 7
                    i32.eq
                    br_if 1 (;@7;)
                    block  ;; label = @9
                      get_local 7
                      i32.const 4
                      i32.add
                      i32.load
                      tee_local 2
                      i32.const 3
                      i32.and
                      i32.const 1
                      i32.ne
                      br_if 0 (;@9;)
                      get_local 0
                      get_local 7
                      get_local 2
                      i32.const -8
                      i32.and
                      tee_local 2
                      call $_ZN8dlmalloc8dlmalloc8Dlmalloc12unlink_chunk17hd0fb28e586d177f0E
                      get_local 2
                      get_local 3
                      i32.add
                      set_local 3
                      get_local 7
                      get_local 2
                      i32.add
                      set_local 7
                    end
                    get_local 7
                    get_local 7
                    i32.load offset=4
                    i32.const -2
                    i32.and
                    i32.store offset=4
                    get_local 1
                    get_local 3
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    get_local 1
                    get_local 3
                    i32.add
                    get_local 3
                    i32.store
                    block  ;; label = @9
                      get_local 3
                      i32.const 256
                      i32.lt_u
                      br_if 0 (;@9;)
                      block  ;; label = @10
                        block  ;; label = @11
                          get_local 3
                          i32.const 8
                          i32.shr_u
                          tee_local 6
                          br_if 0 (;@11;)
                          i32.const 0
                          set_local 2
                          br 1 (;@10;)
                        end
                        i32.const 31
                        set_local 2
                        get_local 3
                        i32.const 16777215
                        i32.gt_u
                        br_if 0 (;@10;)
                        get_local 3
                        i32.const 6
                        get_local 6
                        i32.clz
                        tee_local 2
                        i32.sub
                        i32.const 31
                        i32.and
                        i32.shr_u
                        i32.const 1
                        i32.and
                        get_local 2
                        i32.const 1
                        i32.shl
                        i32.sub
                        i32.const 62
                        i32.add
                        set_local 2
                      end
                      get_local 1
                      i64.const 0
                      i64.store offset=16 align=4
                      get_local 1
                      get_local 2
                      i32.store offset=28
                      get_local 0
                      get_local 2
                      i32.const 2
                      i32.shl
                      i32.add
                      i32.const 272
                      i32.add
                      set_local 6
                      block  ;; label = @10
                        block  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                get_local 0
                                i32.const 4
                                i32.add
                                tee_local 7
                                i32.load
                                tee_local 9
                                i32.const 1
                                get_local 2
                                i32.const 31
                                i32.and
                                i32.shl
                                tee_local 0
                                i32.and
                                i32.eqz
                                br_if 0 (;@14;)
                                get_local 6
                                i32.load
                                tee_local 7
                                i32.const 4
                                i32.add
                                i32.load
                                i32.const -8
                                i32.and
                                get_local 3
                                i32.ne
                                br_if 1 (;@13;)
                                get_local 7
                                set_local 2
                                br 2 (;@12;)
                              end
                              get_local 7
                              get_local 9
                              get_local 0
                              i32.or
                              i32.store
                              get_local 6
                              get_local 1
                              i32.store
                              get_local 1
                              get_local 6
                              i32.store offset=24
                              br 3 (;@10;)
                            end
                            get_local 3
                            i32.const 0
                            i32.const 25
                            get_local 2
                            i32.const 1
                            i32.shr_u
                            i32.sub
                            i32.const 31
                            i32.and
                            get_local 2
                            i32.const 31
                            i32.eq
                            select
                            i32.shl
                            set_local 6
                            loop  ;; label = @13
                              get_local 7
                              get_local 6
                              i32.const 29
                              i32.shr_u
                              i32.const 4
                              i32.and
                              i32.add
                              i32.const 16
                              i32.add
                              tee_local 9
                              i32.load
                              tee_local 2
                              i32.eqz
                              br_if 2 (;@11;)
                              get_local 6
                              i32.const 1
                              i32.shl
                              set_local 6
                              get_local 2
                              set_local 7
                              get_local 2
                              i32.const 4
                              i32.add
                              i32.load
                              i32.const -8
                              i32.and
                              get_local 3
                              i32.ne
                              br_if 0 (;@13;)
                            end
                          end
                          get_local 2
                          i32.load offset=8
                          tee_local 3
                          get_local 1
                          i32.store offset=12
                          get_local 2
                          get_local 1
                          i32.store offset=8
                          get_local 1
                          i32.const 0
                          i32.store offset=24
                          get_local 1
                          get_local 2
                          i32.store offset=12
                          get_local 1
                          get_local 3
                          i32.store offset=8
                          br 5 (;@6;)
                        end
                        get_local 9
                        get_local 1
                        i32.store
                        get_local 1
                        get_local 7
                        i32.store offset=24
                      end
                      get_local 1
                      get_local 1
                      i32.store offset=12
                      get_local 1
                      get_local 1
                      i32.store offset=8
                      br 3 (;@6;)
                    end
                    get_local 0
                    get_local 3
                    i32.const 3
                    i32.shr_u
                    tee_local 2
                    i32.const 3
                    i32.shl
                    i32.add
                    i32.const 8
                    i32.add
                    set_local 3
                    block  ;; label = @9
                      block  ;; label = @10
                        get_local 0
                        i32.load
                        tee_local 6
                        i32.const 1
                        get_local 2
                        i32.const 31
                        i32.and
                        i32.shl
                        tee_local 2
                        i32.and
                        i32.eqz
                        br_if 0 (;@10;)
                        get_local 3
                        i32.load offset=8
                        set_local 2
                        br 1 (;@9;)
                      end
                      get_local 0
                      get_local 6
                      get_local 2
                      i32.or
                      i32.store
                      get_local 3
                      set_local 2
                    end
                    get_local 3
                    get_local 1
                    i32.store offset=8
                    get_local 2
                    get_local 1
                    i32.store offset=12
                    get_local 1
                    get_local 3
                    i32.store offset=12
                    get_local 1
                    get_local 2
                    i32.store offset=8
                    br 2 (;@6;)
                  end
                  get_local 0
                  get_local 1
                  i32.store offset=412
                  get_local 0
                  get_local 0
                  i32.load offset=404
                  get_local 3
                  i32.add
                  tee_local 3
                  i32.store offset=404
                  get_local 1
                  get_local 3
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  br 1 (;@6;)
                end
                get_local 0
                get_local 1
                i32.store offset=408
                get_local 0
                get_local 0
                i32.load offset=400
                get_local 3
                i32.add
                tee_local 3
                i32.store offset=400
                get_local 1
                get_local 3
                i32.const 1
                i32.or
                i32.store offset=4
                get_local 1
                get_local 3
                i32.add
                get_local 3
                i32.store
              end
              get_local 8
              i32.const 8
              i32.add
              return
            end
            get_local 4
            set_local 1
            block  ;; label = @5
              loop  ;; label = @6
                block  ;; label = @7
                  get_local 1
                  i32.load
                  tee_local 7
                  get_local 6
                  i32.gt_u
                  br_if 0 (;@7;)
                  get_local 7
                  get_local 1
                  i32.load offset=4
                  i32.add
                  tee_local 7
                  get_local 6
                  i32.gt_u
                  br_if 2 (;@5;)
                end
                get_local 1
                i32.load offset=8
                set_local 1
                br 0 (;@6;)
              end
            end
            get_local 0
            get_local 8
            i32.store offset=412
            get_local 0
            get_local 5
            i32.const -40
            i32.add
            tee_local 1
            i32.store offset=404
            get_local 8
            get_local 1
            i32.const 1
            i32.or
            i32.store offset=4
            get_local 8
            get_local 1
            i32.add
            i32.const 40
            i32.store offset=4
            get_local 0
            i32.const 2097152
            i32.store offset=440
            get_local 6
            get_local 7
            i32.const -32
            i32.add
            i32.const -8
            i32.and
            i32.const -8
            i32.add
            tee_local 1
            get_local 1
            get_local 6
            i32.const 16
            i32.add
            i32.lt_u
            select
            tee_local 9
            i32.const 27
            i32.store offset=4
            get_local 4
            i64.load align=4
            set_local 10
            get_local 9
            i32.const 16
            i32.add
            get_local 4
            i32.const 8
            i32.add
            i64.load align=4
            i64.store align=4
            get_local 9
            get_local 10
            i64.store offset=8 align=4
            get_local 0
            i32.const 436
            i32.add
            i32.const 0
            i32.store
            get_local 0
            i32.const 428
            i32.add
            get_local 5
            i32.store
            get_local 0
            get_local 8
            i32.store offset=424
            get_local 0
            i32.const 432
            i32.add
            get_local 9
            i32.const 8
            i32.add
            i32.store
            get_local 9
            i32.const 28
            i32.add
            set_local 1
            loop  ;; label = @5
              get_local 1
              i32.const 7
              i32.store
              get_local 7
              get_local 1
              i32.const 4
              i32.add
              tee_local 1
              i32.gt_u
              br_if 0 (;@5;)
            end
            get_local 9
            get_local 6
            i32.eq
            br_if 0 (;@4;)
            get_local 9
            get_local 9
            i32.load offset=4
            i32.const -2
            i32.and
            i32.store offset=4
            get_local 6
            get_local 9
            get_local 6
            i32.sub
            tee_local 8
            i32.const 1
            i32.or
            i32.store offset=4
            get_local 9
            get_local 8
            i32.store
            block  ;; label = @5
              get_local 8
              i32.const 256
              i32.lt_u
              br_if 0 (;@5;)
              block  ;; label = @6
                block  ;; label = @7
                  get_local 8
                  i32.const 8
                  i32.shr_u
                  tee_local 7
                  br_if 0 (;@7;)
                  i32.const 0
                  set_local 1
                  br 1 (;@6;)
                end
                i32.const 31
                set_local 1
                get_local 8
                i32.const 16777215
                i32.gt_u
                br_if 0 (;@6;)
                get_local 8
                i32.const 6
                get_local 7
                i32.clz
                tee_local 1
                i32.sub
                i32.const 31
                i32.and
                i32.shr_u
                i32.const 1
                i32.and
                get_local 1
                i32.const 1
                i32.shl
                i32.sub
                i32.const 62
                i32.add
                set_local 1
              end
              get_local 6
              i64.const 0
              i64.store offset=16 align=4
              get_local 6
              i32.const 28
              i32.add
              get_local 1
              i32.store
              get_local 0
              get_local 1
              i32.const 2
              i32.shl
              i32.add
              i32.const 272
              i32.add
              set_local 7
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        get_local 0
                        i32.const 4
                        i32.add
                        tee_local 9
                        i32.load
                        tee_local 5
                        i32.const 1
                        get_local 1
                        i32.const 31
                        i32.and
                        i32.shl
                        tee_local 4
                        i32.and
                        i32.eqz
                        br_if 0 (;@10;)
                        get_local 7
                        i32.load
                        tee_local 9
                        i32.const 4
                        i32.add
                        i32.load
                        i32.const -8
                        i32.and
                        get_local 8
                        i32.ne
                        br_if 1 (;@9;)
                        get_local 9
                        set_local 1
                        br 2 (;@8;)
                      end
                      get_local 9
                      get_local 5
                      get_local 4
                      i32.or
                      i32.store
                      get_local 7
                      get_local 6
                      i32.store
                      get_local 6
                      i32.const 24
                      i32.add
                      get_local 7
                      i32.store
                      br 3 (;@6;)
                    end
                    get_local 8
                    i32.const 0
                    i32.const 25
                    get_local 1
                    i32.const 1
                    i32.shr_u
                    i32.sub
                    i32.const 31
                    i32.and
                    get_local 1
                    i32.const 31
                    i32.eq
                    select
                    i32.shl
                    set_local 7
                    loop  ;; label = @9
                      get_local 9
                      get_local 7
                      i32.const 29
                      i32.shr_u
                      i32.const 4
                      i32.and
                      i32.add
                      i32.const 16
                      i32.add
                      tee_local 5
                      i32.load
                      tee_local 1
                      i32.eqz
                      br_if 2 (;@7;)
                      get_local 7
                      i32.const 1
                      i32.shl
                      set_local 7
                      get_local 1
                      set_local 9
                      get_local 1
                      i32.const 4
                      i32.add
                      i32.load
                      i32.const -8
                      i32.and
                      get_local 8
                      i32.ne
                      br_if 0 (;@9;)
                    end
                  end
                  get_local 1
                  i32.load offset=8
                  tee_local 7
                  get_local 6
                  i32.store offset=12
                  get_local 1
                  get_local 6
                  i32.store offset=8
                  get_local 6
                  i32.const 24
                  i32.add
                  i32.const 0
                  i32.store
                  get_local 6
                  get_local 1
                  i32.store offset=12
                  get_local 6
                  get_local 7
                  i32.store offset=8
                  br 3 (;@4;)
                end
                get_local 5
                get_local 6
                i32.store
                get_local 6
                i32.const 24
                i32.add
                get_local 9
                i32.store
              end
              get_local 6
              get_local 6
              i32.store offset=12
              get_local 6
              get_local 6
              i32.store offset=8
              br 1 (;@4;)
            end
            get_local 0
            get_local 8
            i32.const 3
            i32.shr_u
            tee_local 7
            i32.const 3
            i32.shl
            i32.add
            i32.const 8
            i32.add
            set_local 1
            block  ;; label = @5
              block  ;; label = @6
                get_local 0
                i32.load
                tee_local 8
                i32.const 1
                get_local 7
                i32.const 31
                i32.and
                i32.shl
                tee_local 7
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                get_local 1
                i32.load offset=8
                set_local 7
                br 1 (;@5;)
              end
              get_local 0
              get_local 8
              get_local 7
              i32.or
              i32.store
              get_local 1
              set_local 7
            end
            get_local 1
            get_local 6
            i32.store offset=8
            get_local 7
            get_local 6
            i32.store offset=12
            get_local 6
            get_local 1
            i32.store offset=12
            get_local 6
            get_local 7
            i32.store offset=8
          end
          get_local 0
          i32.load offset=404
          tee_local 1
          get_local 3
          i32.le_u
          br_if 2 (;@1;)
          get_local 0
          get_local 1
          get_local 3
          i32.sub
          tee_local 2
          i32.store offset=404
          get_local 0
          get_local 0
          i32.load offset=412
          tee_local 1
          get_local 3
          i32.add
          tee_local 6
          i32.store offset=412
          get_local 6
          get_local 2
          i32.const 1
          i32.or
          i32.store offset=4
          get_local 1
          get_local 3
          i32.const 3
          i32.or
          i32.store offset=4
          get_local 1
          i32.const 8
          i32.add
          return
        end
        get_local 6
        get_local 5
        i32.store offset=24
        block  ;; label = @3
          get_local 7
          i32.load offset=16
          tee_local 1
          i32.eqz
          br_if 0 (;@3;)
          get_local 6
          get_local 1
          i32.store offset=16
          get_local 1
          get_local 6
          i32.store offset=24
        end
        get_local 7
        i32.const 20
        i32.add
        i32.load
        tee_local 1
        i32.eqz
        br_if 0 (;@2;)
        get_local 6
        i32.const 20
        i32.add
        get_local 1
        i32.store
        get_local 1
        get_local 6
        i32.store offset=24
      end
      block  ;; label = @2
        block  ;; label = @3
          get_local 2
          i32.const 16
          i32.lt_u
          br_if 0 (;@3;)
          get_local 7
          get_local 3
          i32.const 3
          i32.or
          i32.store offset=4
          get_local 7
          get_local 3
          i32.add
          tee_local 3
          get_local 2
          i32.const 1
          i32.or
          i32.store offset=4
          get_local 3
          get_local 2
          i32.add
          get_local 2
          i32.store
          block  ;; label = @4
            get_local 0
            i32.load offset=400
            tee_local 1
            i32.eqz
            br_if 0 (;@4;)
            get_local 0
            get_local 1
            i32.const 3
            i32.shr_u
            tee_local 8
            i32.const 3
            i32.shl
            i32.add
            i32.const 8
            i32.add
            set_local 6
            get_local 0
            i32.load offset=408
            set_local 1
            block  ;; label = @5
              block  ;; label = @6
                get_local 0
                i32.load
                tee_local 9
                i32.const 1
                get_local 8
                i32.const 31
                i32.and
                i32.shl
                tee_local 8
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                get_local 6
                i32.load offset=8
                set_local 8
                br 1 (;@5;)
              end
              get_local 0
              get_local 9
              get_local 8
              i32.or
              i32.store
              get_local 6
              set_local 8
            end
            get_local 6
            get_local 1
            i32.store offset=8
            get_local 8
            get_local 1
            i32.store offset=12
            get_local 1
            get_local 6
            i32.store offset=12
            get_local 1
            get_local 8
            i32.store offset=8
          end
          get_local 0
          get_local 3
          i32.store offset=408
          get_local 0
          get_local 2
          i32.store offset=400
          br 1 (;@2;)
        end
        get_local 7
        get_local 2
        get_local 3
        i32.add
        tee_local 1
        i32.const 3
        i32.or
        i32.store offset=4
        get_local 7
        get_local 1
        i32.add
        tee_local 1
        get_local 1
        i32.load offset=4
        i32.const 1
        i32.or
        i32.store offset=4
      end
      get_local 7
      i32.const 8
      i32.add
      return
    end
    get_local 2)
  (func $_ZN8dlmalloc8dlmalloc8Dlmalloc12unlink_chunk17hd0fb28e586d177f0E (type 5) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          get_local 2
          i32.const 256
          i32.lt_u
          br_if 0 (;@3;)
          get_local 1
          i32.const 24
          i32.add
          i32.load
          set_local 3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                get_local 1
                i32.load offset=12
                tee_local 4
                get_local 1
                i32.ne
                br_if 0 (;@6;)
                get_local 1
                i32.const 20
                i32.const 16
                get_local 1
                i32.const 20
                i32.add
                tee_local 4
                i32.load
                tee_local 5
                select
                i32.add
                i32.load
                tee_local 2
                br_if 1 (;@5;)
                i32.const 0
                set_local 4
                br 2 (;@4;)
              end
              get_local 1
              i32.load offset=8
              tee_local 2
              get_local 4
              i32.store offset=12
              get_local 4
              get_local 2
              i32.store offset=8
              br 1 (;@4;)
            end
            get_local 4
            get_local 1
            i32.const 16
            i32.add
            get_local 5
            select
            set_local 5
            loop  ;; label = @5
              get_local 5
              set_local 6
              block  ;; label = @6
                get_local 2
                tee_local 4
                i32.const 20
                i32.add
                tee_local 5
                i32.load
                tee_local 2
                br_if 0 (;@6;)
                get_local 4
                i32.const 16
                i32.add
                set_local 5
                get_local 4
                i32.load offset=16
                set_local 2
              end
              get_local 2
              br_if 0 (;@5;)
            end
            get_local 6
            i32.const 0
            i32.store
          end
          get_local 3
          i32.eqz
          br_if 2 (;@1;)
          block  ;; label = @4
            get_local 0
            get_local 1
            i32.const 28
            i32.add
            i32.load
            i32.const 2
            i32.shl
            i32.add
            i32.const 272
            i32.add
            tee_local 2
            i32.load
            get_local 1
            i32.eq
            br_if 0 (;@4;)
            get_local 3
            i32.const 16
            i32.const 20
            get_local 3
            i32.load offset=16
            get_local 1
            i32.eq
            select
            i32.add
            get_local 4
            i32.store
            get_local 4
            i32.eqz
            br_if 3 (;@1;)
            br 2 (;@2;)
          end
          get_local 2
          get_local 4
          i32.store
          get_local 4
          br_if 1 (;@2;)
          get_local 0
          get_local 0
          i32.load offset=4
          i32.const -2
          get_local 1
          i32.load offset=28
          i32.rotl
          i32.and
          i32.store offset=4
          return
        end
        block  ;; label = @3
          get_local 1
          i32.const 12
          i32.add
          i32.load
          tee_local 4
          get_local 1
          i32.const 8
          i32.add
          i32.load
          tee_local 5
          i32.eq
          br_if 0 (;@3;)
          get_local 5
          get_local 4
          i32.store offset=12
          get_local 4
          get_local 5
          i32.store offset=8
          return
        end
        get_local 0
        get_local 0
        i32.load
        i32.const -2
        get_local 2
        i32.const 3
        i32.shr_u
        i32.rotl
        i32.and
        i32.store
        br 1 (;@1;)
      end
      get_local 4
      get_local 3
      i32.store offset=24
      block  ;; label = @2
        get_local 1
        i32.load offset=16
        tee_local 2
        i32.eqz
        br_if 0 (;@2;)
        get_local 4
        get_local 2
        i32.store offset=16
        get_local 2
        get_local 4
        i32.store offset=24
      end
      get_local 1
      i32.const 20
      i32.add
      i32.load
      tee_local 2
      i32.eqz
      br_if 0 (;@1;)
      get_local 4
      i32.const 20
      i32.add
      get_local 2
      i32.store
      get_local 2
      get_local 4
      i32.store offset=24
      return
    end)
  (func $_ZN8dlmalloc8dlmalloc8Dlmalloc7realloc17h58780d2b1a57ee25E (type 1) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32)
    i32.const 0
    set_local 3
    block  ;; label = @1
      get_local 2
      i32.const -65588
      i32.gt_u
      br_if 0 (;@1;)
      i32.const 16
      get_local 2
      i32.const 11
      i32.add
      i32.const -8
      i32.and
      get_local 2
      i32.const 11
      i32.lt_u
      select
      set_local 4
      get_local 1
      i32.const -4
      i32.add
      tee_local 5
      i32.load
      tee_local 6
      i32.const -8
      i32.and
      set_local 7
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    get_local 6
                    i32.const 3
                    i32.and
                    i32.eqz
                    br_if 0 (;@8;)
                    get_local 1
                    i32.const -8
                    i32.add
                    tee_local 8
                    get_local 7
                    i32.add
                    set_local 9
                    get_local 7
                    get_local 4
                    i32.ge_u
                    br_if 1 (;@7;)
                    get_local 0
                    i32.load offset=412
                    get_local 9
                    i32.eq
                    br_if 2 (;@6;)
                    get_local 0
                    i32.load offset=408
                    get_local 9
                    i32.eq
                    br_if 3 (;@5;)
                    get_local 9
                    i32.const 4
                    i32.add
                    i32.load
                    tee_local 6
                    i32.const 2
                    i32.and
                    br_if 6 (;@2;)
                    get_local 6
                    i32.const -8
                    i32.and
                    tee_local 6
                    get_local 7
                    i32.add
                    tee_local 7
                    get_local 4
                    i32.ge_u
                    br_if 4 (;@4;)
                    br 6 (;@2;)
                  end
                  get_local 4
                  i32.const 256
                  i32.lt_u
                  br_if 5 (;@2;)
                  get_local 7
                  get_local 4
                  i32.const 4
                  i32.or
                  i32.lt_u
                  br_if 5 (;@2;)
                  get_local 7
                  get_local 4
                  i32.sub
                  i32.const 131073
                  i32.ge_u
                  br_if 5 (;@2;)
                  br 4 (;@3;)
                end
                get_local 7
                get_local 4
                i32.sub
                tee_local 2
                i32.const 16
                i32.lt_u
                br_if 3 (;@3;)
                get_local 5
                get_local 4
                get_local 6
                i32.const 1
                i32.and
                i32.or
                i32.const 2
                i32.or
                i32.store
                get_local 8
                get_local 4
                i32.add
                tee_local 3
                get_local 2
                i32.const 3
                i32.or
                i32.store offset=4
                get_local 9
                get_local 9
                i32.load offset=4
                i32.const 1
                i32.or
                i32.store offset=4
                get_local 0
                get_local 3
                get_local 2
                call $_ZN8dlmalloc8dlmalloc8Dlmalloc13dispose_chunk17h0b3efee3840cc165E
                br 3 (;@3;)
              end
              get_local 0
              i32.load offset=404
              get_local 7
              i32.add
              tee_local 7
              get_local 4
              i32.le_u
              br_if 3 (;@2;)
              get_local 5
              get_local 4
              get_local 6
              i32.const 1
              i32.and
              i32.or
              i32.const 2
              i32.or
              i32.store
              get_local 8
              get_local 4
              i32.add
              tee_local 2
              get_local 7
              get_local 4
              i32.sub
              tee_local 3
              i32.const 1
              i32.or
              i32.store offset=4
              get_local 0
              get_local 3
              i32.store offset=404
              get_local 0
              get_local 2
              i32.store offset=412
              br 2 (;@3;)
            end
            get_local 0
            i32.load offset=400
            get_local 7
            i32.add
            tee_local 7
            get_local 4
            i32.lt_u
            br_if 2 (;@2;)
            block  ;; label = @5
              block  ;; label = @6
                get_local 7
                get_local 4
                i32.sub
                tee_local 2
                i32.const 15
                i32.gt_u
                br_if 0 (;@6;)
                get_local 5
                get_local 6
                i32.const 1
                i32.and
                get_local 7
                i32.or
                i32.const 2
                i32.or
                i32.store
                get_local 8
                get_local 7
                i32.add
                tee_local 2
                get_local 2
                i32.load offset=4
                i32.const 1
                i32.or
                i32.store offset=4
                i32.const 0
                set_local 2
                i32.const 0
                set_local 3
                br 1 (;@5;)
              end
              get_local 5
              get_local 4
              get_local 6
              i32.const 1
              i32.and
              i32.or
              i32.const 2
              i32.or
              i32.store
              get_local 8
              get_local 4
              i32.add
              tee_local 3
              get_local 2
              i32.const 1
              i32.or
              i32.store offset=4
              get_local 8
              get_local 7
              i32.add
              tee_local 4
              get_local 2
              i32.store
              get_local 4
              get_local 4
              i32.load offset=4
              i32.const -2
              i32.and
              i32.store offset=4
            end
            get_local 0
            get_local 3
            i32.store offset=408
            get_local 0
            get_local 2
            i32.store offset=400
            br 1 (;@3;)
          end
          get_local 0
          get_local 9
          get_local 6
          call $_ZN8dlmalloc8dlmalloc8Dlmalloc12unlink_chunk17hd0fb28e586d177f0E
          block  ;; label = @4
            get_local 7
            get_local 4
            i32.sub
            tee_local 2
            i32.const 16
            i32.lt_u
            br_if 0 (;@4;)
            get_local 5
            get_local 4
            get_local 5
            i32.load
            i32.const 1
            i32.and
            i32.or
            i32.const 2
            i32.or
            i32.store
            get_local 8
            get_local 4
            i32.add
            tee_local 3
            get_local 2
            i32.const 3
            i32.or
            i32.store offset=4
            get_local 8
            get_local 7
            i32.add
            tee_local 4
            get_local 4
            i32.load offset=4
            i32.const 1
            i32.or
            i32.store offset=4
            get_local 0
            get_local 3
            get_local 2
            call $_ZN8dlmalloc8dlmalloc8Dlmalloc13dispose_chunk17h0b3efee3840cc165E
            br 1 (;@3;)
          end
          get_local 5
          get_local 7
          get_local 5
          i32.load
          i32.const 1
          i32.and
          i32.or
          i32.const 2
          i32.or
          i32.store
          get_local 8
          get_local 7
          i32.add
          tee_local 2
          get_local 2
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
        end
        get_local 1
        set_local 3
        br 1 (;@1;)
      end
      get_local 0
      get_local 2
      call $_ZN8dlmalloc8dlmalloc8Dlmalloc6malloc17ha394eee15885f4c7E
      tee_local 4
      i32.eqz
      br_if 0 (;@1;)
      get_local 4
      get_local 1
      get_local 2
      get_local 5
      i32.load
      tee_local 3
      i32.const -8
      i32.and
      i32.const 4
      i32.const 8
      get_local 3
      i32.const 3
      i32.and
      select
      i32.sub
      tee_local 3
      get_local 3
      get_local 2
      i32.gt_u
      select
      call $memcpy
      set_local 2
      get_local 0
      get_local 1
      call $_ZN8dlmalloc8dlmalloc8Dlmalloc4free17hdc2dafb68e09cc01E
      get_local 2
      return
    end
    get_local 3)
  (func $_ZN8dlmalloc8dlmalloc8Dlmalloc13dispose_chunk17h0b3efee3840cc165E (type 5) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    get_local 1
    get_local 2
    i32.add
    set_local 3
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              get_local 1
              i32.const 4
              i32.add
              i32.load
              tee_local 4
              i32.const 1
              i32.and
              br_if 0 (;@5;)
              get_local 4
              i32.const 3
              i32.and
              i32.eqz
              br_if 1 (;@4;)
              get_local 1
              i32.load
              tee_local 4
              get_local 2
              i32.add
              set_local 2
              block  ;; label = @6
                get_local 0
                i32.load offset=408
                get_local 1
                get_local 4
                i32.sub
                tee_local 1
                i32.ne
                br_if 0 (;@6;)
                get_local 3
                i32.load offset=4
                i32.const 3
                i32.and
                i32.const 3
                i32.ne
                br_if 1 (;@5;)
                get_local 0
                get_local 2
                i32.store offset=400
                get_local 3
                get_local 3
                i32.load offset=4
                i32.const -2
                i32.and
                i32.store offset=4
                get_local 1
                get_local 2
                i32.const 1
                i32.or
                i32.store offset=4
                get_local 3
                get_local 2
                i32.store
                return
              end
              get_local 0
              get_local 1
              get_local 4
              call $_ZN8dlmalloc8dlmalloc8Dlmalloc12unlink_chunk17hd0fb28e586d177f0E
            end
            block  ;; label = @5
              block  ;; label = @6
                get_local 3
                i32.const 4
                i32.add
                i32.load
                tee_local 4
                i32.const 2
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                get_local 3
                i32.const 4
                i32.add
                get_local 4
                i32.const -2
                i32.and
                i32.store
                get_local 1
                get_local 2
                i32.const 1
                i32.or
                i32.store offset=4
                get_local 1
                get_local 2
                i32.add
                get_local 2
                i32.store
                br 1 (;@5;)
              end
              block  ;; label = @6
                block  ;; label = @7
                  get_local 0
                  i32.load offset=412
                  get_local 3
                  i32.eq
                  br_if 0 (;@7;)
                  get_local 0
                  i32.load offset=408
                  get_local 3
                  i32.eq
                  br_if 1 (;@6;)
                  get_local 0
                  get_local 3
                  get_local 4
                  i32.const -8
                  i32.and
                  tee_local 4
                  call $_ZN8dlmalloc8dlmalloc8Dlmalloc12unlink_chunk17hd0fb28e586d177f0E
                  get_local 1
                  get_local 4
                  get_local 2
                  i32.add
                  tee_local 2
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  get_local 1
                  get_local 2
                  i32.add
                  get_local 2
                  i32.store
                  get_local 1
                  get_local 0
                  i32.load offset=408
                  i32.ne
                  br_if 2 (;@5;)
                  get_local 0
                  get_local 2
                  i32.store offset=400
                  return
                end
                get_local 0
                get_local 1
                i32.store offset=412
                get_local 0
                get_local 0
                i32.load offset=404
                get_local 2
                i32.add
                tee_local 2
                i32.store offset=404
                get_local 1
                get_local 2
                i32.const 1
                i32.or
                i32.store offset=4
                get_local 1
                get_local 0
                i32.load offset=408
                i32.ne
                br_if 2 (;@4;)
                get_local 0
                i32.const 0
                i32.store offset=400
                get_local 0
                i32.const 0
                i32.store offset=408
                return
              end
              get_local 0
              get_local 1
              i32.store offset=408
              get_local 0
              get_local 0
              i32.load offset=400
              get_local 2
              i32.add
              tee_local 2
              i32.store offset=400
              get_local 1
              get_local 2
              i32.const 1
              i32.or
              i32.store offset=4
              get_local 1
              get_local 2
              i32.add
              get_local 2
              i32.store
              return
            end
            get_local 2
            i32.const 256
            i32.lt_u
            br_if 3 (;@1;)
            block  ;; label = @5
              block  ;; label = @6
                get_local 2
                i32.const 8
                i32.shr_u
                tee_local 4
                br_if 0 (;@6;)
                i32.const 0
                set_local 3
                br 1 (;@5;)
              end
              i32.const 31
              set_local 3
              get_local 2
              i32.const 16777215
              i32.gt_u
              br_if 0 (;@5;)
              get_local 2
              i32.const 6
              get_local 4
              i32.clz
              tee_local 3
              i32.sub
              i32.const 31
              i32.and
              i32.shr_u
              i32.const 1
              i32.and
              get_local 3
              i32.const 1
              i32.shl
              i32.sub
              i32.const 62
              i32.add
              set_local 3
            end
            get_local 1
            i64.const 0
            i64.store offset=16 align=4
            get_local 1
            i32.const 28
            i32.add
            get_local 3
            i32.store
            get_local 0
            get_local 3
            i32.const 2
            i32.shl
            i32.add
            i32.const 272
            i32.add
            set_local 4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  get_local 0
                  i32.const 4
                  i32.add
                  tee_local 0
                  i32.load
                  tee_local 5
                  i32.const 1
                  get_local 3
                  i32.const 31
                  i32.and
                  i32.shl
                  tee_local 6
                  i32.and
                  i32.eqz
                  br_if 0 (;@7;)
                  get_local 4
                  i32.load
                  tee_local 4
                  i32.const 4
                  i32.add
                  i32.load
                  i32.const -8
                  i32.and
                  get_local 2
                  i32.ne
                  br_if 1 (;@6;)
                  get_local 4
                  set_local 0
                  br 2 (;@5;)
                end
                get_local 0
                get_local 5
                get_local 6
                i32.or
                i32.store
                get_local 4
                get_local 1
                i32.store
                get_local 1
                i32.const 24
                i32.add
                get_local 4
                i32.store
                br 4 (;@2;)
              end
              get_local 2
              i32.const 0
              i32.const 25
              get_local 3
              i32.const 1
              i32.shr_u
              i32.sub
              i32.const 31
              i32.and
              get_local 3
              i32.const 31
              i32.eq
              select
              i32.shl
              set_local 3
              loop  ;; label = @6
                get_local 4
                get_local 3
                i32.const 29
                i32.shr_u
                i32.const 4
                i32.and
                i32.add
                i32.const 16
                i32.add
                tee_local 5
                i32.load
                tee_local 0
                i32.eqz
                br_if 3 (;@3;)
                get_local 3
                i32.const 1
                i32.shl
                set_local 3
                get_local 0
                set_local 4
                get_local 0
                i32.const 4
                i32.add
                i32.load
                i32.const -8
                i32.and
                get_local 2
                i32.ne
                br_if 0 (;@6;)
              end
            end
            get_local 0
            i32.load offset=8
            tee_local 2
            get_local 1
            i32.store offset=12
            get_local 0
            get_local 1
            i32.store offset=8
            get_local 1
            i32.const 24
            i32.add
            i32.const 0
            i32.store
            get_local 1
            get_local 0
            i32.store offset=12
            get_local 1
            get_local 2
            i32.store offset=8
          end
          return
        end
        get_local 5
        get_local 1
        i32.store
        get_local 1
        i32.const 24
        i32.add
        get_local 4
        i32.store
      end
      get_local 1
      get_local 1
      i32.store offset=12
      get_local 1
      get_local 1
      i32.store offset=8
      return
    end
    get_local 0
    get_local 2
    i32.const 3
    i32.shr_u
    tee_local 3
    i32.const 3
    i32.shl
    i32.add
    i32.const 8
    i32.add
    set_local 2
    block  ;; label = @1
      block  ;; label = @2
        get_local 0
        i32.load
        tee_local 4
        i32.const 1
        get_local 3
        i32.const 31
        i32.and
        i32.shl
        tee_local 3
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        get_local 2
        i32.load offset=8
        set_local 0
        br 1 (;@1;)
      end
      get_local 0
      get_local 4
      get_local 3
      i32.or
      i32.store
      get_local 2
      set_local 0
    end
    get_local 2
    get_local 1
    i32.store offset=8
    get_local 0
    get_local 1
    i32.store offset=12
    get_local 1
    get_local 2
    i32.store offset=12
    get_local 1
    get_local 0
    i32.store offset=8)
  (func $_ZN8dlmalloc8dlmalloc8Dlmalloc4free17hdc2dafb68e09cc01E (type 0) (param i32 i32)
    (local i32 i32 i32 i32 i32 i32)
    get_local 1
    i32.const -8
    i32.add
    tee_local 2
    get_local 1
    i32.const -4
    i32.add
    i32.load
    tee_local 3
    i32.const -8
    i32.and
    tee_local 1
    i32.add
    set_local 4
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            get_local 3
            i32.const 1
            i32.and
            br_if 0 (;@4;)
            get_local 3
            i32.const 3
            i32.and
            i32.eqz
            br_if 1 (;@3;)
            get_local 2
            i32.load
            tee_local 3
            get_local 1
            i32.add
            set_local 1
            block  ;; label = @5
              get_local 0
              i32.load offset=408
              get_local 2
              get_local 3
              i32.sub
              tee_local 2
              i32.ne
              br_if 0 (;@5;)
              get_local 4
              i32.load offset=4
              i32.const 3
              i32.and
              i32.const 3
              i32.ne
              br_if 1 (;@4;)
              get_local 0
              get_local 1
              i32.store offset=400
              get_local 4
              get_local 4
              i32.load offset=4
              i32.const -2
              i32.and
              i32.store offset=4
              get_local 2
              get_local 1
              i32.const 1
              i32.or
              i32.store offset=4
              get_local 2
              get_local 1
              i32.add
              get_local 1
              i32.store
              return
            end
            get_local 0
            get_local 2
            get_local 3
            call $_ZN8dlmalloc8dlmalloc8Dlmalloc12unlink_chunk17hd0fb28e586d177f0E
          end
          block  ;; label = @4
            block  ;; label = @5
              get_local 4
              i32.const 4
              i32.add
              tee_local 5
              i32.load
              tee_local 3
              i32.const 2
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              get_local 5
              get_local 3
              i32.const -2
              i32.and
              i32.store
              get_local 2
              get_local 1
              i32.const 1
              i32.or
              i32.store offset=4
              get_local 2
              get_local 1
              i32.add
              get_local 1
              i32.store
              br 1 (;@4;)
            end
            block  ;; label = @5
              block  ;; label = @6
                get_local 0
                i32.load offset=412
                get_local 4
                i32.eq
                br_if 0 (;@6;)
                get_local 0
                i32.load offset=408
                get_local 4
                i32.eq
                br_if 1 (;@5;)
                get_local 0
                get_local 4
                get_local 3
                i32.const -8
                i32.and
                tee_local 3
                call $_ZN8dlmalloc8dlmalloc8Dlmalloc12unlink_chunk17hd0fb28e586d177f0E
                get_local 2
                get_local 3
                get_local 1
                i32.add
                tee_local 1
                i32.const 1
                i32.or
                i32.store offset=4
                get_local 2
                get_local 1
                i32.add
                get_local 1
                i32.store
                get_local 2
                get_local 0
                i32.load offset=408
                i32.ne
                br_if 2 (;@4;)
                get_local 0
                get_local 1
                i32.store offset=400
                return
              end
              get_local 0
              get_local 2
              i32.store offset=412
              get_local 0
              get_local 0
              i32.load offset=404
              get_local 1
              i32.add
              tee_local 1
              i32.store offset=404
              get_local 2
              get_local 1
              i32.const 1
              i32.or
              i32.store offset=4
              block  ;; label = @6
                get_local 2
                get_local 0
                i32.load offset=408
                i32.ne
                br_if 0 (;@6;)
                get_local 0
                i32.const 0
                i32.store offset=400
                get_local 0
                i32.const 0
                i32.store offset=408
              end
              get_local 0
              i32.const 440
              i32.add
              i32.load
              tee_local 3
              get_local 1
              i32.ge_u
              br_if 2 (;@3;)
              get_local 0
              i32.load offset=412
              tee_local 1
              i32.eqz
              br_if 2 (;@3;)
              block  ;; label = @6
                get_local 0
                i32.load offset=404
                tee_local 5
                i32.const 41
                i32.lt_u
                br_if 0 (;@6;)
                get_local 0
                i32.const 424
                i32.add
                set_local 2
                loop  ;; label = @7
                  block  ;; label = @8
                    get_local 2
                    i32.load
                    tee_local 4
                    get_local 1
                    i32.gt_u
                    br_if 0 (;@8;)
                    get_local 4
                    get_local 2
                    i32.load offset=4
                    i32.add
                    get_local 1
                    i32.gt_u
                    br_if 2 (;@6;)
                  end
                  get_local 2
                  i32.load offset=8
                  tee_local 2
                  br_if 0 (;@7;)
                end
              end
              block  ;; label = @6
                block  ;; label = @7
                  get_local 0
                  i32.const 432
                  i32.add
                  i32.load
                  tee_local 1
                  br_if 0 (;@7;)
                  i32.const 4095
                  set_local 2
                  br 1 (;@6;)
                end
                i32.const 0
                set_local 2
                loop  ;; label = @7
                  get_local 2
                  i32.const 1
                  i32.add
                  set_local 2
                  get_local 1
                  i32.load offset=8
                  tee_local 1
                  br_if 0 (;@7;)
                end
                get_local 2
                i32.const 4095
                get_local 2
                i32.const 4095
                i32.gt_u
                select
                set_local 2
              end
              get_local 0
              get_local 2
              i32.store offset=448
              get_local 5
              get_local 3
              i32.le_u
              br_if 2 (;@3;)
              get_local 0
              i32.const 440
              i32.add
              i32.const -1
              i32.store
              return
            end
            get_local 0
            get_local 2
            i32.store offset=408
            get_local 0
            get_local 0
            i32.load offset=400
            get_local 1
            i32.add
            tee_local 1
            i32.store offset=400
            get_local 2
            get_local 1
            i32.const 1
            i32.or
            i32.store offset=4
            get_local 2
            get_local 1
            i32.add
            get_local 1
            i32.store
            return
          end
          get_local 1
          i32.const 256
          i32.lt_u
          br_if 1 (;@2;)
          block  ;; label = @4
            block  ;; label = @5
              get_local 1
              i32.const 8
              i32.shr_u
              tee_local 3
              br_if 0 (;@5;)
              i32.const 0
              set_local 4
              br 1 (;@4;)
            end
            i32.const 31
            set_local 4
            get_local 1
            i32.const 16777215
            i32.gt_u
            br_if 0 (;@4;)
            get_local 1
            i32.const 6
            get_local 3
            i32.clz
            tee_local 4
            i32.sub
            i32.const 31
            i32.and
            i32.shr_u
            i32.const 1
            i32.and
            get_local 4
            i32.const 1
            i32.shl
            i32.sub
            i32.const 62
            i32.add
            set_local 4
          end
          get_local 2
          i64.const 0
          i64.store offset=16 align=4
          get_local 2
          i32.const 28
          i32.add
          get_local 4
          i32.store
          get_local 0
          get_local 4
          i32.const 2
          i32.shl
          i32.add
          i32.const 272
          i32.add
          set_local 3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      get_local 0
                      i32.const 4
                      i32.add
                      tee_local 5
                      i32.load
                      tee_local 6
                      i32.const 1
                      get_local 4
                      i32.const 31
                      i32.and
                      i32.shl
                      tee_local 7
                      i32.and
                      i32.eqz
                      br_if 0 (;@9;)
                      get_local 3
                      i32.load
                      tee_local 5
                      i32.const 4
                      i32.add
                      i32.load
                      i32.const -8
                      i32.and
                      get_local 1
                      i32.ne
                      br_if 1 (;@8;)
                      get_local 5
                      set_local 4
                      br 2 (;@7;)
                    end
                    get_local 5
                    get_local 6
                    get_local 7
                    i32.or
                    i32.store
                    get_local 3
                    get_local 2
                    i32.store
                    get_local 2
                    i32.const 24
                    i32.add
                    get_local 3
                    i32.store
                    br 3 (;@5;)
                  end
                  get_local 1
                  i32.const 0
                  i32.const 25
                  get_local 4
                  i32.const 1
                  i32.shr_u
                  i32.sub
                  i32.const 31
                  i32.and
                  get_local 4
                  i32.const 31
                  i32.eq
                  select
                  i32.shl
                  set_local 3
                  loop  ;; label = @8
                    get_local 5
                    get_local 3
                    i32.const 29
                    i32.shr_u
                    i32.const 4
                    i32.and
                    i32.add
                    i32.const 16
                    i32.add
                    tee_local 6
                    i32.load
                    tee_local 4
                    i32.eqz
                    br_if 2 (;@6;)
                    get_local 3
                    i32.const 1
                    i32.shl
                    set_local 3
                    get_local 4
                    set_local 5
                    get_local 4
                    i32.const 4
                    i32.add
                    i32.load
                    i32.const -8
                    i32.and
                    get_local 1
                    i32.ne
                    br_if 0 (;@8;)
                  end
                end
                get_local 4
                i32.load offset=8
                tee_local 1
                get_local 2
                i32.store offset=12
                get_local 4
                get_local 2
                i32.store offset=8
                get_local 2
                i32.const 24
                i32.add
                i32.const 0
                i32.store
                get_local 2
                get_local 4
                i32.store offset=12
                get_local 2
                get_local 1
                i32.store offset=8
                br 2 (;@4;)
              end
              get_local 6
              get_local 2
              i32.store
              get_local 2
              i32.const 24
              i32.add
              get_local 5
              i32.store
            end
            get_local 2
            get_local 2
            i32.store offset=12
            get_local 2
            get_local 2
            i32.store offset=8
          end
          get_local 0
          get_local 0
          i32.load offset=448
          i32.const -1
          i32.add
          tee_local 2
          i32.store offset=448
          get_local 2
          i32.eqz
          br_if 2 (;@1;)
        end
        return
      end
      get_local 0
      get_local 1
      i32.const 3
      i32.shr_u
      tee_local 4
      i32.const 3
      i32.shl
      i32.add
      i32.const 8
      i32.add
      set_local 1
      block  ;; label = @2
        block  ;; label = @3
          get_local 0
          i32.load
          tee_local 3
          i32.const 1
          get_local 4
          i32.const 31
          i32.and
          i32.shl
          tee_local 4
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          get_local 1
          i32.load offset=8
          set_local 0
          br 1 (;@2;)
        end
        get_local 0
        get_local 3
        get_local 4
        i32.or
        i32.store
        get_local 1
        set_local 0
      end
      get_local 1
      get_local 2
      i32.store offset=8
      get_local 0
      get_local 2
      i32.store offset=12
      get_local 2
      get_local 1
      i32.store offset=12
      get_local 2
      get_local 0
      i32.store offset=8
      return
    end
    block  ;; label = @1
      get_local 0
      i32.const 432
      i32.add
      i32.load
      tee_local 1
      br_if 0 (;@1;)
      get_local 0
      i32.const 4095
      i32.store offset=448
      return
    end
    i32.const 0
    set_local 2
    loop  ;; label = @1
      get_local 2
      i32.const 1
      i32.add
      set_local 2
      get_local 1
      i32.load offset=8
      tee_local 1
      br_if 0 (;@1;)
    end
    get_local 0
    get_local 2
    i32.const 4095
    get_local 2
    i32.const 4095
    i32.gt_u
    select
    i32.store offset=448)
  (func $_ZN8dlmalloc8dlmalloc8Dlmalloc8memalign17h6d9752ec0c410003E (type 1) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32)
    i32.const 0
    set_local 3
    block  ;; label = @1
      i32.const -65587
      get_local 1
      i32.const 16
      get_local 1
      i32.const 16
      i32.gt_u
      select
      tee_local 1
      i32.sub
      get_local 2
      i32.le_u
      br_if 0 (;@1;)
      get_local 0
      get_local 1
      i32.const 16
      get_local 2
      i32.const 11
      i32.add
      i32.const -8
      i32.and
      get_local 2
      i32.const 11
      i32.lt_u
      select
      tee_local 4
      i32.add
      i32.const 12
      i32.add
      call $_ZN8dlmalloc8dlmalloc8Dlmalloc6malloc17ha394eee15885f4c7E
      tee_local 2
      i32.eqz
      br_if 0 (;@1;)
      get_local 2
      i32.const -8
      i32.add
      set_local 3
      block  ;; label = @2
        block  ;; label = @3
          get_local 1
          i32.const -1
          i32.add
          tee_local 5
          get_local 2
          i32.and
          br_if 0 (;@3;)
          get_local 3
          set_local 1
          br 1 (;@2;)
        end
        get_local 2
        i32.const -4
        i32.add
        tee_local 6
        i32.load
        tee_local 7
        i32.const -8
        i32.and
        get_local 5
        get_local 2
        i32.add
        i32.const 0
        get_local 1
        i32.sub
        i32.and
        i32.const -8
        i32.add
        tee_local 2
        get_local 2
        get_local 1
        i32.add
        get_local 2
        get_local 3
        i32.sub
        i32.const 16
        i32.gt_u
        select
        tee_local 1
        get_local 3
        i32.sub
        tee_local 2
        i32.sub
        set_local 5
        block  ;; label = @3
          get_local 7
          i32.const 3
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          get_local 1
          get_local 5
          get_local 1
          i32.load offset=4
          i32.const 1
          i32.and
          i32.or
          i32.const 2
          i32.or
          i32.store offset=4
          get_local 1
          get_local 5
          i32.add
          tee_local 5
          get_local 5
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
          get_local 6
          get_local 2
          get_local 6
          i32.load
          i32.const 1
          i32.and
          i32.or
          i32.const 2
          i32.or
          i32.store
          get_local 1
          get_local 1
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
          get_local 0
          get_local 3
          get_local 2
          call $_ZN8dlmalloc8dlmalloc8Dlmalloc13dispose_chunk17h0b3efee3840cc165E
          br 1 (;@2;)
        end
        get_local 3
        i32.load
        set_local 3
        get_local 1
        get_local 5
        i32.store offset=4
        get_local 1
        get_local 3
        get_local 2
        i32.add
        i32.store
      end
      block  ;; label = @2
        get_local 1
        i32.const 4
        i32.add
        i32.load
        tee_local 2
        i32.const 3
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        get_local 2
        i32.const -8
        i32.and
        tee_local 3
        get_local 4
        i32.const 16
        i32.add
        i32.le_u
        br_if 0 (;@2;)
        get_local 1
        i32.const 4
        i32.add
        get_local 4
        get_local 2
        i32.const 1
        i32.and
        i32.or
        i32.const 2
        i32.or
        i32.store
        get_local 1
        get_local 4
        i32.add
        tee_local 2
        get_local 3
        get_local 4
        i32.sub
        tee_local 4
        i32.const 3
        i32.or
        i32.store offset=4
        get_local 1
        get_local 3
        i32.add
        tee_local 3
        get_local 3
        i32.load offset=4
        i32.const 1
        i32.or
        i32.store offset=4
        get_local 0
        get_local 2
        get_local 4
        call $_ZN8dlmalloc8dlmalloc8Dlmalloc13dispose_chunk17h0b3efee3840cc165E
      end
      get_local 1
      i32.const 8
      i32.add
      set_local 3
    end
    get_local 3)
  (func $_ZN5alloc5alloc18handle_alloc_error17hc11aade6dede5d47E (type 0) (param i32 i32)
    get_local 0
    get_local 1
    call $rust_oom
    unreachable)
  (func $_ZN5alloc7raw_vec17capacity_overflow17he8b42367ee028944E (type 8)
    i32.const 1048907
    i32.const 17
    i32.const 1048924
    call $_ZN4core9panicking5panic17h2574daf311dde64fE
    unreachable)
  (func $_ZN4core3ops8function6FnOnce9call_once17h8f9c18931d78bf61E (type 2) (param i32 i32) (result i32)
    get_local 0
    i32.load
    drop
    loop (result i32)  ;; label = @1
      br 0 (;@1;)
    end)
  (func $_ZN4core3ptr13drop_in_place17h00410729a46e8620E (type 4) (param i32))
  (func $_ZN4core9panicking18panic_bounds_check17h9b2ee111ce4aa0dbE (type 5) (param i32 i32 i32)
    (local i32)
    get_global 0
    i32.const 48
    i32.sub
    tee_local 3
    set_global 0
    get_local 3
    get_local 2
    i32.store offset=4
    get_local 3
    get_local 1
    i32.store
    get_local 3
    i32.const 28
    i32.add
    i32.const 2
    i32.store
    get_local 3
    i32.const 44
    i32.add
    i32.const 1
    i32.store
    get_local 3
    i64.const 2
    i64.store offset=12 align=4
    get_local 3
    i32.const 1049008
    i32.store offset=8
    get_local 3
    i32.const 1
    i32.store offset=36
    get_local 3
    get_local 3
    i32.const 32
    i32.add
    i32.store offset=24
    get_local 3
    get_local 3
    i32.store offset=40
    get_local 3
    get_local 3
    i32.const 4
    i32.add
    i32.store offset=32
    get_local 3
    i32.const 8
    i32.add
    get_local 0
    call $_ZN4core9panicking9panic_fmt17h13b59f2eed0c6357E
    unreachable)
  (func $_ZN4core9panicking5panic17h2574daf311dde64fE (type 5) (param i32 i32 i32)
    (local i32)
    get_global 0
    i32.const 32
    i32.sub
    tee_local 3
    set_global 0
    get_local 3
    i32.const 20
    i32.add
    i32.const 0
    i32.store
    get_local 3
    i32.const 1048940
    i32.store offset=16
    get_local 3
    i64.const 1
    i64.store offset=4 align=4
    get_local 3
    get_local 1
    i32.store offset=28
    get_local 3
    get_local 0
    i32.store offset=24
    get_local 3
    get_local 3
    i32.const 24
    i32.add
    i32.store
    get_local 3
    get_local 2
    call $_ZN4core9panicking9panic_fmt17h13b59f2eed0c6357E
    unreachable)
  (func $_ZN4core9panicking9panic_fmt17h13b59f2eed0c6357E (type 0) (param i32 i32)
    (local i32)
    get_global 0
    i32.const 16
    i32.sub
    tee_local 2
    set_global 0
    get_local 2
    get_local 1
    i32.store offset=12
    get_local 2
    get_local 0
    i32.store offset=8
    get_local 2
    i32.const 1048940
    i32.store offset=4
    get_local 2
    i32.const 1048940
    i32.store
    get_local 2
    call $rust_begin_unwind
    unreachable)
  (func $_ZN4core3fmt3num3imp52_$LT$impl$u20$core..fmt..Display$u20$for$u20$u32$GT$3fmt17h99ce8059c5fdd42bE (type 2) (param i32 i32) (result i32)
    get_local 0
    i64.load32_u
    i32.const 1
    get_local 1
    call $_ZN4core3fmt3num3imp7fmt_u6417h88aa9a282d13def7E)
  (func $_ZN4core3fmt5write17h3d72ff271d605c43E (type 1) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    get_global 0
    i32.const 48
    i32.sub
    tee_local 3
    set_global 0
    get_local 3
    i32.const 36
    i32.add
    get_local 1
    i32.store
    get_local 3
    i32.const 3
    i32.store8 offset=40
    get_local 3
    i64.const 137438953472
    i64.store offset=8
    get_local 3
    get_local 0
    i32.store offset=32
    i32.const 0
    set_local 4
    get_local 3
    i32.const 0
    i32.store offset=24
    get_local 3
    i32.const 0
    i32.store offset=16
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            get_local 2
            i32.load offset=8
            tee_local 5
            i32.eqz
            br_if 0 (;@4;)
            get_local 2
            i32.load
            set_local 6
            get_local 2
            i32.load offset=4
            tee_local 7
            get_local 2
            i32.const 12
            i32.add
            i32.load
            tee_local 8
            get_local 8
            get_local 7
            i32.gt_u
            select
            tee_local 9
            i32.eqz
            br_if 1 (;@3;)
            get_local 2
            i32.const 20
            i32.add
            i32.load
            set_local 10
            get_local 2
            i32.load offset=16
            set_local 11
            i32.const 1
            set_local 8
            get_local 0
            get_local 6
            i32.load
            get_local 6
            i32.load offset=4
            get_local 1
            i32.load offset=12
            call_indirect (type 1)
            br_if 3 (;@1;)
            get_local 6
            i32.const 12
            i32.add
            set_local 2
            i32.const 1
            set_local 4
            block  ;; label = @5
              block  ;; label = @6
                loop  ;; label = @7
                  get_local 3
                  get_local 5
                  i32.const 4
                  i32.add
                  i32.load
                  i32.store offset=12
                  get_local 3
                  get_local 5
                  i32.const 28
                  i32.add
                  i32.load8_u
                  i32.store8 offset=40
                  get_local 3
                  get_local 5
                  i32.const 8
                  i32.add
                  i32.load
                  i32.store offset=8
                  get_local 5
                  i32.const 24
                  i32.add
                  i32.load
                  set_local 8
                  i32.const 0
                  set_local 1
                  i32.const 0
                  set_local 0
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        get_local 5
                        i32.const 20
                        i32.add
                        i32.load
                        br_table 1 (;@9;) 0 (;@10;) 2 (;@8;) 1 (;@9;)
                      end
                      get_local 8
                      get_local 10
                      i32.ge_u
                      br_if 3 (;@6;)
                      get_local 8
                      i32.const 3
                      i32.shl
                      set_local 12
                      i32.const 0
                      set_local 0
                      get_local 11
                      get_local 12
                      i32.add
                      tee_local 12
                      i32.load offset=4
                      i32.const 17
                      i32.ne
                      br_if 1 (;@8;)
                      get_local 12
                      i32.load
                      i32.load
                      set_local 8
                    end
                    i32.const 1
                    set_local 0
                  end
                  get_local 3
                  get_local 8
                  i32.store offset=20
                  get_local 3
                  get_local 0
                  i32.store offset=16
                  get_local 5
                  i32.const 16
                  i32.add
                  i32.load
                  set_local 8
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        get_local 5
                        i32.const 12
                        i32.add
                        i32.load
                        br_table 1 (;@9;) 0 (;@10;) 2 (;@8;) 1 (;@9;)
                      end
                      get_local 8
                      get_local 10
                      i32.ge_u
                      br_if 4 (;@5;)
                      get_local 8
                      i32.const 3
                      i32.shl
                      set_local 0
                      get_local 11
                      get_local 0
                      i32.add
                      tee_local 0
                      i32.load offset=4
                      i32.const 17
                      i32.ne
                      br_if 1 (;@8;)
                      get_local 0
                      i32.load
                      i32.load
                      set_local 8
                    end
                    i32.const 1
                    set_local 1
                  end
                  get_local 3
                  get_local 8
                  i32.store offset=28
                  get_local 3
                  get_local 1
                  i32.store offset=24
                  block  ;; label = @8
                    get_local 5
                    i32.load
                    tee_local 8
                    get_local 10
                    i32.ge_u
                    br_if 0 (;@8;)
                    get_local 11
                    get_local 8
                    i32.const 3
                    i32.shl
                    i32.add
                    tee_local 8
                    i32.load
                    get_local 3
                    i32.const 8
                    i32.add
                    get_local 8
                    i32.load offset=4
                    call_indirect (type 2)
                    br_if 6 (;@2;)
                    get_local 4
                    get_local 9
                    i32.ge_u
                    br_if 5 (;@3;)
                    get_local 2
                    i32.const -4
                    i32.add
                    set_local 0
                    get_local 2
                    i32.load
                    set_local 1
                    get_local 2
                    i32.const 8
                    i32.add
                    set_local 2
                    get_local 5
                    i32.const 32
                    i32.add
                    set_local 5
                    i32.const 1
                    set_local 8
                    get_local 4
                    i32.const 1
                    i32.add
                    set_local 4
                    get_local 3
                    i32.load offset=32
                    get_local 0
                    i32.load
                    get_local 1
                    get_local 3
                    i32.load offset=36
                    i32.load offset=12
                    call_indirect (type 1)
                    i32.eqz
                    br_if 1 (;@7;)
                    br 7 (;@1;)
                  end
                end
                i32.const 1049248
                get_local 8
                get_local 10
                call $_ZN4core9panicking18panic_bounds_check17h9b2ee111ce4aa0dbE
                unreachable
              end
              i32.const 1049264
              get_local 8
              get_local 10
              call $_ZN4core9panicking18panic_bounds_check17h9b2ee111ce4aa0dbE
              unreachable
            end
            i32.const 1049264
            get_local 8
            get_local 10
            call $_ZN4core9panicking18panic_bounds_check17h9b2ee111ce4aa0dbE
            unreachable
          end
          get_local 2
          i32.load
          set_local 6
          get_local 2
          i32.load offset=4
          tee_local 7
          get_local 2
          i32.const 20
          i32.add
          i32.load
          tee_local 5
          get_local 5
          get_local 7
          i32.gt_u
          select
          tee_local 10
          i32.eqz
          br_if 0 (;@3;)
          get_local 2
          i32.load offset=16
          set_local 5
          i32.const 1
          set_local 8
          get_local 0
          get_local 6
          i32.load
          get_local 6
          i32.load offset=4
          get_local 1
          i32.load offset=12
          call_indirect (type 1)
          br_if 2 (;@1;)
          get_local 6
          i32.const 12
          i32.add
          set_local 2
          i32.const 1
          set_local 4
          loop  ;; label = @4
            get_local 5
            i32.load
            get_local 3
            i32.const 8
            i32.add
            get_local 5
            i32.const 4
            i32.add
            i32.load
            call_indirect (type 2)
            br_if 2 (;@2;)
            get_local 4
            get_local 10
            i32.ge_u
            br_if 1 (;@3;)
            get_local 2
            i32.const -4
            i32.add
            set_local 0
            get_local 2
            i32.load
            set_local 1
            get_local 2
            i32.const 8
            i32.add
            set_local 2
            get_local 5
            i32.const 8
            i32.add
            set_local 5
            i32.const 1
            set_local 8
            get_local 4
            i32.const 1
            i32.add
            set_local 4
            get_local 3
            i32.load offset=32
            get_local 0
            i32.load
            get_local 1
            get_local 3
            i32.load offset=36
            i32.load offset=12
            call_indirect (type 1)
            i32.eqz
            br_if 0 (;@4;)
            br 3 (;@1;)
          end
        end
        block  ;; label = @3
          get_local 7
          get_local 4
          i32.le_u
          br_if 0 (;@3;)
          i32.const 1
          set_local 8
          get_local 3
          i32.load offset=32
          get_local 6
          get_local 4
          i32.const 3
          i32.shl
          i32.add
          tee_local 5
          i32.load
          get_local 5
          i32.load offset=4
          get_local 3
          i32.load offset=36
          i32.load offset=12
          call_indirect (type 1)
          br_if 2 (;@1;)
        end
        i32.const 0
        set_local 8
        br 1 (;@1;)
      end
      i32.const 1
      set_local 8
    end
    get_local 3
    i32.const 48
    i32.add
    set_global 0
    get_local 8)
  (func $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17hd56d7694ecea05efE (type 6) (param i32) (result i64)
    i64.const -620717282625089513)
  (func $_ZN4core5panic9PanicInfo7message17h36a3d1b385d755a2E (type 3) (param i32) (result i32)
    get_local 0
    i32.load offset=8)
  (func $_ZN4core5panic9PanicInfo8location17hf4c15ed78456999dE (type 3) (param i32) (result i32)
    get_local 0
    i32.load offset=12)
  (func $_ZN4core5panic8Location6caller17ha201287b09b4d397E (type 3) (param i32) (result i32)
    get_local 0)
  (func $_ZN4core3fmt9Formatter12pad_integral17h45c2ae804e73f116E (type 10) (param i32 i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        get_local 1
        i32.eqz
        br_if 0 (;@2;)
        i32.const 43
        i32.const 1114112
        get_local 0
        i32.load
        tee_local 6
        i32.const 1
        i32.and
        tee_local 1
        select
        set_local 7
        get_local 1
        get_local 5
        i32.add
        set_local 8
        br 1 (;@1;)
      end
      get_local 5
      i32.const 1
      i32.add
      set_local 8
      get_local 0
      i32.load
      set_local 6
      i32.const 45
      set_local 7
    end
    block  ;; label = @1
      block  ;; label = @2
        get_local 6
        i32.const 4
        i32.and
        br_if 0 (;@2;)
        i32.const 0
        set_local 2
        br 1 (;@1;)
      end
      i32.const 0
      set_local 9
      block  ;; label = @2
        get_local 3
        i32.eqz
        br_if 0 (;@2;)
        get_local 3
        set_local 10
        get_local 2
        set_local 1
        loop  ;; label = @3
          get_local 9
          get_local 1
          i32.load8_u
          i32.const 192
          i32.and
          i32.const 128
          i32.eq
          i32.add
          set_local 9
          get_local 1
          i32.const 1
          i32.add
          set_local 1
          get_local 10
          i32.const -1
          i32.add
          tee_local 10
          br_if 0 (;@3;)
        end
      end
      get_local 8
      get_local 3
      i32.add
      get_local 9
      i32.sub
      set_local 8
    end
    i32.const 1
    set_local 1
    block  ;; label = @1
      block  ;; label = @2
        get_local 0
        i32.load offset=8
        i32.const 1
        i32.eq
        br_if 0 (;@2;)
        get_local 0
        get_local 7
        get_local 2
        get_local 3
        call $_ZN4core3fmt9Formatter12pad_integral12write_prefix17hac9bbd52894cffc0E
        br_if 1 (;@1;)
        get_local 0
        i32.load offset=24
        get_local 4
        get_local 5
        get_local 0
        i32.const 28
        i32.add
        i32.load
        i32.load offset=12
        call_indirect (type 1)
        set_local 1
        br 1 (;@1;)
      end
      block  ;; label = @2
        get_local 0
        i32.const 12
        i32.add
        i32.load
        tee_local 9
        get_local 8
        i32.gt_u
        br_if 0 (;@2;)
        get_local 0
        get_local 7
        get_local 2
        get_local 3
        call $_ZN4core3fmt9Formatter12pad_integral12write_prefix17hac9bbd52894cffc0E
        br_if 1 (;@1;)
        get_local 0
        i32.load offset=24
        get_local 4
        get_local 5
        get_local 0
        i32.const 28
        i32.add
        i32.load
        i32.load offset=12
        call_indirect (type 1)
        return
      end
      block  ;; label = @2
        block  ;; label = @3
          get_local 6
          i32.const 8
          i32.and
          br_if 0 (;@3;)
          i32.const 0
          set_local 1
          get_local 9
          get_local 8
          i32.sub
          tee_local 9
          set_local 8
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                i32.const 1
                get_local 0
                i32.load8_u offset=32
                tee_local 10
                get_local 10
                i32.const 3
                i32.eq
                select
                br_table 2 (;@4;) 1 (;@5;) 0 (;@6;) 1 (;@5;) 2 (;@4;)
              end
              get_local 9
              i32.const 1
              i32.shr_u
              set_local 1
              get_local 9
              i32.const 1
              i32.add
              i32.const 1
              i32.shr_u
              set_local 8
              br 1 (;@4;)
            end
            i32.const 0
            set_local 8
            get_local 9
            set_local 1
          end
          get_local 1
          i32.const 1
          i32.add
          set_local 1
          loop  ;; label = @4
            get_local 1
            i32.const -1
            i32.add
            tee_local 1
            i32.eqz
            br_if 2 (;@2;)
            get_local 0
            i32.load offset=24
            get_local 0
            i32.load offset=4
            get_local 0
            i32.load offset=28
            i32.load offset=16
            call_indirect (type 2)
            i32.eqz
            br_if 0 (;@4;)
          end
          i32.const 1
          return
        end
        get_local 0
        i32.load offset=4
        set_local 6
        get_local 0
        i32.const 48
        i32.store offset=4
        get_local 0
        i32.load8_u offset=32
        set_local 11
        i32.const 1
        set_local 1
        get_local 0
        i32.const 1
        i32.store8 offset=32
        get_local 0
        get_local 7
        get_local 2
        get_local 3
        call $_ZN4core3fmt9Formatter12pad_integral12write_prefix17hac9bbd52894cffc0E
        br_if 1 (;@1;)
        i32.const 0
        set_local 1
        get_local 9
        get_local 8
        i32.sub
        tee_local 10
        set_local 3
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              i32.const 1
              get_local 0
              i32.load8_u offset=32
              tee_local 9
              get_local 9
              i32.const 3
              i32.eq
              select
              br_table 2 (;@3;) 1 (;@4;) 0 (;@5;) 1 (;@4;) 2 (;@3;)
            end
            get_local 10
            i32.const 1
            i32.shr_u
            set_local 1
            get_local 10
            i32.const 1
            i32.add
            i32.const 1
            i32.shr_u
            set_local 3
            br 1 (;@3;)
          end
          i32.const 0
          set_local 3
          get_local 10
          set_local 1
        end
        get_local 1
        i32.const 1
        i32.add
        set_local 1
        block  ;; label = @3
          loop  ;; label = @4
            get_local 1
            i32.const -1
            i32.add
            tee_local 1
            i32.eqz
            br_if 1 (;@3;)
            get_local 0
            i32.load offset=24
            get_local 0
            i32.load offset=4
            get_local 0
            i32.load offset=28
            i32.load offset=16
            call_indirect (type 2)
            i32.eqz
            br_if 0 (;@4;)
          end
          i32.const 1
          return
        end
        get_local 0
        i32.load offset=4
        set_local 10
        i32.const 1
        set_local 1
        get_local 0
        i32.load offset=24
        get_local 4
        get_local 5
        get_local 0
        i32.load offset=28
        i32.load offset=12
        call_indirect (type 1)
        br_if 1 (;@1;)
        get_local 3
        i32.const 1
        i32.add
        set_local 9
        get_local 0
        i32.load offset=28
        set_local 3
        get_local 0
        i32.load offset=24
        set_local 2
        block  ;; label = @3
          loop  ;; label = @4
            get_local 9
            i32.const -1
            i32.add
            tee_local 9
            i32.eqz
            br_if 1 (;@3;)
            i32.const 1
            set_local 1
            get_local 2
            get_local 10
            get_local 3
            i32.load offset=16
            call_indirect (type 2)
            br_if 3 (;@1;)
            br 0 (;@4;)
          end
        end
        get_local 0
        get_local 11
        i32.store8 offset=32
        get_local 0
        get_local 6
        i32.store offset=4
        i32.const 0
        return
      end
      get_local 0
      i32.load offset=4
      set_local 10
      i32.const 1
      set_local 1
      get_local 0
      get_local 7
      get_local 2
      get_local 3
      call $_ZN4core3fmt9Formatter12pad_integral12write_prefix17hac9bbd52894cffc0E
      br_if 0 (;@1;)
      get_local 0
      i32.load offset=24
      get_local 4
      get_local 5
      get_local 0
      i32.load offset=28
      i32.load offset=12
      call_indirect (type 1)
      br_if 0 (;@1;)
      get_local 8
      i32.const 1
      i32.add
      set_local 9
      get_local 0
      i32.load offset=28
      set_local 3
      get_local 0
      i32.load offset=24
      set_local 0
      loop  ;; label = @2
        block  ;; label = @3
          get_local 9
          i32.const -1
          i32.add
          tee_local 9
          br_if 0 (;@3;)
          i32.const 0
          return
        end
        i32.const 1
        set_local 1
        get_local 0
        get_local 10
        get_local 3
        i32.load offset=16
        call_indirect (type 2)
        i32.eqz
        br_if 0 (;@2;)
      end
    end
    get_local 1)
  (func $_ZN4core3fmt9Formatter12pad_integral12write_prefix17hac9bbd52894cffc0E (type 7) (param i32 i32 i32 i32) (result i32)
    (local i32)
    block  ;; label = @1
      block  ;; label = @2
        get_local 1
        i32.const 1114112
        i32.eq
        br_if 0 (;@2;)
        i32.const 1
        set_local 4
        get_local 0
        i32.load offset=24
        get_local 1
        get_local 0
        i32.const 28
        i32.add
        i32.load
        i32.load offset=16
        call_indirect (type 2)
        br_if 1 (;@1;)
      end
      block  ;; label = @2
        get_local 2
        br_if 0 (;@2;)
        i32.const 0
        return
      end
      get_local 0
      i32.load offset=24
      get_local 2
      get_local 3
      get_local 0
      i32.const 28
      i32.add
      i32.load
      i32.load offset=12
      call_indirect (type 1)
      set_local 4
    end
    get_local 4)
  (func $_ZN4core3fmt3num3imp7fmt_u6417h88aa9a282d13def7E (type 11) (param i64 i32 i32) (result i32)
    (local i32 i32 i64 i32 i32 i32)
    get_global 0
    i32.const 48
    i32.sub
    tee_local 3
    set_global 0
    i32.const 39
    set_local 4
    block  ;; label = @1
      block  ;; label = @2
        get_local 0
        i64.const 10000
        i64.ge_u
        br_if 0 (;@2;)
        get_local 0
        set_local 5
        br 1 (;@1;)
      end
      i32.const 39
      set_local 4
      loop  ;; label = @2
        get_local 3
        i32.const 9
        i32.add
        get_local 4
        i32.add
        tee_local 6
        i32.const -4
        i32.add
        get_local 0
        get_local 0
        i64.const 10000
        i64.div_u
        tee_local 5
        i64.const 10000
        i64.mul
        i64.sub
        i32.wrap/i64
        tee_local 7
        i32.const 65535
        i32.and
        i32.const 100
        i32.div_u
        tee_local 8
        i32.const 1
        i32.shl
        i32.const 1049024
        i32.add
        i32.load16_u align=1
        i32.store16 align=1
        get_local 6
        i32.const -2
        i32.add
        get_local 7
        get_local 8
        i32.const 100
        i32.mul
        i32.sub
        i32.const 65535
        i32.and
        i32.const 1
        i32.shl
        i32.const 1049024
        i32.add
        i32.load16_u align=1
        i32.store16 align=1
        get_local 4
        i32.const -4
        i32.add
        set_local 4
        get_local 0
        i64.const 99999999
        i64.gt_u
        set_local 6
        get_local 5
        set_local 0
        get_local 6
        br_if 0 (;@2;)
      end
    end
    block  ;; label = @1
      get_local 5
      i32.wrap/i64
      tee_local 6
      i32.const 99
      i32.le_s
      br_if 0 (;@1;)
      get_local 3
      i32.const 9
      i32.add
      get_local 4
      i32.const -2
      i32.add
      tee_local 4
      i32.add
      get_local 5
      i32.wrap/i64
      tee_local 6
      get_local 6
      i32.const 65535
      i32.and
      i32.const 100
      i32.div_u
      tee_local 6
      i32.const 100
      i32.mul
      i32.sub
      i32.const 65535
      i32.and
      i32.const 1
      i32.shl
      i32.const 1049024
      i32.add
      i32.load16_u align=1
      i32.store16 align=1
    end
    block  ;; label = @1
      block  ;; label = @2
        get_local 6
        i32.const 10
        i32.lt_s
        br_if 0 (;@2;)
        get_local 3
        i32.const 9
        i32.add
        get_local 4
        i32.const -2
        i32.add
        tee_local 4
        i32.add
        get_local 6
        i32.const 1
        i32.shl
        i32.const 1049024
        i32.add
        i32.load16_u align=1
        i32.store16 align=1
        br 1 (;@1;)
      end
      get_local 3
      i32.const 9
      i32.add
      get_local 4
      i32.const -1
      i32.add
      tee_local 4
      i32.add
      get_local 6
      i32.const 48
      i32.add
      i32.store8
    end
    get_local 2
    get_local 1
    i32.const 1048940
    i32.const 0
    get_local 3
    i32.const 9
    i32.add
    get_local 4
    i32.add
    i32.const 39
    get_local 4
    i32.sub
    call $_ZN4core3fmt9Formatter12pad_integral17h45c2ae804e73f116E
    set_local 4
    get_local 3
    i32.const 48
    i32.add
    set_global 0
    get_local 4)
  (func $memset (type 1) (param i32 i32 i32) (result i32)
    (local i32)
    block  ;; label = @1
      get_local 2
      i32.eqz
      br_if 0 (;@1;)
      get_local 0
      set_local 3
      loop  ;; label = @2
        get_local 3
        get_local 1
        i32.store8
        get_local 3
        i32.const 1
        i32.add
        set_local 3
        get_local 2
        i32.const -1
        i32.add
        tee_local 2
        br_if 0 (;@2;)
      end
    end
    get_local 0)
  (func $memcpy (type 1) (param i32 i32 i32) (result i32)
    (local i32)
    block  ;; label = @1
      get_local 2
      i32.eqz
      br_if 0 (;@1;)
      get_local 0
      set_local 3
      loop  ;; label = @2
        get_local 3
        get_local 1
        i32.load8_u
        i32.store8
        get_local 3
        i32.const 1
        i32.add
        set_local 3
        get_local 1
        i32.const 1
        i32.add
        set_local 1
        get_local 2
        i32.const -1
        i32.add
        tee_local 2
        br_if 0 (;@2;)
      end
    end
    get_local 0)
  (func $log2 (type 12) (param f64) (result f64)
    (local i64 i32 i32 i64 f64 f64 f64 f64 f64)
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            get_local 0
            i64.reinterpret/f64
            tee_local 1
            i64.const 0
            i64.lt_s
            br_if 0 (;@4;)
            get_local 1
            i64.const 32
            i64.shr_u
            i32.wrap/i64
            tee_local 2
            i32.const 1048576
            i32.lt_u
            br_if 0 (;@4;)
            get_local 2
            i32.const 2146435071
            i32.gt_u
            br_if 1 (;@3;)
            i32.const -1023
            set_local 3
            get_local 1
            i64.const 4294967295
            i64.and
            tee_local 1
            i64.const 0
            i64.ne
            br_if 3 (;@1;)
            f64.const 0x0p+0 (;=0;)
            set_local 0
            get_local 2
            i32.const 1072693248
            i32.eq
            br_if 1 (;@3;)
            br 3 (;@1;)
          end
          get_local 1
          i64.const 9223372036854775807
          i64.and
          i64.eqz
          br_if 1 (;@2;)
          block  ;; label = @4
            get_local 1
            i64.const 0
            i64.lt_s
            br_if 0 (;@4;)
            get_local 0
            f64.const 0x1p+54 (;=1.80144e+16;)
            f64.mul
            i64.reinterpret/f64
            tee_local 4
            i64.const 4294967295
            i64.and
            set_local 1
            get_local 4
            i64.const 32
            i64.shr_u
            i32.wrap/i64
            set_local 2
            i32.const -1077
            set_local 3
            br 3 (;@1;)
          end
          get_local 0
          get_local 0
          f64.sub
          f64.const 0x0p+0 (;=0;)
          f64.div
          set_local 0
        end
        get_local 0
        return
      end
      f64.const -0x1p+0 (;=-1;)
      get_local 0
      get_local 0
      f64.mul
      f64.div
      return
    end
    get_local 2
    i32.const 614242
    i32.add
    tee_local 2
    i32.const 1048575
    i32.and
    i32.const 1072079006
    i32.add
    i64.extend_u/i32
    i64.const 32
    i64.shl
    get_local 1
    i64.or
    f64.reinterpret/i64
    f64.const -0x1p+0 (;=-1;)
    f64.add
    tee_local 0
    get_local 0
    get_local 0
    f64.const 0x1p-1 (;=0.5;)
    f64.mul
    f64.mul
    tee_local 5
    f64.sub
    i64.reinterpret/f64
    i64.const -4294967296
    i64.and
    f64.reinterpret/i64
    tee_local 6
    f64.const 0x1.71547652p+0 (;=1.4427;)
    f64.mul
    tee_local 7
    get_local 2
    i32.const 20
    i32.shr_u
    get_local 3
    i32.add
    f64.convert_s/i32
    tee_local 8
    f64.add
    tee_local 9
    get_local 7
    get_local 8
    get_local 9
    f64.sub
    f64.add
    get_local 0
    get_local 6
    f64.sub
    get_local 5
    f64.sub
    get_local 0
    get_local 0
    f64.const 0x1p+1 (;=2;)
    f64.add
    f64.div
    tee_local 0
    get_local 5
    get_local 0
    get_local 0
    f64.mul
    tee_local 7
    get_local 7
    f64.mul
    tee_local 0
    get_local 0
    get_local 0
    f64.const 0x1.39a09d078c69fp-3 (;=0.153138;)
    f64.mul
    f64.const 0x1.c71c51d8e78afp-3 (;=0.222222;)
    f64.add
    f64.mul
    f64.const 0x1.999999997fa04p-2 (;=0.4;)
    f64.add
    f64.mul
    get_local 7
    get_local 0
    get_local 0
    get_local 0
    f64.const 0x1.2f112df3e5244p-3 (;=0.147982;)
    f64.mul
    f64.const 0x1.7466496cb03dep-3 (;=0.181836;)
    f64.add
    f64.mul
    f64.const 0x1.2492494229359p-2 (;=0.285714;)
    f64.add
    f64.mul
    f64.const 0x1.5555555555593p-1 (;=0.666667;)
    f64.add
    f64.mul
    f64.add
    f64.add
    f64.mul
    f64.add
    tee_local 0
    f64.const 0x1.71547652p+0 (;=1.4427;)
    f64.mul
    get_local 0
    get_local 6
    f64.add
    f64.const 0x1.705fc2eefa2p-33 (;=1.67517e-10;)
    f64.mul
    f64.add
    f64.add
    f64.add)
  (table (;0;) 20 20 anyfunc)
  (memory (;0;) 17)
  (global (;0;) (mut i32) (i32.const 1048576))
  (global (;1;) i32 (i32.const 1049760))
  (global (;2;) i32 (i32.const 1049760))
  (export "memory" (memory 0))
  (export "nth_prime" (func $nth_prime))
  (export "is_prime" (func $is_prime))
  (export "assert_prime" (func $assert_prime))
  (export "assert_not_prime" (func $assert_not_prime))
  (export "__data_end" (global 1))
  (export "__heap_base" (global 2))
  (elem (i32.const 1) $_ZN4core3fmt3num3imp52_$LT$impl$u20$core..fmt..Display$u20$for$u20$u32$GT$3fmt17h99ce8059c5fdd42bE $_ZN4core3ptr13drop_in_place17h3ede11bfa89f9051E $_ZN91_$LT$std..panicking..begin_panic..PanicPayload$LT$A$GT$$u20$as$u20$core..panic..BoxMeUp$GT$8take_box17hde9b5c896fde66d8E $_ZN91_$LT$std..panicking..begin_panic..PanicPayload$LT$A$GT$$u20$as$u20$core..panic..BoxMeUp$GT$3get17h4739c367a12a045cE $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h9f9141c4d3b51f22E $_ZN3std5alloc24default_alloc_error_hook17h5f799078d47b0575E $_ZN4core3ptr13drop_in_place17h091c0eb23df207dbE $_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$9write_str17h2561a40bc4fb773bE $_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17h65719fab9c99edbbE $_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$9write_fmt17h1e75a79353b91b58E $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h9ba061566a081877E $_ZN4core3ptr13drop_in_place17hfda159fcd4de37a1E $_ZN90_$LT$std..panicking..begin_panic_handler..PanicPayload$u20$as$u20$core..panic..BoxMeUp$GT$8take_box17h7fbe7fd4ced85654E $_ZN90_$LT$std..panicking..begin_panic_handler..PanicPayload$u20$as$u20$core..panic..BoxMeUp$GT$3get17h6462774f78ab3653E $_ZN4core3ptr13drop_in_place17h0a661afa028ee03bE $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h793a77747fda2e92E $_ZN4core3ops8function6FnOnce9call_once17h8f9c18931d78bf61E $_ZN4core3ptr13drop_in_place17h00410729a46e8620E $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17hd56d7694ecea05efE)
  (data (i32.const 1048576) ", \00\00\00\00\10\00\00\00\00\00\00\00\10\00\02\00\00\00src/lib.rs\00\00\14\00\10\00\0a\00\00\006\00\00\00 \00\00\00explicit panic\00\00\14\00\10\00\0a\00\00\00L\00\00\00\09\00\00\00\14\00\10\00\0a\00\00\00S\00\00\00\09\00\00\00\02\00\00\00\08\00\00\00\04\00\00\00\03\00\00\00\04\00\00\00\02\00\00\00\08\00\00\00\04\00\00\00\05\00\00\00\07\00\00\00\04\00\00\00\04\00\00\00\08\00\00\00\09\00\00\00\0a\00\00\00\07\00\00\00\00\00\00\00\01\00\00\00\0b\00\00\00called `Option::unwrap()` on a `None` valuesrc/libstd/panicking.rs\00\00\d7\00\10\00\17\00\00\00x\01\00\00\0f\00\00\00\d7\00\10\00\17\00\00\00y\01\00\00\0f\00\00\00\0c\00\00\00\10\00\00\00\04\00\00\00\0d\00\00\00\0e\00\00\00\0f\00\00\00\0c\00\00\00\04\00\00\00\10\00\00\00src/liballoc/raw_vec.rscapacity overflow4\01\10\00\17\00\00\00\f0\02\00\00\05\00\00\00\12\00\00\00\00\00\00\00\01\00\00\00\13\00\00\00index out of bounds: the len is  but the index is \00\00|\01\10\00 \00\00\00\9c\01\10\00\12\00\00\0000010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899src/libcore/fmt/mod.rs\00\00\88\02\10\00\16\00\00\00F\04\00\00\11\00\00\00\88\02\10\00\16\00\00\00P\04\00\00$\00\00\00")
  (data (i32.const 1049280) "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"))

(assert_return (invoke "is_prime" (i32.const 0x3e7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3e6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3e5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3e4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3e3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3e2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3e1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3e0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3df)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3de)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3dd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3dc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3db)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3da)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3d6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d1)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3d0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3cf)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3ce)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3cd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3cc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3cb)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3ca)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3c6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3c0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3bf)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3be)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3bd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3bc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3bb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3ba)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b9)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3b8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3b2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3af)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3ae)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3ad)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3ac)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3ab)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3aa)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3a9)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3a8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3a7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3a6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3a5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3a4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3a3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3a2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3a1)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3a0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x39f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x39e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x39d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x39c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x39b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x39a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x399)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x398)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x397)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x396)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x395)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x394)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x393)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x392)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x391)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x390)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x38f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x38e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x38d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x38c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x38b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x38a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x389)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x388)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x387)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x386)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x385)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x384)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x383)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x382)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x381)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x380)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x37f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x37e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x37d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x37c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x37b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x37a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x379)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x378)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x377)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x376)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x375)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x374)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x373)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x372)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x371)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x370)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x36f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x36e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x36d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x36c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x36b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x36a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x369)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x368)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x367)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x366)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x365)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x364)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x363)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x362)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x361)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x360)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x35f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x35e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x35d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x35c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x35b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x35a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x359)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x358)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x357)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x356)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x355)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x354)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x353)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x352)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x351)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x350)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x34f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x34e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x34d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x34c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x34b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x34a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x349)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x348)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x347)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x346)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x345)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x344)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x343)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x342)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x341)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x340)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x33f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x33e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x33d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x33c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x33b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x33a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x339)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x338)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x337)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x336)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x335)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x334)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x333)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x332)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x331)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x330)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x32f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x32e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x32d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x32c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x32b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x32a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x329)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x328)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x327)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x326)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x325)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x324)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x323)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x322)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x321)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x320)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x31f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x31e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x31d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x31c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x31b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x31a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x319)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x318)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x317)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x316)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x315)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x314)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x313)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x312)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x311)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x310)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x30f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x30e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x30d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x30c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x30b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x30a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x309)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x308)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x307)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x306)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x305)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x304)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x303)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x302)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x301)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x300)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ff)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2fe)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2fd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2fc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2fb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2fa)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f9)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2f8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2f4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ef)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2ee)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ed)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ec)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2eb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ea)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2e9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2e8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2e7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2e6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2e5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2e4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2e3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2e2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2e1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2e0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2df)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2de)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2dd)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2dc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2db)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2da)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2d6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2cf)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2ce)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2cd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2cc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2cb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ca)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2c4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2bf)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2be)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2bd)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2bc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2bb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ba)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2b2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2af)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ae)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ad)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ac)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2ab)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2aa)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2a9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2a8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2a7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2a6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2a5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2a4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2a3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2a2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2a1)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2a0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x29f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x29e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x29d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x29c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x29b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x29a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x299)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x298)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x297)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x296)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x295)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x294)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x293)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x292)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x291)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x290)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x28f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x28e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x28d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x28c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x28b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x28a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x289)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x288)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x287)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x286)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x285)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x284)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x283)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x282)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x281)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x280)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x27f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x27e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x27d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x27c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x27b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x27a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x279)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x278)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x277)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x276)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x275)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x274)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x273)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x272)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x271)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x270)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x26f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x26e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x26d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x26c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x26b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x26a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x269)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x268)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x267)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x266)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x265)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x264)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x263)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x262)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x261)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x260)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x25f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x25e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x25d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x25c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x25b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x25a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x259)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x258)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x257)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x256)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x255)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x254)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x253)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x252)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x251)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x250)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x24f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x24e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x24d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x24c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x24b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x24a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x249)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x248)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x247)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x246)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x245)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x244)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x243)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x242)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x241)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x240)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x23f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x23e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x23d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x23c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x23b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x23a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x239)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x238)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x237)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x236)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x235)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x234)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x233)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x232)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x231)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x230)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x22f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x22e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x22d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x22c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x22b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x22a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x229)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x228)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x227)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x226)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x225)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x224)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x223)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x222)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x221)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x220)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x21f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x21e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x21d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x21c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x21b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x21a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x219)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x218)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x217)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x216)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x215)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x214)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x213)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x212)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x211)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x210)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x20f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x20e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x20d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x20c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x20b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x20a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x209)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x208)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x207)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x206)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x205)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x204)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x203)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x202)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x201)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x200)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ff)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1fe)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1fd)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1fc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1fb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1fa)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1f6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1f2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ef)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ee)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ed)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ec)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1eb)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1ea)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1e6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1e0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1df)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1de)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1dd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1dc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1db)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1da)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1d2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1cf)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1ce)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1cd)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1cc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1cb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ca)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1c9)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1c8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1c7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1c6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1c5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1c4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1c3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1c2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1c1)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1c0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1bf)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1be)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1bd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1bc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1bb)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1ba)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1b6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b1)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1b0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1af)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1ae)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ad)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ac)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1ab)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1aa)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1a4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1a2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x19f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x19e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x19d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x19c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x19b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x19a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x199)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x198)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x197)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x196)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x195)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x194)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x193)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x192)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x191)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x190)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x18f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x18e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x18d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x18c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x18b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x18a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x189)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x188)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x187)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x186)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x185)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x184)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x183)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x182)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x181)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x180)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x17f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x17e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x17d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x17c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x17b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x17a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x179)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x178)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x177)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x176)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x175)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x174)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x173)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x172)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x171)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x170)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x16f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x16e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x16d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x16c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x16b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x16a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x169)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x168)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x167)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x166)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x165)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x164)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x163)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x162)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x161)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x160)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x15f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x15e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x15d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x15c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x15b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x15a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x159)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x158)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x157)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x156)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x155)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x154)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x153)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x152)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x151)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x150)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x14f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x14e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x14d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x14c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x14b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x14a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x149)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x148)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x147)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x146)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x145)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x144)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x143)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x142)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x141)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x140)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x13f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x13e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x13d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x13c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x13b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x13a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x139)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x138)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x137)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x136)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x135)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x134)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x133)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x132)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x131)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x130)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x12f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x12e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x12d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x12c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x12b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x12a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x129)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x128)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x127)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x126)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x125)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x124)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x123)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x122)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x121)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x120)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x11f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x11e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x11d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x11c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x11b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x11a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x119)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x118)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x117)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x116)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x115)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x114)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x113)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x112)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x111)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x110)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x10f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x10e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x10d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x10c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x10b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x10a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x109)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x108)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x107)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x106)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x105)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x104)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x103)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x102)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x101)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x100)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xff)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xfe)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xfd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xfc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xfb)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xfa)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf1)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xf0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xef)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xee)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xed)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xec)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xeb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xea)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xe9)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xe8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xe7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xe6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xe5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xe4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xe3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xe2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xe1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xe0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xdf)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xde)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xdd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xdc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xdb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xda)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xd2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xcf)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xce)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xcd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xcc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xcb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xca)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xc9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xc8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xc7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xc6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xc5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xc4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xc3)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xc2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xc1)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xc0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xbf)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xbe)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xbd)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xbc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xbb)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xba)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb7)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xb4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xb2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xaf)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xae)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xad)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xac)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xab)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xaa)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xa9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xa8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xa7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xa6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xa5)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xa4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xa3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xa2)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xa1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xa0)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x9f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x9e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x9d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x9c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x9b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x9a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x99)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x98)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x97)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x96)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x95)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x94)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x93)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x92)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x91)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x90)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x8f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x8e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x8d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x8c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x8b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x8a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x89)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x88)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x87)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x86)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x85)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x84)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x83)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x82)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x81)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x80)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x7f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x7e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x7d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x7c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x7b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x7a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x79)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x78)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x77)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x76)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x75)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x74)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x73)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x72)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x71)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x70)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x6f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x6e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x6d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x6c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x6b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x6a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x69)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x68)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x67)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x66)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x65)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x64)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x63)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x62)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x61)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x60)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x5f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x5e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x5d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x5c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x5b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x5a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x59)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x58)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x57)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x56)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x55)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x54)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x53)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x52)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x51)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x50)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x4f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x4e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x4d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x4c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x4b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x4a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x49)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x48)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x47)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x46)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x45)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x44)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x43)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x42)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x41)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x40)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3f)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x3a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x39)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x38)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x37)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x36)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x35)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x34)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x33)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x32)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x31)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x30)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2d)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x2b)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x29)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x28)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x27)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x26)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x25)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x24)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x23)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x22)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x21)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x20)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1f)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1e)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1d)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1c)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1b)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x1a)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x19)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x18)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x17)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x16)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x15)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x14)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x13)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x12)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x11)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x10)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xf)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xe)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xd)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xc)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0xb)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0xa)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x9)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x8)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x7)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x6)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x5)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x4)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x3)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x2)) (i32.const 0x1))
(assert_return (invoke "is_prime" (i32.const 0x1)) (i32.const 0x0))
(assert_return (invoke "is_prime" (i32.const 0x0)) (i32.const 0x0))
(assert_return (invoke "nth_prime" (i32.const 0x0)) (i32.const 0x2))
(assert_return (invoke "nth_prime" (i32.const 0x1)) (i32.const 0x3))
(assert_return (invoke "nth_prime" (i32.const 0x2)) (i32.const 0x5))
(assert_return (invoke "nth_prime" (i32.const 0x3)) (i32.const 0x7))
(assert_return (invoke "nth_prime" (i32.const 0x4)) (i32.const 0xb))
(assert_return (invoke "nth_prime" (i32.const 0x5)) (i32.const 0xd))
(assert_return (invoke "nth_prime" (i32.const 0x6)) (i32.const 0x11))
(assert_return (invoke "nth_prime" (i32.const 0x7)) (i32.const 0x13))
(assert_return (invoke "nth_prime" (i32.const 0x8)) (i32.const 0x17))
(assert_return (invoke "nth_prime" (i32.const 0x9)) (i32.const 0x1d))
(assert_return (invoke "nth_prime" (i32.const 0xa)) (i32.const 0x1f))
(assert_return (invoke "nth_prime" (i32.const 0xb)) (i32.const 0x25))
(assert_return (invoke "nth_prime" (i32.const 0xc)) (i32.const 0x29))
(assert_return (invoke "nth_prime" (i32.const 0xd)) (i32.const 0x2b))
(assert_return (invoke "nth_prime" (i32.const 0xe)) (i32.const 0x2f))
(assert_return (invoke "nth_prime" (i32.const 0xf)) (i32.const 0x35))
(assert_return (invoke "nth_prime" (i32.const 0x10)) (i32.const 0x3b))
(assert_return (invoke "nth_prime" (i32.const 0x11)) (i32.const 0x3d))
(assert_return (invoke "nth_prime" (i32.const 0x12)) (i32.const 0x43))
(assert_return (invoke "nth_prime" (i32.const 0x13)) (i32.const 0x47))
(assert_return (invoke "nth_prime" (i32.const 0x14)) (i32.const 0x49))
(assert_return (invoke "nth_prime" (i32.const 0x15)) (i32.const 0x4f))
(assert_return (invoke "nth_prime" (i32.const 0x16)) (i32.const 0x53))
(assert_return (invoke "nth_prime" (i32.const 0x17)) (i32.const 0x59))
(assert_return (invoke "nth_prime" (i32.const 0x18)) (i32.const 0x61))
(assert_return (invoke "nth_prime" (i32.const 0x19)) (i32.const 0x65))
(assert_return (invoke "nth_prime" (i32.const 0x1a)) (i32.const 0x67))
(assert_return (invoke "nth_prime" (i32.const 0x1b)) (i32.const 0x6b))
(assert_return (invoke "nth_prime" (i32.const 0x1c)) (i32.const 0x6d))
(assert_return (invoke "nth_prime" (i32.const 0x1d)) (i32.const 0x71))
(assert_return (invoke "nth_prime" (i32.const 0x1e)) (i32.const 0x7f))
(assert_return (invoke "nth_prime" (i32.const 0x1f)) (i32.const 0x83))
(assert_return (invoke "nth_prime" (i32.const 0x20)) (i32.const 0x89))
(assert_return (invoke "nth_prime" (i32.const 0x21)) (i32.const 0x8b))
(assert_return (invoke "nth_prime" (i32.const 0x22)) (i32.const 0x95))
(assert_return (invoke "nth_prime" (i32.const 0x23)) (i32.const 0x97))
(assert_return (invoke "nth_prime" (i32.const 0x24)) (i32.const 0x9d))
(assert_return (invoke "nth_prime" (i32.const 0x25)) (i32.const 0xa3))
(assert_return (invoke "nth_prime" (i32.const 0x26)) (i32.const 0xa7))
(assert_return (invoke "nth_prime" (i32.const 0x27)) (i32.const 0xad))
(assert_return (invoke "nth_prime" (i32.const 0x28)) (i32.const 0xb3))
(assert_return (invoke "nth_prime" (i32.const 0x29)) (i32.const 0xb5))
(assert_return (invoke "nth_prime" (i32.const 0x2a)) (i32.const 0xbf))
(assert_return (invoke "nth_prime" (i32.const 0x2b)) (i32.const 0xc1))
(assert_return (invoke "nth_prime" (i32.const 0x2c)) (i32.const 0xc5))
(assert_return (invoke "nth_prime" (i32.const 0x2d)) (i32.const 0xc7))
(assert_return (invoke "nth_prime" (i32.const 0x2e)) (i32.const 0xd3))
(assert_return (invoke "nth_prime" (i32.const 0x2f)) (i32.const 0xdf))
(assert_return (invoke "nth_prime" (i32.const 0x30)) (i32.const 0xe3))
(assert_return (invoke "nth_prime" (i32.const 0x31)) (i32.const 0xe5))
(assert_return (invoke "nth_prime" (i32.const 0x32)) (i32.const 0xe9))
(assert_return (invoke "nth_prime" (i32.const 0x33)) (i32.const 0xef))
(assert_return (invoke "nth_prime" (i32.const 0x34)) (i32.const 0xf1))
(assert_return (invoke "nth_prime" (i32.const 0x35)) (i32.const 0xfb))
(assert_return (invoke "nth_prime" (i32.const 0x36)) (i32.const 0x101))
(assert_return (invoke "nth_prime" (i32.const 0x37)) (i32.const 0x107))
(assert_return (invoke "nth_prime" (i32.const 0x38)) (i32.const 0x10d))
(assert_return (invoke "nth_prime" (i32.const 0x39)) (i32.const 0x10f))
(assert_return (invoke "nth_prime" (i32.const 0x3a)) (i32.const 0x115))
(assert_return (invoke "nth_prime" (i32.const 0x3b)) (i32.const 0x119))
(assert_return (invoke "nth_prime" (i32.const 0x3c)) (i32.const 0x11b))
(assert_return (invoke "nth_prime" (i32.const 0x3d)) (i32.const 0x125))
(assert_return (invoke "nth_prime" (i32.const 0x3e)) (i32.const 0x133))
(assert_return (invoke "nth_prime" (i32.const 0x3f)) (i32.const 0x137))
(assert_return (invoke "nth_prime" (i32.const 0x40)) (i32.const 0x139))
(assert_return (invoke "nth_prime" (i32.const 0x41)) (i32.const 0x13d))
(assert_return (invoke "nth_prime" (i32.const 0x42)) (i32.const 0x14b))
(assert_return (invoke "nth_prime" (i32.const 0x43)) (i32.const 0x151))
(assert_return (invoke "nth_prime" (i32.const 0x44)) (i32.const 0x15b))
(assert_return (invoke "nth_prime" (i32.const 0x45)) (i32.const 0x15d))
(assert_return (invoke "nth_prime" (i32.const 0x46)) (i32.const 0x161))
(assert_return (invoke "nth_prime" (i32.const 0x47)) (i32.const 0x167))
(assert_return (invoke "nth_prime" (i32.const 0x48)) (i32.const 0x16f))
(assert_return (invoke "nth_prime" (i32.const 0x49)) (i32.const 0x175))
(assert_return (invoke "nth_prime" (i32.const 0x4a)) (i32.const 0x17b))
(assert_return (invoke "nth_prime" (i32.const 0x4b)) (i32.const 0x17f))
(assert_return (invoke "nth_prime" (i32.const 0x4c)) (i32.const 0x185))
(assert_return (invoke "nth_prime" (i32.const 0x4d)) (i32.const 0x18d))
(assert_return (invoke "nth_prime" (i32.const 0x4e)) (i32.const 0x191))
(assert_return (invoke "nth_prime" (i32.const 0x4f)) (i32.const 0x199))
(assert_return (invoke "nth_prime" (i32.const 0x50)) (i32.const 0x1a3))
(assert_return (invoke "nth_prime" (i32.const 0x51)) (i32.const 0x1a5))
(assert_return (invoke "nth_prime" (i32.const 0x52)) (i32.const 0x1af))
(assert_return (invoke "nth_prime" (i32.const 0x53)) (i32.const 0x1b1))
(assert_return (invoke "nth_prime" (i32.const 0x54)) (i32.const 0x1b7))
(assert_return (invoke "nth_prime" (i32.const 0x55)) (i32.const 0x1bb))
(assert_return (invoke "nth_prime" (i32.const 0x56)) (i32.const 0x1c1))
(assert_return (invoke "nth_prime" (i32.const 0x57)) (i32.const 0x1c9))
(assert_return (invoke "nth_prime" (i32.const 0x58)) (i32.const 0x1cd))
(assert_return (invoke "nth_prime" (i32.const 0x59)) (i32.const 0x1cf))
(assert_return (invoke "nth_prime" (i32.const 0x5a)) (i32.const 0x1d3))
(assert_return (invoke "nth_prime" (i32.const 0x5b)) (i32.const 0x1df))
(assert_return (invoke "nth_prime" (i32.const 0x5c)) (i32.const 0x1e7))
(assert_return (invoke "nth_prime" (i32.const 0x5d)) (i32.const 0x1eb))
(assert_return (invoke "nth_prime" (i32.const 0x5e)) (i32.const 0x1f3))
(assert_return (invoke "nth_prime" (i32.const 0x5f)) (i32.const 0x1f7))
(assert_return (invoke "nth_prime" (i32.const 0x60)) (i32.const 0x1fd))
(assert_return (invoke "nth_prime" (i32.const 0x61)) (i32.const 0x209))
(assert_return (invoke "nth_prime" (i32.const 0x62)) (i32.const 0x20b))
(assert_return (invoke "nth_prime" (i32.const 0x63)) (i32.const 0x21d))
(assert_return (invoke "nth_prime" (i32.const 0x64)) (i32.const 0x223))
(assert_return (invoke "nth_prime" (i32.const 0x65)) (i32.const 0x22d))
(assert_return (invoke "nth_prime" (i32.const 0x66)) (i32.const 0x233))
(assert_return (invoke "nth_prime" (i32.const 0x67)) (i32.const 0x239))
(assert_return (invoke "nth_prime" (i32.const 0x68)) (i32.const 0x23b))
(assert_return (invoke "nth_prime" (i32.const 0x69)) (i32.const 0x241))
(assert_return (invoke "nth_prime" (i32.const 0x6a)) (i32.const 0x24b))
(assert_return (invoke "nth_prime" (i32.const 0x6b)) (i32.const 0x251))
(assert_return (invoke "nth_prime" (i32.const 0x6c)) (i32.const 0x257))
(assert_return (invoke "nth_prime" (i32.const 0x6d)) (i32.const 0x259))
(assert_return (invoke "nth_prime" (i32.const 0x6e)) (i32.const 0x25f))
(assert_return (invoke "nth_prime" (i32.const 0x6f)) (i32.const 0x265))
(assert_return (invoke "nth_prime" (i32.const 0x70)) (i32.const 0x269))
(assert_return (invoke "nth_prime" (i32.const 0x71)) (i32.const 0x26b))
(assert_return (invoke "nth_prime" (i32.const 0x72)) (i32.const 0x277))
(assert_return (invoke "nth_prime" (i32.const 0x73)) (i32.const 0x281))
(assert_return (invoke "nth_prime" (i32.const 0x74)) (i32.const 0x283))
(assert_return (invoke "nth_prime" (i32.const 0x75)) (i32.const 0x287))
(assert_return (invoke "nth_prime" (i32.const 0x76)) (i32.const 0x28d))
(assert_return (invoke "nth_prime" (i32.const 0x77)) (i32.const 0x293))
(assert_return (invoke "nth_prime" (i32.const 0x78)) (i32.const 0x295))
(assert_return (invoke "nth_prime" (i32.const 0x79)) (i32.const 0x2a1))
(assert_return (invoke "nth_prime" (i32.const 0x7a)) (i32.const 0x2a5))
(assert_return (invoke "nth_prime" (i32.const 0x7b)) (i32.const 0x2ab))
(assert_return (invoke "nth_prime" (i32.const 0x7c)) (i32.const 0x2b3))
(assert_return (invoke "nth_prime" (i32.const 0x7d)) (i32.const 0x2bd))
(assert_return (invoke "nth_prime" (i32.const 0x7e)) (i32.const 0x2c5))
(assert_return (invoke "nth_prime" (i32.const 0x7f)) (i32.const 0x2cf))
(assert_return (invoke "nth_prime" (i32.const 0x80)) (i32.const 0x2d7))
(assert_return (invoke "nth_prime" (i32.const 0x81)) (i32.const 0x2dd))
(assert_return (invoke "nth_prime" (i32.const 0x82)) (i32.const 0x2e3))
(assert_return (invoke "nth_prime" (i32.const 0x83)) (i32.const 0x2e7))
(assert_return (invoke "nth_prime" (i32.const 0x84)) (i32.const 0x2ef))
(assert_return (invoke "nth_prime" (i32.const 0x85)) (i32.const 0x2f5))
(assert_return (invoke "nth_prime" (i32.const 0x86)) (i32.const 0x2f9))
(assert_return (invoke "nth_prime" (i32.const 0x87)) (i32.const 0x301))
(assert_return (invoke "nth_prime" (i32.const 0x88)) (i32.const 0x305))
(assert_return (invoke "nth_prime" (i32.const 0x89)) (i32.const 0x313))
(assert_return (invoke "nth_prime" (i32.const 0x8a)) (i32.const 0x31d))
(assert_return (invoke "nth_prime" (i32.const 0x8b)) (i32.const 0x329))
(assert_return (invoke "nth_prime" (i32.const 0x8c)) (i32.const 0x32b))
(assert_return (invoke "nth_prime" (i32.const 0x8d)) (i32.const 0x335))
(assert_return (invoke "nth_prime" (i32.const 0x8e)) (i32.const 0x337))
(assert_return (invoke "nth_prime" (i32.const 0x8f)) (i32.const 0x33b))
(assert_return (invoke "nth_prime" (i32.const 0x90)) (i32.const 0x33d))
(assert_return (invoke "nth_prime" (i32.const 0x91)) (i32.const 0x347))
(assert_return (invoke "nth_prime" (i32.const 0x92)) (i32.const 0x355))
(assert_return (invoke "nth_prime" (i32.const 0x93)) (i32.const 0x359))
(assert_return (invoke "nth_prime" (i32.const 0x94)) (i32.const 0x35b))
(assert_return (invoke "nth_prime" (i32.const 0x95)) (i32.const 0x35f))
(assert_return (invoke "nth_prime" (i32.const 0x96)) (i32.const 0x36d))
(assert_return (invoke "nth_prime" (i32.const 0x97)) (i32.const 0x371))
(assert_return (invoke "nth_prime" (i32.const 0x98)) (i32.const 0x373))
(assert_return (invoke "nth_prime" (i32.const 0x99)) (i32.const 0x377))
(assert_return (invoke "nth_prime" (i32.const 0x9a)) (i32.const 0x38b))
(assert_return (invoke "nth_prime" (i32.const 0x9b)) (i32.const 0x38f))
(assert_return (invoke "nth_prime" (i32.const 0x9c)) (i32.const 0x397))
(assert_return (invoke "nth_prime" (i32.const 0x9d)) (i32.const 0x3a1))
(assert_return (invoke "nth_prime" (i32.const 0x9e)) (i32.const 0x3a9))
(assert_return (invoke "nth_prime" (i32.const 0x9f)) (i32.const 0x3ad))
(assert_return (invoke "nth_prime" (i32.const 0xa0)) (i32.const 0x3b3))
(assert_return (invoke "nth_prime" (i32.const 0xa1)) (i32.const 0x3b9))
(assert_return (invoke "nth_prime" (i32.const 0xa2)) (i32.const 0x3c7))
(assert_return (invoke "nth_prime" (i32.const 0xa3)) (i32.const 0x3cb))
(assert_return (invoke "nth_prime" (i32.const 0xa4)) (i32.const 0x3d1))
(assert_return (invoke "nth_prime" (i32.const 0xa5)) (i32.const 0x3d7))
(assert_return (invoke "nth_prime" (i32.const 0xa6)) (i32.const 0x3df))
(assert_return (invoke "nth_prime" (i32.const 0xa7)) (i32.const 0x3e5))
(assert_return (invoke "assert_not_prime" (i32.const 0x0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1)))
(assert_return (invoke "assert_prime" (i32.const 0x2)))
(assert_return (invoke "assert_prime" (i32.const 0x3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x4)))
(assert_return (invoke "assert_prime" (i32.const 0x5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x6)))
(assert_return (invoke "assert_prime" (i32.const 0x7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x9)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa)))
(assert_return (invoke "assert_prime" (i32.const 0xb)))
(assert_return (invoke "assert_not_prime" (i32.const 0xc)))
(assert_return (invoke "assert_prime" (i32.const 0xd)))
(assert_return (invoke "assert_not_prime" (i32.const 0xe)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf)))
(assert_return (invoke "assert_not_prime" (i32.const 0x10)))
(assert_return (invoke "assert_prime" (i32.const 0x11)))
(assert_return (invoke "assert_not_prime" (i32.const 0x12)))
(assert_return (invoke "assert_prime" (i32.const 0x13)))
(assert_return (invoke "assert_not_prime" (i32.const 0x14)))
(assert_return (invoke "assert_not_prime" (i32.const 0x15)))
(assert_return (invoke "assert_not_prime" (i32.const 0x16)))
(assert_return (invoke "assert_prime" (i32.const 0x17)))
(assert_return (invoke "assert_not_prime" (i32.const 0x18)))
(assert_return (invoke "assert_not_prime" (i32.const 0x19)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c)))
(assert_return (invoke "assert_prime" (i32.const 0x1d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e)))
(assert_return (invoke "assert_prime" (i32.const 0x1f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x20)))
(assert_return (invoke "assert_not_prime" (i32.const 0x21)))
(assert_return (invoke "assert_not_prime" (i32.const 0x22)))
(assert_return (invoke "assert_not_prime" (i32.const 0x23)))
(assert_return (invoke "assert_not_prime" (i32.const 0x24)))
(assert_return (invoke "assert_prime" (i32.const 0x25)))
(assert_return (invoke "assert_not_prime" (i32.const 0x26)))
(assert_return (invoke "assert_not_prime" (i32.const 0x27)))
(assert_return (invoke "assert_not_prime" (i32.const 0x28)))
(assert_return (invoke "assert_prime" (i32.const 0x29)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a)))
(assert_return (invoke "assert_prime" (i32.const 0x2b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e)))
(assert_return (invoke "assert_prime" (i32.const 0x2f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x30)))
(assert_return (invoke "assert_not_prime" (i32.const 0x31)))
(assert_return (invoke "assert_not_prime" (i32.const 0x32)))
(assert_return (invoke "assert_not_prime" (i32.const 0x33)))
(assert_return (invoke "assert_not_prime" (i32.const 0x34)))
(assert_return (invoke "assert_prime" (i32.const 0x35)))
(assert_return (invoke "assert_not_prime" (i32.const 0x36)))
(assert_return (invoke "assert_not_prime" (i32.const 0x37)))
(assert_return (invoke "assert_not_prime" (i32.const 0x38)))
(assert_return (invoke "assert_not_prime" (i32.const 0x39)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a)))
(assert_return (invoke "assert_prime" (i32.const 0x3b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c)))
(assert_return (invoke "assert_prime" (i32.const 0x3d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x40)))
(assert_return (invoke "assert_not_prime" (i32.const 0x41)))
(assert_return (invoke "assert_not_prime" (i32.const 0x42)))
(assert_return (invoke "assert_prime" (i32.const 0x43)))
(assert_return (invoke "assert_not_prime" (i32.const 0x44)))
(assert_return (invoke "assert_not_prime" (i32.const 0x45)))
(assert_return (invoke "assert_not_prime" (i32.const 0x46)))
(assert_return (invoke "assert_prime" (i32.const 0x47)))
(assert_return (invoke "assert_not_prime" (i32.const 0x48)))
(assert_return (invoke "assert_prime" (i32.const 0x49)))
(assert_return (invoke "assert_not_prime" (i32.const 0x4a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x4b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x4c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x4d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x4e)))
(assert_return (invoke "assert_prime" (i32.const 0x4f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x50)))
(assert_return (invoke "assert_not_prime" (i32.const 0x51)))
(assert_return (invoke "assert_not_prime" (i32.const 0x52)))
(assert_return (invoke "assert_prime" (i32.const 0x53)))
(assert_return (invoke "assert_not_prime" (i32.const 0x54)))
(assert_return (invoke "assert_not_prime" (i32.const 0x55)))
(assert_return (invoke "assert_not_prime" (i32.const 0x56)))
(assert_return (invoke "assert_not_prime" (i32.const 0x57)))
(assert_return (invoke "assert_not_prime" (i32.const 0x58)))
(assert_return (invoke "assert_prime" (i32.const 0x59)))
(assert_return (invoke "assert_not_prime" (i32.const 0x5a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x5b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x5c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x5d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x5e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x5f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x60)))
(assert_return (invoke "assert_prime" (i32.const 0x61)))
(assert_return (invoke "assert_not_prime" (i32.const 0x62)))
(assert_return (invoke "assert_not_prime" (i32.const 0x63)))
(assert_return (invoke "assert_not_prime" (i32.const 0x64)))
(assert_return (invoke "assert_prime" (i32.const 0x65)))
(assert_return (invoke "assert_not_prime" (i32.const 0x66)))
(assert_return (invoke "assert_prime" (i32.const 0x67)))
(assert_return (invoke "assert_not_prime" (i32.const 0x68)))
(assert_return (invoke "assert_not_prime" (i32.const 0x69)))
(assert_return (invoke "assert_not_prime" (i32.const 0x6a)))
(assert_return (invoke "assert_prime" (i32.const 0x6b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x6c)))
(assert_return (invoke "assert_prime" (i32.const 0x6d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x6e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x6f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x70)))
(assert_return (invoke "assert_prime" (i32.const 0x71)))
(assert_return (invoke "assert_not_prime" (i32.const 0x72)))
(assert_return (invoke "assert_not_prime" (i32.const 0x73)))
(assert_return (invoke "assert_not_prime" (i32.const 0x74)))
(assert_return (invoke "assert_not_prime" (i32.const 0x75)))
(assert_return (invoke "assert_not_prime" (i32.const 0x76)))
(assert_return (invoke "assert_not_prime" (i32.const 0x77)))
(assert_return (invoke "assert_not_prime" (i32.const 0x78)))
(assert_return (invoke "assert_not_prime" (i32.const 0x79)))
(assert_return (invoke "assert_not_prime" (i32.const 0x7a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x7b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x7c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x7d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x7e)))
(assert_return (invoke "assert_prime" (i32.const 0x7f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x80)))
(assert_return (invoke "assert_not_prime" (i32.const 0x81)))
(assert_return (invoke "assert_not_prime" (i32.const 0x82)))
(assert_return (invoke "assert_prime" (i32.const 0x83)))
(assert_return (invoke "assert_not_prime" (i32.const 0x84)))
(assert_return (invoke "assert_not_prime" (i32.const 0x85)))
(assert_return (invoke "assert_not_prime" (i32.const 0x86)))
(assert_return (invoke "assert_not_prime" (i32.const 0x87)))
(assert_return (invoke "assert_not_prime" (i32.const 0x88)))
(assert_return (invoke "assert_prime" (i32.const 0x89)))
(assert_return (invoke "assert_not_prime" (i32.const 0x8a)))
(assert_return (invoke "assert_prime" (i32.const 0x8b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x8c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x8d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x8e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x8f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x90)))
(assert_return (invoke "assert_not_prime" (i32.const 0x91)))
(assert_return (invoke "assert_not_prime" (i32.const 0x92)))
(assert_return (invoke "assert_not_prime" (i32.const 0x93)))
(assert_return (invoke "assert_not_prime" (i32.const 0x94)))
(assert_return (invoke "assert_prime" (i32.const 0x95)))
(assert_return (invoke "assert_not_prime" (i32.const 0x96)))
(assert_return (invoke "assert_prime" (i32.const 0x97)))
(assert_return (invoke "assert_not_prime" (i32.const 0x98)))
(assert_return (invoke "assert_not_prime" (i32.const 0x99)))
(assert_return (invoke "assert_not_prime" (i32.const 0x9a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x9b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x9c)))
(assert_return (invoke "assert_prime" (i32.const 0x9d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x9e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x9f)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa0)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa1)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa2)))
(assert_return (invoke "assert_prime" (i32.const 0xa3)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa4)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa5)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa6)))
(assert_return (invoke "assert_prime" (i32.const 0xa7)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa8)))
(assert_return (invoke "assert_not_prime" (i32.const 0xa9)))
(assert_return (invoke "assert_not_prime" (i32.const 0xaa)))
(assert_return (invoke "assert_not_prime" (i32.const 0xab)))
(assert_return (invoke "assert_not_prime" (i32.const 0xac)))
(assert_return (invoke "assert_prime" (i32.const 0xad)))
(assert_return (invoke "assert_not_prime" (i32.const 0xae)))
(assert_return (invoke "assert_not_prime" (i32.const 0xaf)))
(assert_return (invoke "assert_not_prime" (i32.const 0xb0)))
(assert_return (invoke "assert_not_prime" (i32.const 0xb1)))
(assert_return (invoke "assert_not_prime" (i32.const 0xb2)))
(assert_return (invoke "assert_prime" (i32.const 0xb3)))
(assert_return (invoke "assert_not_prime" (i32.const 0xb4)))
(assert_return (invoke "assert_prime" (i32.const 0xb5)))
(assert_return (invoke "assert_not_prime" (i32.const 0xb6)))
(assert_return (invoke "assert_not_prime" (i32.const 0xb7)))
(assert_return (invoke "assert_not_prime" (i32.const 0xb8)))
(assert_return (invoke "assert_not_prime" (i32.const 0xb9)))
(assert_return (invoke "assert_not_prime" (i32.const 0xba)))
(assert_return (invoke "assert_not_prime" (i32.const 0xbb)))
(assert_return (invoke "assert_not_prime" (i32.const 0xbc)))
(assert_return (invoke "assert_not_prime" (i32.const 0xbd)))
(assert_return (invoke "assert_not_prime" (i32.const 0xbe)))
(assert_return (invoke "assert_prime" (i32.const 0xbf)))
(assert_return (invoke "assert_not_prime" (i32.const 0xc0)))
(assert_return (invoke "assert_prime" (i32.const 0xc1)))
(assert_return (invoke "assert_not_prime" (i32.const 0xc2)))
(assert_return (invoke "assert_not_prime" (i32.const 0xc3)))
(assert_return (invoke "assert_not_prime" (i32.const 0xc4)))
(assert_return (invoke "assert_prime" (i32.const 0xc5)))
(assert_return (invoke "assert_not_prime" (i32.const 0xc6)))
(assert_return (invoke "assert_prime" (i32.const 0xc7)))
(assert_return (invoke "assert_not_prime" (i32.const 0xc8)))
(assert_return (invoke "assert_not_prime" (i32.const 0xc9)))
(assert_return (invoke "assert_not_prime" (i32.const 0xca)))
(assert_return (invoke "assert_not_prime" (i32.const 0xcb)))
(assert_return (invoke "assert_not_prime" (i32.const 0xcc)))
(assert_return (invoke "assert_not_prime" (i32.const 0xcd)))
(assert_return (invoke "assert_not_prime" (i32.const 0xce)))
(assert_return (invoke "assert_not_prime" (i32.const 0xcf)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd0)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd1)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd2)))
(assert_return (invoke "assert_prime" (i32.const 0xd3)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd4)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd5)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd6)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd7)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd8)))
(assert_return (invoke "assert_not_prime" (i32.const 0xd9)))
(assert_return (invoke "assert_not_prime" (i32.const 0xda)))
(assert_return (invoke "assert_not_prime" (i32.const 0xdb)))
(assert_return (invoke "assert_not_prime" (i32.const 0xdc)))
(assert_return (invoke "assert_not_prime" (i32.const 0xdd)))
(assert_return (invoke "assert_not_prime" (i32.const 0xde)))
(assert_return (invoke "assert_prime" (i32.const 0xdf)))
(assert_return (invoke "assert_not_prime" (i32.const 0xe0)))
(assert_return (invoke "assert_not_prime" (i32.const 0xe1)))
(assert_return (invoke "assert_not_prime" (i32.const 0xe2)))
(assert_return (invoke "assert_prime" (i32.const 0xe3)))
(assert_return (invoke "assert_not_prime" (i32.const 0xe4)))
(assert_return (invoke "assert_prime" (i32.const 0xe5)))
(assert_return (invoke "assert_not_prime" (i32.const 0xe6)))
(assert_return (invoke "assert_not_prime" (i32.const 0xe7)))
(assert_return (invoke "assert_not_prime" (i32.const 0xe8)))
(assert_return (invoke "assert_prime" (i32.const 0xe9)))
(assert_return (invoke "assert_not_prime" (i32.const 0xea)))
(assert_return (invoke "assert_not_prime" (i32.const 0xeb)))
(assert_return (invoke "assert_not_prime" (i32.const 0xec)))
(assert_return (invoke "assert_not_prime" (i32.const 0xed)))
(assert_return (invoke "assert_not_prime" (i32.const 0xee)))
(assert_return (invoke "assert_prime" (i32.const 0xef)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf0)))
(assert_return (invoke "assert_prime" (i32.const 0xf1)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf2)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf3)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf4)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf5)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf6)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf7)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf8)))
(assert_return (invoke "assert_not_prime" (i32.const 0xf9)))
(assert_return (invoke "assert_not_prime" (i32.const 0xfa)))
(assert_return (invoke "assert_prime" (i32.const 0xfb)))
(assert_return (invoke "assert_not_prime" (i32.const 0xfc)))
(assert_return (invoke "assert_not_prime" (i32.const 0xfd)))
(assert_return (invoke "assert_not_prime" (i32.const 0xfe)))
(assert_return (invoke "assert_not_prime" (i32.const 0xff)))
(assert_return (invoke "assert_not_prime" (i32.const 0x100)))
(assert_return (invoke "assert_prime" (i32.const 0x101)))
(assert_return (invoke "assert_not_prime" (i32.const 0x102)))
(assert_return (invoke "assert_not_prime" (i32.const 0x103)))
(assert_return (invoke "assert_not_prime" (i32.const 0x104)))
(assert_return (invoke "assert_not_prime" (i32.const 0x105)))
(assert_return (invoke "assert_not_prime" (i32.const 0x106)))
(assert_return (invoke "assert_prime" (i32.const 0x107)))
(assert_return (invoke "assert_not_prime" (i32.const 0x108)))
(assert_return (invoke "assert_not_prime" (i32.const 0x109)))
(assert_return (invoke "assert_not_prime" (i32.const 0x10a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x10b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x10c)))
(assert_return (invoke "assert_prime" (i32.const 0x10d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x10e)))
(assert_return (invoke "assert_prime" (i32.const 0x10f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x110)))
(assert_return (invoke "assert_not_prime" (i32.const 0x111)))
(assert_return (invoke "assert_not_prime" (i32.const 0x112)))
(assert_return (invoke "assert_not_prime" (i32.const 0x113)))
(assert_return (invoke "assert_not_prime" (i32.const 0x114)))
(assert_return (invoke "assert_prime" (i32.const 0x115)))
(assert_return (invoke "assert_not_prime" (i32.const 0x116)))
(assert_return (invoke "assert_not_prime" (i32.const 0x117)))
(assert_return (invoke "assert_not_prime" (i32.const 0x118)))
(assert_return (invoke "assert_prime" (i32.const 0x119)))
(assert_return (invoke "assert_not_prime" (i32.const 0x11a)))
(assert_return (invoke "assert_prime" (i32.const 0x11b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x11c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x11d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x11e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x11f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x120)))
(assert_return (invoke "assert_not_prime" (i32.const 0x121)))
(assert_return (invoke "assert_not_prime" (i32.const 0x122)))
(assert_return (invoke "assert_not_prime" (i32.const 0x123)))
(assert_return (invoke "assert_not_prime" (i32.const 0x124)))
(assert_return (invoke "assert_prime" (i32.const 0x125)))
(assert_return (invoke "assert_not_prime" (i32.const 0x126)))
(assert_return (invoke "assert_not_prime" (i32.const 0x127)))
(assert_return (invoke "assert_not_prime" (i32.const 0x128)))
(assert_return (invoke "assert_not_prime" (i32.const 0x129)))
(assert_return (invoke "assert_not_prime" (i32.const 0x12a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x12b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x12c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x12d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x12e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x12f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x130)))
(assert_return (invoke "assert_not_prime" (i32.const 0x131)))
(assert_return (invoke "assert_not_prime" (i32.const 0x132)))
(assert_return (invoke "assert_prime" (i32.const 0x133)))
(assert_return (invoke "assert_not_prime" (i32.const 0x134)))
(assert_return (invoke "assert_not_prime" (i32.const 0x135)))
(assert_return (invoke "assert_not_prime" (i32.const 0x136)))
(assert_return (invoke "assert_prime" (i32.const 0x137)))
(assert_return (invoke "assert_not_prime" (i32.const 0x138)))
(assert_return (invoke "assert_prime" (i32.const 0x139)))
(assert_return (invoke "assert_not_prime" (i32.const 0x13a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x13b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x13c)))
(assert_return (invoke "assert_prime" (i32.const 0x13d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x13e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x13f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x140)))
(assert_return (invoke "assert_not_prime" (i32.const 0x141)))
(assert_return (invoke "assert_not_prime" (i32.const 0x142)))
(assert_return (invoke "assert_not_prime" (i32.const 0x143)))
(assert_return (invoke "assert_not_prime" (i32.const 0x144)))
(assert_return (invoke "assert_not_prime" (i32.const 0x145)))
(assert_return (invoke "assert_not_prime" (i32.const 0x146)))
(assert_return (invoke "assert_not_prime" (i32.const 0x147)))
(assert_return (invoke "assert_not_prime" (i32.const 0x148)))
(assert_return (invoke "assert_not_prime" (i32.const 0x149)))
(assert_return (invoke "assert_not_prime" (i32.const 0x14a)))
(assert_return (invoke "assert_prime" (i32.const 0x14b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x14c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x14d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x14e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x14f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x150)))
(assert_return (invoke "assert_prime" (i32.const 0x151)))
(assert_return (invoke "assert_not_prime" (i32.const 0x152)))
(assert_return (invoke "assert_not_prime" (i32.const 0x153)))
(assert_return (invoke "assert_not_prime" (i32.const 0x154)))
(assert_return (invoke "assert_not_prime" (i32.const 0x155)))
(assert_return (invoke "assert_not_prime" (i32.const 0x156)))
(assert_return (invoke "assert_not_prime" (i32.const 0x157)))
(assert_return (invoke "assert_not_prime" (i32.const 0x158)))
(assert_return (invoke "assert_not_prime" (i32.const 0x159)))
(assert_return (invoke "assert_not_prime" (i32.const 0x15a)))
(assert_return (invoke "assert_prime" (i32.const 0x15b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x15c)))
(assert_return (invoke "assert_prime" (i32.const 0x15d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x15e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x15f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x160)))
(assert_return (invoke "assert_prime" (i32.const 0x161)))
(assert_return (invoke "assert_not_prime" (i32.const 0x162)))
(assert_return (invoke "assert_not_prime" (i32.const 0x163)))
(assert_return (invoke "assert_not_prime" (i32.const 0x164)))
(assert_return (invoke "assert_not_prime" (i32.const 0x165)))
(assert_return (invoke "assert_not_prime" (i32.const 0x166)))
(assert_return (invoke "assert_prime" (i32.const 0x167)))
(assert_return (invoke "assert_not_prime" (i32.const 0x168)))
(assert_return (invoke "assert_not_prime" (i32.const 0x169)))
(assert_return (invoke "assert_not_prime" (i32.const 0x16a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x16b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x16c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x16d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x16e)))
(assert_return (invoke "assert_prime" (i32.const 0x16f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x170)))
(assert_return (invoke "assert_not_prime" (i32.const 0x171)))
(assert_return (invoke "assert_not_prime" (i32.const 0x172)))
(assert_return (invoke "assert_not_prime" (i32.const 0x173)))
(assert_return (invoke "assert_not_prime" (i32.const 0x174)))
(assert_return (invoke "assert_prime" (i32.const 0x175)))
(assert_return (invoke "assert_not_prime" (i32.const 0x176)))
(assert_return (invoke "assert_not_prime" (i32.const 0x177)))
(assert_return (invoke "assert_not_prime" (i32.const 0x178)))
(assert_return (invoke "assert_not_prime" (i32.const 0x179)))
(assert_return (invoke "assert_not_prime" (i32.const 0x17a)))
(assert_return (invoke "assert_prime" (i32.const 0x17b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x17c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x17d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x17e)))
(assert_return (invoke "assert_prime" (i32.const 0x17f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x180)))
(assert_return (invoke "assert_not_prime" (i32.const 0x181)))
(assert_return (invoke "assert_not_prime" (i32.const 0x182)))
(assert_return (invoke "assert_not_prime" (i32.const 0x183)))
(assert_return (invoke "assert_not_prime" (i32.const 0x184)))
(assert_return (invoke "assert_prime" (i32.const 0x185)))
(assert_return (invoke "assert_not_prime" (i32.const 0x186)))
(assert_return (invoke "assert_not_prime" (i32.const 0x187)))
(assert_return (invoke "assert_not_prime" (i32.const 0x188)))
(assert_return (invoke "assert_not_prime" (i32.const 0x189)))
(assert_return (invoke "assert_not_prime" (i32.const 0x18a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x18b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x18c)))
(assert_return (invoke "assert_prime" (i32.const 0x18d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x18e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x18f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x190)))
(assert_return (invoke "assert_prime" (i32.const 0x191)))
(assert_return (invoke "assert_not_prime" (i32.const 0x192)))
(assert_return (invoke "assert_not_prime" (i32.const 0x193)))
(assert_return (invoke "assert_not_prime" (i32.const 0x194)))
(assert_return (invoke "assert_not_prime" (i32.const 0x195)))
(assert_return (invoke "assert_not_prime" (i32.const 0x196)))
(assert_return (invoke "assert_not_prime" (i32.const 0x197)))
(assert_return (invoke "assert_not_prime" (i32.const 0x198)))
(assert_return (invoke "assert_prime" (i32.const 0x199)))
(assert_return (invoke "assert_not_prime" (i32.const 0x19a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x19b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x19c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x19d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x19e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x19f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a2)))
(assert_return (invoke "assert_prime" (i32.const 0x1a3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a4)))
(assert_return (invoke "assert_prime" (i32.const 0x1a5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1a9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1aa)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ab)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ac)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ad)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ae)))
(assert_return (invoke "assert_prime" (i32.const 0x1af)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b0)))
(assert_return (invoke "assert_prime" (i32.const 0x1b1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b6)))
(assert_return (invoke "assert_prime" (i32.const 0x1b7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1b9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ba)))
(assert_return (invoke "assert_prime" (i32.const 0x1bb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1bc)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1bd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1be)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1bf)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c0)))
(assert_return (invoke "assert_prime" (i32.const 0x1c1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1c8)))
(assert_return (invoke "assert_prime" (i32.const 0x1c9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ca)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1cb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1cc)))
(assert_return (invoke "assert_prime" (i32.const 0x1cd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ce)))
(assert_return (invoke "assert_prime" (i32.const 0x1cf)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d2)))
(assert_return (invoke "assert_prime" (i32.const 0x1d3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1d9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1da)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1db)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1dc)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1dd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1de)))
(assert_return (invoke "assert_prime" (i32.const 0x1df)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e6)))
(assert_return (invoke "assert_prime" (i32.const 0x1e7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1e9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ea)))
(assert_return (invoke "assert_prime" (i32.const 0x1eb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ec)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ed)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ee)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ef)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1f0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1f1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1f2)))
(assert_return (invoke "assert_prime" (i32.const 0x1f3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1f4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1f5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1f6)))
(assert_return (invoke "assert_prime" (i32.const 0x1f7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1f8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1f9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1fa)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1fb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1fc)))
(assert_return (invoke "assert_prime" (i32.const 0x1fd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1fe)))
(assert_return (invoke "assert_not_prime" (i32.const 0x1ff)))
(assert_return (invoke "assert_not_prime" (i32.const 0x200)))
(assert_return (invoke "assert_not_prime" (i32.const 0x201)))
(assert_return (invoke "assert_not_prime" (i32.const 0x202)))
(assert_return (invoke "assert_not_prime" (i32.const 0x203)))
(assert_return (invoke "assert_not_prime" (i32.const 0x204)))
(assert_return (invoke "assert_not_prime" (i32.const 0x205)))
(assert_return (invoke "assert_not_prime" (i32.const 0x206)))
(assert_return (invoke "assert_not_prime" (i32.const 0x207)))
(assert_return (invoke "assert_not_prime" (i32.const 0x208)))
(assert_return (invoke "assert_prime" (i32.const 0x209)))
(assert_return (invoke "assert_not_prime" (i32.const 0x20a)))
(assert_return (invoke "assert_prime" (i32.const 0x20b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x20c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x20d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x20e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x20f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x210)))
(assert_return (invoke "assert_not_prime" (i32.const 0x211)))
(assert_return (invoke "assert_not_prime" (i32.const 0x212)))
(assert_return (invoke "assert_not_prime" (i32.const 0x213)))
(assert_return (invoke "assert_not_prime" (i32.const 0x214)))
(assert_return (invoke "assert_not_prime" (i32.const 0x215)))
(assert_return (invoke "assert_not_prime" (i32.const 0x216)))
(assert_return (invoke "assert_not_prime" (i32.const 0x217)))
(assert_return (invoke "assert_not_prime" (i32.const 0x218)))
(assert_return (invoke "assert_not_prime" (i32.const 0x219)))
(assert_return (invoke "assert_not_prime" (i32.const 0x21a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x21b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x21c)))
(assert_return (invoke "assert_prime" (i32.const 0x21d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x21e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x21f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x220)))
(assert_return (invoke "assert_not_prime" (i32.const 0x221)))
(assert_return (invoke "assert_not_prime" (i32.const 0x222)))
(assert_return (invoke "assert_prime" (i32.const 0x223)))
(assert_return (invoke "assert_not_prime" (i32.const 0x224)))
(assert_return (invoke "assert_not_prime" (i32.const 0x225)))
(assert_return (invoke "assert_not_prime" (i32.const 0x226)))
(assert_return (invoke "assert_not_prime" (i32.const 0x227)))
(assert_return (invoke "assert_not_prime" (i32.const 0x228)))
(assert_return (invoke "assert_not_prime" (i32.const 0x229)))
(assert_return (invoke "assert_not_prime" (i32.const 0x22a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x22b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x22c)))
(assert_return (invoke "assert_prime" (i32.const 0x22d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x22e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x22f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x230)))
(assert_return (invoke "assert_not_prime" (i32.const 0x231)))
(assert_return (invoke "assert_not_prime" (i32.const 0x232)))
(assert_return (invoke "assert_prime" (i32.const 0x233)))
(assert_return (invoke "assert_not_prime" (i32.const 0x234)))
(assert_return (invoke "assert_not_prime" (i32.const 0x235)))
(assert_return (invoke "assert_not_prime" (i32.const 0x236)))
(assert_return (invoke "assert_not_prime" (i32.const 0x237)))
(assert_return (invoke "assert_not_prime" (i32.const 0x238)))
(assert_return (invoke "assert_prime" (i32.const 0x239)))
(assert_return (invoke "assert_not_prime" (i32.const 0x23a)))
(assert_return (invoke "assert_prime" (i32.const 0x23b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x23c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x23d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x23e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x23f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x240)))
(assert_return (invoke "assert_prime" (i32.const 0x241)))
(assert_return (invoke "assert_not_prime" (i32.const 0x242)))
(assert_return (invoke "assert_not_prime" (i32.const 0x243)))
(assert_return (invoke "assert_not_prime" (i32.const 0x244)))
(assert_return (invoke "assert_not_prime" (i32.const 0x245)))
(assert_return (invoke "assert_not_prime" (i32.const 0x246)))
(assert_return (invoke "assert_not_prime" (i32.const 0x247)))
(assert_return (invoke "assert_not_prime" (i32.const 0x248)))
(assert_return (invoke "assert_not_prime" (i32.const 0x249)))
(assert_return (invoke "assert_not_prime" (i32.const 0x24a)))
(assert_return (invoke "assert_prime" (i32.const 0x24b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x24c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x24d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x24e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x24f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x250)))
(assert_return (invoke "assert_prime" (i32.const 0x251)))
(assert_return (invoke "assert_not_prime" (i32.const 0x252)))
(assert_return (invoke "assert_not_prime" (i32.const 0x253)))
(assert_return (invoke "assert_not_prime" (i32.const 0x254)))
(assert_return (invoke "assert_not_prime" (i32.const 0x255)))
(assert_return (invoke "assert_not_prime" (i32.const 0x256)))
(assert_return (invoke "assert_prime" (i32.const 0x257)))
(assert_return (invoke "assert_not_prime" (i32.const 0x258)))
(assert_return (invoke "assert_prime" (i32.const 0x259)))
(assert_return (invoke "assert_not_prime" (i32.const 0x25a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x25b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x25c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x25d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x25e)))
(assert_return (invoke "assert_prime" (i32.const 0x25f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x260)))
(assert_return (invoke "assert_not_prime" (i32.const 0x261)))
(assert_return (invoke "assert_not_prime" (i32.const 0x262)))
(assert_return (invoke "assert_not_prime" (i32.const 0x263)))
(assert_return (invoke "assert_not_prime" (i32.const 0x264)))
(assert_return (invoke "assert_prime" (i32.const 0x265)))
(assert_return (invoke "assert_not_prime" (i32.const 0x266)))
(assert_return (invoke "assert_not_prime" (i32.const 0x267)))
(assert_return (invoke "assert_not_prime" (i32.const 0x268)))
(assert_return (invoke "assert_prime" (i32.const 0x269)))
(assert_return (invoke "assert_not_prime" (i32.const 0x26a)))
(assert_return (invoke "assert_prime" (i32.const 0x26b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x26c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x26d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x26e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x26f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x270)))
(assert_return (invoke "assert_not_prime" (i32.const 0x271)))
(assert_return (invoke "assert_not_prime" (i32.const 0x272)))
(assert_return (invoke "assert_not_prime" (i32.const 0x273)))
(assert_return (invoke "assert_not_prime" (i32.const 0x274)))
(assert_return (invoke "assert_not_prime" (i32.const 0x275)))
(assert_return (invoke "assert_not_prime" (i32.const 0x276)))
(assert_return (invoke "assert_prime" (i32.const 0x277)))
(assert_return (invoke "assert_not_prime" (i32.const 0x278)))
(assert_return (invoke "assert_not_prime" (i32.const 0x279)))
(assert_return (invoke "assert_not_prime" (i32.const 0x27a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x27b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x27c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x27d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x27e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x27f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x280)))
(assert_return (invoke "assert_prime" (i32.const 0x281)))
(assert_return (invoke "assert_not_prime" (i32.const 0x282)))
(assert_return (invoke "assert_prime" (i32.const 0x283)))
(assert_return (invoke "assert_not_prime" (i32.const 0x284)))
(assert_return (invoke "assert_not_prime" (i32.const 0x285)))
(assert_return (invoke "assert_not_prime" (i32.const 0x286)))
(assert_return (invoke "assert_prime" (i32.const 0x287)))
(assert_return (invoke "assert_not_prime" (i32.const 0x288)))
(assert_return (invoke "assert_not_prime" (i32.const 0x289)))
(assert_return (invoke "assert_not_prime" (i32.const 0x28a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x28b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x28c)))
(assert_return (invoke "assert_prime" (i32.const 0x28d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x28e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x28f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x290)))
(assert_return (invoke "assert_not_prime" (i32.const 0x291)))
(assert_return (invoke "assert_not_prime" (i32.const 0x292)))
(assert_return (invoke "assert_prime" (i32.const 0x293)))
(assert_return (invoke "assert_not_prime" (i32.const 0x294)))
(assert_return (invoke "assert_prime" (i32.const 0x295)))
(assert_return (invoke "assert_not_prime" (i32.const 0x296)))
(assert_return (invoke "assert_not_prime" (i32.const 0x297)))
(assert_return (invoke "assert_not_prime" (i32.const 0x298)))
(assert_return (invoke "assert_not_prime" (i32.const 0x299)))
(assert_return (invoke "assert_not_prime" (i32.const 0x29a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x29b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x29c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x29d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x29e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x29f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a0)))
(assert_return (invoke "assert_prime" (i32.const 0x2a1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a4)))
(assert_return (invoke "assert_prime" (i32.const 0x2a5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2a9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2aa)))
(assert_return (invoke "assert_prime" (i32.const 0x2ab)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ac)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ad)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ae)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2af)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b2)))
(assert_return (invoke "assert_prime" (i32.const 0x2b3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2b9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ba)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2bb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2bc)))
(assert_return (invoke "assert_prime" (i32.const 0x2bd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2be)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2bf)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c4)))
(assert_return (invoke "assert_prime" (i32.const 0x2c5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2c9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ca)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2cb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2cc)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2cd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ce)))
(assert_return (invoke "assert_prime" (i32.const 0x2cf)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d6)))
(assert_return (invoke "assert_prime" (i32.const 0x2d7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2d9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2da)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2db)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2dc)))
(assert_return (invoke "assert_prime" (i32.const 0x2dd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2de)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2df)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e2)))
(assert_return (invoke "assert_prime" (i32.const 0x2e3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e6)))
(assert_return (invoke "assert_prime" (i32.const 0x2e7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2e9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ea)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2eb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ec)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ed)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ee)))
(assert_return (invoke "assert_prime" (i32.const 0x2ef)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2f0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2f1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2f2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2f3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2f4)))
(assert_return (invoke "assert_prime" (i32.const 0x2f5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2f6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2f7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2f8)))
(assert_return (invoke "assert_prime" (i32.const 0x2f9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2fa)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2fb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2fc)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2fd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2fe)))
(assert_return (invoke "assert_not_prime" (i32.const 0x2ff)))
(assert_return (invoke "assert_not_prime" (i32.const 0x300)))
(assert_return (invoke "assert_prime" (i32.const 0x301)))
(assert_return (invoke "assert_not_prime" (i32.const 0x302)))
(assert_return (invoke "assert_not_prime" (i32.const 0x303)))
(assert_return (invoke "assert_not_prime" (i32.const 0x304)))
(assert_return (invoke "assert_prime" (i32.const 0x305)))
(assert_return (invoke "assert_not_prime" (i32.const 0x306)))
(assert_return (invoke "assert_not_prime" (i32.const 0x307)))
(assert_return (invoke "assert_not_prime" (i32.const 0x308)))
(assert_return (invoke "assert_not_prime" (i32.const 0x309)))
(assert_return (invoke "assert_not_prime" (i32.const 0x30a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x30b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x30c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x30d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x30e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x30f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x310)))
(assert_return (invoke "assert_not_prime" (i32.const 0x311)))
(assert_return (invoke "assert_not_prime" (i32.const 0x312)))
(assert_return (invoke "assert_prime" (i32.const 0x313)))
(assert_return (invoke "assert_not_prime" (i32.const 0x314)))
(assert_return (invoke "assert_not_prime" (i32.const 0x315)))
(assert_return (invoke "assert_not_prime" (i32.const 0x316)))
(assert_return (invoke "assert_not_prime" (i32.const 0x317)))
(assert_return (invoke "assert_not_prime" (i32.const 0x318)))
(assert_return (invoke "assert_not_prime" (i32.const 0x319)))
(assert_return (invoke "assert_not_prime" (i32.const 0x31a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x31b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x31c)))
(assert_return (invoke "assert_prime" (i32.const 0x31d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x31e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x31f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x320)))
(assert_return (invoke "assert_not_prime" (i32.const 0x321)))
(assert_return (invoke "assert_not_prime" (i32.const 0x322)))
(assert_return (invoke "assert_not_prime" (i32.const 0x323)))
(assert_return (invoke "assert_not_prime" (i32.const 0x324)))
(assert_return (invoke "assert_not_prime" (i32.const 0x325)))
(assert_return (invoke "assert_not_prime" (i32.const 0x326)))
(assert_return (invoke "assert_not_prime" (i32.const 0x327)))
(assert_return (invoke "assert_not_prime" (i32.const 0x328)))
(assert_return (invoke "assert_prime" (i32.const 0x329)))
(assert_return (invoke "assert_not_prime" (i32.const 0x32a)))
(assert_return (invoke "assert_prime" (i32.const 0x32b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x32c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x32d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x32e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x32f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x330)))
(assert_return (invoke "assert_not_prime" (i32.const 0x331)))
(assert_return (invoke "assert_not_prime" (i32.const 0x332)))
(assert_return (invoke "assert_not_prime" (i32.const 0x333)))
(assert_return (invoke "assert_not_prime" (i32.const 0x334)))
(assert_return (invoke "assert_prime" (i32.const 0x335)))
(assert_return (invoke "assert_not_prime" (i32.const 0x336)))
(assert_return (invoke "assert_prime" (i32.const 0x337)))
(assert_return (invoke "assert_not_prime" (i32.const 0x338)))
(assert_return (invoke "assert_not_prime" (i32.const 0x339)))
(assert_return (invoke "assert_not_prime" (i32.const 0x33a)))
(assert_return (invoke "assert_prime" (i32.const 0x33b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x33c)))
(assert_return (invoke "assert_prime" (i32.const 0x33d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x33e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x33f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x340)))
(assert_return (invoke "assert_not_prime" (i32.const 0x341)))
(assert_return (invoke "assert_not_prime" (i32.const 0x342)))
(assert_return (invoke "assert_not_prime" (i32.const 0x343)))
(assert_return (invoke "assert_not_prime" (i32.const 0x344)))
(assert_return (invoke "assert_not_prime" (i32.const 0x345)))
(assert_return (invoke "assert_not_prime" (i32.const 0x346)))
(assert_return (invoke "assert_prime" (i32.const 0x347)))
(assert_return (invoke "assert_not_prime" (i32.const 0x348)))
(assert_return (invoke "assert_not_prime" (i32.const 0x349)))
(assert_return (invoke "assert_not_prime" (i32.const 0x34a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x34b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x34c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x34d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x34e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x34f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x350)))
(assert_return (invoke "assert_not_prime" (i32.const 0x351)))
(assert_return (invoke "assert_not_prime" (i32.const 0x352)))
(assert_return (invoke "assert_not_prime" (i32.const 0x353)))
(assert_return (invoke "assert_not_prime" (i32.const 0x354)))
(assert_return (invoke "assert_prime" (i32.const 0x355)))
(assert_return (invoke "assert_not_prime" (i32.const 0x356)))
(assert_return (invoke "assert_not_prime" (i32.const 0x357)))
(assert_return (invoke "assert_not_prime" (i32.const 0x358)))
(assert_return (invoke "assert_prime" (i32.const 0x359)))
(assert_return (invoke "assert_not_prime" (i32.const 0x35a)))
(assert_return (invoke "assert_prime" (i32.const 0x35b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x35c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x35d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x35e)))
(assert_return (invoke "assert_prime" (i32.const 0x35f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x360)))
(assert_return (invoke "assert_not_prime" (i32.const 0x361)))
(assert_return (invoke "assert_not_prime" (i32.const 0x362)))
(assert_return (invoke "assert_not_prime" (i32.const 0x363)))
(assert_return (invoke "assert_not_prime" (i32.const 0x364)))
(assert_return (invoke "assert_not_prime" (i32.const 0x365)))
(assert_return (invoke "assert_not_prime" (i32.const 0x366)))
(assert_return (invoke "assert_not_prime" (i32.const 0x367)))
(assert_return (invoke "assert_not_prime" (i32.const 0x368)))
(assert_return (invoke "assert_not_prime" (i32.const 0x369)))
(assert_return (invoke "assert_not_prime" (i32.const 0x36a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x36b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x36c)))
(assert_return (invoke "assert_prime" (i32.const 0x36d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x36e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x36f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x370)))
(assert_return (invoke "assert_prime" (i32.const 0x371)))
(assert_return (invoke "assert_not_prime" (i32.const 0x372)))
(assert_return (invoke "assert_prime" (i32.const 0x373)))
(assert_return (invoke "assert_not_prime" (i32.const 0x374)))
(assert_return (invoke "assert_not_prime" (i32.const 0x375)))
(assert_return (invoke "assert_not_prime" (i32.const 0x376)))
(assert_return (invoke "assert_prime" (i32.const 0x377)))
(assert_return (invoke "assert_not_prime" (i32.const 0x378)))
(assert_return (invoke "assert_not_prime" (i32.const 0x379)))
(assert_return (invoke "assert_not_prime" (i32.const 0x37a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x37b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x37c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x37d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x37e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x37f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x380)))
(assert_return (invoke "assert_not_prime" (i32.const 0x381)))
(assert_return (invoke "assert_not_prime" (i32.const 0x382)))
(assert_return (invoke "assert_not_prime" (i32.const 0x383)))
(assert_return (invoke "assert_not_prime" (i32.const 0x384)))
(assert_return (invoke "assert_not_prime" (i32.const 0x385)))
(assert_return (invoke "assert_not_prime" (i32.const 0x386)))
(assert_return (invoke "assert_not_prime" (i32.const 0x387)))
(assert_return (invoke "assert_not_prime" (i32.const 0x388)))
(assert_return (invoke "assert_not_prime" (i32.const 0x389)))
(assert_return (invoke "assert_not_prime" (i32.const 0x38a)))
(assert_return (invoke "assert_prime" (i32.const 0x38b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x38c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x38d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x38e)))
(assert_return (invoke "assert_prime" (i32.const 0x38f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x390)))
(assert_return (invoke "assert_not_prime" (i32.const 0x391)))
(assert_return (invoke "assert_not_prime" (i32.const 0x392)))
(assert_return (invoke "assert_not_prime" (i32.const 0x393)))
(assert_return (invoke "assert_not_prime" (i32.const 0x394)))
(assert_return (invoke "assert_not_prime" (i32.const 0x395)))
(assert_return (invoke "assert_not_prime" (i32.const 0x396)))
(assert_return (invoke "assert_prime" (i32.const 0x397)))
(assert_return (invoke "assert_not_prime" (i32.const 0x398)))
(assert_return (invoke "assert_not_prime" (i32.const 0x399)))
(assert_return (invoke "assert_not_prime" (i32.const 0x39a)))
(assert_return (invoke "assert_not_prime" (i32.const 0x39b)))
(assert_return (invoke "assert_not_prime" (i32.const 0x39c)))
(assert_return (invoke "assert_not_prime" (i32.const 0x39d)))
(assert_return (invoke "assert_not_prime" (i32.const 0x39e)))
(assert_return (invoke "assert_not_prime" (i32.const 0x39f)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a0)))
(assert_return (invoke "assert_prime" (i32.const 0x3a1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3a8)))
(assert_return (invoke "assert_prime" (i32.const 0x3a9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3aa)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3ab)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3ac)))
(assert_return (invoke "assert_prime" (i32.const 0x3ad)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3ae)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3af)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3b0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3b1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3b2)))
(assert_return (invoke "assert_prime" (i32.const 0x3b3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3b4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3b5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3b6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3b7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3b8)))
(assert_return (invoke "assert_prime" (i32.const 0x3b9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3ba)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3bb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3bc)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3bd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3be)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3bf)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c6)))
(assert_return (invoke "assert_prime" (i32.const 0x3c7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3c9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3ca)))
(assert_return (invoke "assert_prime" (i32.const 0x3cb)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3cc)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3cd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3ce)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3cf)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3d0)))
(assert_return (invoke "assert_prime" (i32.const 0x3d1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3d2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3d3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3d4)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3d5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3d6)))
(assert_return (invoke "assert_prime" (i32.const 0x3d7)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3d8)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3d9)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3da)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3db)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3dc)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3dd)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3de)))
(assert_return (invoke "assert_prime" (i32.const 0x3df)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3e0)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3e1)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3e2)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3e3)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3e4)))
(assert_return (invoke "assert_prime" (i32.const 0x3e5)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3e6)))
(assert_return (invoke "assert_not_prime" (i32.const 0x3e7)))
