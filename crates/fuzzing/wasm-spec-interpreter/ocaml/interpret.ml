(* This module exposes an [interpret] function to Rust. It wraps several different calls from the
WebAssembly specification interpreter in a way that we can access across the FFI boundary. To
understand this better, see:
 - the OCaml manual documentation re: calling OCaml from C, https://ocaml.org/manual/intfc.html#s%3Ac-advexample
 - the [ocaml-interop] example, https://github.com/tezedge/ocaml-interop/blob/master/testing/rust-caller/ocaml/callable.ml
*)

(* Here we access the WebAssembly specification interpreter; this must be linked in. *)
open Wasm

(** Enumerate the types of values we pass across the FFI boundary. This must match `Value` in
`src/lib.rs` *)
type ffi_value =
  | I32 of int32
  | I64 of int64
  | F32 of int32
  | F64 of int64

(** Helper for converting the FFI values to their spec interpreter type. *)
let convert_to_wasm (v: ffi_value) : Values.value = match v with
| I32 n -> Values.Num (I32 n)
| I64 n -> Values.Num (I64 n)
| F32 n -> Values.Num (F32 (F32.of_bits n))
| F64 n -> Values.Num (F64 (F64.of_bits n))

(** Helper for converting the spec interpreter values to their FFI type. *)
let convert_from_wasm (v: Values.value) : ffi_value = match v with
| Values.Num (I32 n) -> I32 n
| Values.Num (I64 n) -> I64 n
| Values.Num (F32 n) -> F32 (F32.to_bits n)
| Values.Num (F64 n) -> F64 (F64.to_bits n)
| _ -> failwith "Unknown type"

(** Parse the given WebAssembly module binary into an Ast.module_. At some point in the future this
should also be able to parse the textual form (TODO). *)
let parse bytes =
  (* Optionally, use Bytes.unsafe_to_string here to avoid the copy *)
  let bytes_as_str = Bytes.to_string bytes in
  Decode.decode "default" bytes_as_str

(** Return true if an export is a function. *)
let match_exported_func export = match export with
| (_, Instance.ExternFunc(func)) -> true
| _ -> false

(** Extract a function from its export or fail. *)
let extract_exported_func export = match export with
| (_, Instance.ExternFunc(func)) -> func
| _ -> failwith ""

(** Interpret the first exported function with the given parameters and return the result. *)
let interpret_exn module_bytes params =
  let params' = List.map convert_to_wasm params in
  let module_ = parse module_bytes in
  let instance = Eval.init module_ [] in
  let func = extract_exported_func (List.find match_exported_func instance.exports) in
  let returns = Eval.invoke func params' in
  let returns' = List.map convert_from_wasm returns in
  returns' (* TODO eventually we should hash the memory state and return the hash *)

let interpret module_bytes params =
  try Ok(interpret_exn module_bytes params) with
  | _ as e -> Error(Printexc.to_string e)

let () =
  Callback.register "interpret" interpret;
