#pragma once
#include <string_view>

#define CHECK_ERR(err)                                                         \
  do {                                                                         \
    if (err) {                                                                 \
      auto msg = wasm_name_t{};                                                \
      wasmtime_error_message(err, &msg);                                       \
      EXPECT_EQ(err, nullptr) << std::string_view{msg.data, msg.size};         \
    }                                                                          \
  } while (false)

// From crates/component-util/src/lib.rs
inline constexpr std::string_view REALLOC_AND_FREE =
    R"END(
(global $last (mut i32) (i32.const 8))
(func $realloc (export "realloc")
	(param $old_ptr i32)
	(param $old_size i32)
	(param $align i32)
	(param $new_size i32)
	(result i32)

	(local $ret i32)

	;; Test if the old pointer is non-null
	local.get $old_ptr
	if
		;; If the old size is bigger than the new size then
		;; this is a shrink and transparently allow it
		local.get $old_size
		local.get $new_size
		i32.gt_u
		if
			local.get $old_ptr
			return
		end

		;; otherwise fall through to allocate a new chunk which will later
		;; copy data over
	end

	;; align up `$last`
	(global.set $last
		(i32.and
			(i32.add
				(global.get $last)
				(i32.add
					(local.get $align)
					(i32.const -1)))
			(i32.xor
				(i32.add
					(local.get $align)
					(i32.const -1))
				(i32.const -1))))

	;; save the current value of `$last` as the return value
	global.get $last
	local.set $ret

	;; bump our pointer
	(global.set $last
		(i32.add
			(global.get $last)
			(local.get $new_size)))

	;; while `memory.size` is less than `$last`, grow memory
	;; by one page
	(loop $loop
		(if
			(i32.lt_u
				(i32.mul (memory.size) (i32.const 65536))
				(global.get $last))
			(then
				i32.const 1
				memory.grow
				;; test to make sure growth succeeded
				i32.const -1
				i32.eq
				if unreachable end

				br $loop)))


	;; ensure anything necessary is set to valid data by spraying a bit
	;; pattern that is invalid
	local.get $ret
	i32.const 0xde
	local.get $new_size
	memory.fill

	;; If the old pointer is present then that means this was a reallocation
	;; of an existing chunk which means the existing data must be copied.
	local.get $old_ptr
	if
		local.get $ret          ;; destination
		local.get $old_ptr      ;; source
		local.get $old_size     ;; size
		memory.copy
	end

	local.get $ret
)
)END";
