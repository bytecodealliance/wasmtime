# The component model CLI world exports its entrypoint under the name
# "main". However, LLVM has special handling for functions named `main`
# in order to handle the `main(void)` vs `main(int argc, char **argv)`
# difference on Wasm where the caller needs to know the exact signature.
# To avoid this, define a function with a different name and export it
# as `main`.
#
# To generate the `main.o` file from this `main.s` file, compile with
# `clang --target=wasm32-wasi -c main.s`

	.text
	.functype	main () -> (i32)
	.export_name	main, main
	.functype	_start () -> ()
	.import_name	_start, _start
	.import_module	_start, __main_module__
	.section	.text.main,"",@
	.hidden	main
	.globl	main
	.type	main,@function
main:
	.functype	main () -> (i32)
	call	_start
	i32.const	0
	return
	end_function
	.no_dead_strip	main
