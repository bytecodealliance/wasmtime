use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut function_names = Vec::new();
    function_names.push("replace_realloc_global_ptr".to_owned());
    function_names.push("replace_realloc_global_len".to_owned());
    function_names.push("replace_fds".to_owned());

    let mut global_names = Vec::new();
    global_names.push("internal_realloc_global_ptr".to_owned());
    global_names.push("internal_realloc_global_len".to_owned());
    global_names.push("internal_fds".to_owned());

    let wasm = build_raw_intrinsics(&function_names, &global_names);
    let archive = build_archive(&wasm, &function_names);

    std::fs::write(out_dir.join("libwasm-raw-intrinsics.a"), &archive).unwrap();
    println!("cargo:rustc-link-lib=static=wasm-raw-intrinsics");
    println!(
        "cargo:rustc-link-search=native={}",
        out_dir.to_str().unwrap()
    );
}

/// This function will produce a wasm module which is itself an object file
/// that is the basic equivalent of:
///
/// ```rust
/// std::arch::global_asm!(
///     "
///         .globaltype internal_realloc_global_ptr, i32
///         internal_realloc_global_ptr:
///         .globaltype internal_realloc_global_len, i32
///         internal_realloc_global_len:
///         .globaltype internal_fds, i32
///         internal_fds:
///     "
/// );
///
/// #[no_mangle]
/// extern "C" fn replace_realloc_global_ptr(val: *mut u8) -> *mut u8 {
///     unsafe {
///         let ret: *mut u8;
///         std::arch::asm!(
///             "
///                 global.get internal_realloc_global_ptr
///                 local.get {}
///                 global.set internal_realloc_global_ptr
///             ",
///             out(local) ret,
///             in(local) val,
///             options(nostack, readonly)
///         );
///         ret
///     }
/// }
///
/// #[no_mangle]
/// extern "C" fn replace_realloc_global_len(val: usize) -> usize {
///     unsafe {
///         let ret: usize;
///         std::arch::asm!(
///             "
///                 global.get internal_realloc_global_len
///                 local.get {}
///                 global.set internal_realloc_global_len
///             ",
///             out(local) ret,
///             in(local) val,
///             options(nostack, readonly)
///         );
///         ret
///     }
/// }
/// ```
///
/// The main trickiness here is getting the `reloc.CODE` and `linking` sections
/// right.
fn build_raw_intrinsics(function_names: &[String], global_names: &[String]) -> Vec<u8> {
    use wasm_encoder::Instruction::*;
    use wasm_encoder::*;

    let mut module = Module::new();

    // All our functions have the same type, i32 -> i32
    let mut types = TypeSection::new();
    types.function([ValType::I32], [ValType::I32]);
    module.section(&types);

    // Declare the functions, using the type we just added.
    let mut funcs = FunctionSection::new();
    for _ in function_names {
        funcs.function(0);
    }
    module.section(&funcs);

    // Declare the globals.
    let mut globals = GlobalSection::new();
    for _ in global_names {
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
            },
            &ConstExpr::i32_const(0),
        );
    }
    module.section(&globals);

    // Here the `code` section is defined. This is tricky because an offset is
    // needed within the code section itself for the `reloc.CODE` section
    // later. At this time `wasm-encoder` doesn't give enough functionality to
    // use the high-level APIs. so everything is done manually here.
    //
    // First the function body is created and then it's appended into a code
    // section.
    let mut body = Vec::new();
    0u32.encode(&mut body); // no locals
    let global_offset0 = body.len() + 1;
    // global.get 0 ;; but with maximal encoding of the 0
    body.extend_from_slice(&[0x23, 0x80, 0x80, 0x80, 0x80, 0x00]);
    LocalGet(0).encode(&mut body);
    let global_offset1 = body.len() + 1;
    // global.set 0 ;; but with maximal encoding of the 0
    body.extend_from_slice(&[0x24, 0x81, 0x80, 0x80, 0x80, 0x00]);
    End.encode(&mut body);

    let mut body_offsets = Vec::new();

    let mut code = Vec::new();
    function_names.len().encode(&mut code);
    for _ in function_names {
        body.len().encode(&mut code); // length of the function
        body_offsets.push(code.len());
        code.extend_from_slice(&body); // the function itself
    }
    module.section(&RawSection {
        id: SectionId::Code as u8,
        data: &code,
    });

    // Calculate the relocation offsets within the `code` section itself by
    // adding the start of where the body was placed to the offset within the
    // body.
    let global_offsets0 = body_offsets
        .iter()
        .map(|x| x + global_offset0)
        .collect::<Vec<_>>();
    let global_offsets1 = body_offsets
        .iter()
        .map(|x| x + global_offset1)
        .collect::<Vec<_>>();

    // Here the linking section is constructed. There are two symbols described
    // here, one for the function that we injected and one for the global
    // that was injected. The injected global here is referenced in the
    // relocations below.
    //
    // More information about this format is at
    // https://github.com/WebAssembly/tool-conventions/blob/main/Linking.md
    {
        let mut linking = Vec::new();
        linking.push(0x02); // version

        linking.push(0x08); // `WASM_SYMBOL_TABLE`
        let mut subsection = Vec::new();
        6u32.encode(&mut subsection); // 6 symbols (3 functions + 3 globals)

        for (index, name) in function_names.iter().enumerate() {
            subsection.push(0x00); // SYMTAB_FUNCTION
            0x00.encode(&mut subsection); // flags
            index.encode(&mut subsection); // function index
            name.encode(&mut subsection); // symbol name
        }

        for (index, name) in global_names.iter().enumerate() {
            subsection.push(0x02); // SYMTAB_GLOBAL
            0x02.encode(&mut subsection); // flags
            index.encode(&mut subsection); // global index
            name.encode(&mut subsection); // symbol name
        }

        subsection.encode(&mut linking);
        module.section(&CustomSection {
            name: "linking",
            data: &linking,
        });
    }

    // A `reloc.CODE` section is appended here with two relocations for the
    // two `global`-referencing instructions that were added.
    {
        let mut reloc = Vec::new();
        3u32.encode(&mut reloc); // target section (code is the 4th section, 3 when 0-indexed)
        6u32.encode(&mut reloc); // 6 relocations
        for index in 0..global_names.len() {
            reloc.push(0x07); // R_WASM_GLOBAL_INDEX_LEB
            global_offsets0[index as usize].encode(&mut reloc); // offset
            (function_names.len() + index).encode(&mut reloc); // symbol index
            reloc.push(0x07); // R_WASM_GLOBAL_INDEX_LEB
            global_offsets1[index as usize].encode(&mut reloc); // offset
            (function_names.len() + index).encode(&mut reloc); // symbol index
        }

        module.section(&CustomSection {
            name: "reloc.CODE",
            data: &reloc,
        });
    }

    module.finish()
}

/// This function produces the output of `llvm-ar crus libfoo.a foo.o` given
/// the object file above as input. The archive is what's eventually fed to
/// LLD.
///
/// Like above this is still tricky, mainly around the production of the symbol
/// table.
fn build_archive(wasm: &[u8], function_names: &[String]) -> Vec<u8> {
    use object::{bytes_of, endian::BigEndian, U32Bytes};

    let mut archive = Vec::new();
    archive.extend_from_slice(&object::archive::MAGIC);

    // The symbol table is in the "GNU" format which means it has a structure
    // that looks like:
    //
    // * a big-endian 32-bit integer for the number of symbols
    // * N big-endian 32-bit integers for the offset to the object file, within
    //   the entire archive, for which object has the symbol
    // * N nul-delimited strings for each symbol
    //
    // Here we're building an archive with just one symbol so it's a bit
    // easier. Note though we don't know the offset of our `intrinsics.o` up
    // front so it's left as 0 for now and filled in later.
    let mut symbol_table = Vec::new();
    symbol_table.extend_from_slice(bytes_of(&U32Bytes::new(
        BigEndian,
        function_names.len().try_into().unwrap(),
    )));
    for _ in function_names {
        symbol_table.extend_from_slice(bytes_of(&U32Bytes::new(BigEndian, 0)));
    }
    for name in function_names {
        symbol_table.extend_from_slice(name.as_bytes());
        symbol_table.push(0x00);
    }

    archive.extend_from_slice(bytes_of(&object::archive::Header {
        name: *b"/               ",
        date: *b"0           ",
        uid: *b"0     ",
        gid: *b"0     ",
        mode: *b"0       ",
        size: format!("{:<10}", symbol_table.len())
            .as_bytes()
            .try_into()
            .unwrap(),
        terminator: object::archive::TERMINATOR,
    }));
    let symtab_offset = archive.len();
    archive.extend_from_slice(&symbol_table);

    // All archive memberes must start on even offsets
    if archive.len() & 1 == 1 {
        archive.push(0x00);
    }

    // Now that we have the starting offset of the `intrinsics.o` file go back
    // and fill in the offset within the symbol table generated earlier.
    let member_offset = archive.len();
    archive[symtab_offset + 4..][..4].copy_from_slice(bytes_of(&U32Bytes::new(
        BigEndian,
        member_offset.try_into().unwrap(),
    )));

    archive.extend_from_slice(object::bytes_of(&object::archive::Header {
        name: *b"intrinsics.o    ",
        date: *b"0           ",
        uid: *b"0     ",
        gid: *b"0     ",
        mode: *b"644     ",
        size: format!("{:<10}", wasm.len()).as_bytes().try_into().unwrap(),
        terminator: object::archive::TERMINATOR,
    }));
    archive.extend_from_slice(&wasm);
    archive
}
