use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let wasm = build_raw_intrinsics();
    let archive = build_archive(&wasm);

    std::fs::write(out_dir.join("libwasm-raw-intrinsics.a"), &archive).unwrap();
    println!("cargo:rustc-link-lib=static=wasm-raw-intrinsics");
    println!(
        "cargo:rustc-link-search=native={}",
        out_dir.to_str().unwrap()
    );

    // Some specific flags to `wasm-ld` to inform the shape of this adapter.
    // Notably we're importing memory from the main module and additionally our
    // own module has no stack at all since it's specifically allocated at
    // startup.
    println!("cargo:rustc-link-arg=--import-memory");
    println!("cargo:rustc-link-arg=-zstack-size=0");
}

/// This function will produce a wasm module which is itself an object file
/// that is the basic equivalent of:
///
/// ```rust
/// std::arch::global_asm!(
///     "
///         .globaltype internal_state_ptr, i32
///         internal_state_ptr:
///     "
/// );
///
/// #[no_mangle]
/// extern "C" fn get_state_ptr() -> *mut u8 {
///     unsafe {
///         let ret: *mut u8;
///         std::arch::asm!(
///             "
///                 global.get internal_state_ptr
///             ",
///             out(local) ret,
///             options(nostack, readonly)
///         );
///         ret
///     }
/// }
///
/// #[no_mangle]
/// extern "C" fn set_state_ptr(val: *mut u8) {
///     unsafe {
///         std::arch::asm!(
///             "
///                 local.get {}
///                 global.set internal_state_ptr
///             ",
///             in(local) val,
///             options(nostack, readonly)
///         );
///     }
/// }
///
/// // And likewise for `allocation_state`, `get_allocation_state`, and `set_allocation_state`
/// ```
///
/// The main trickiness here is getting the `reloc.CODE` and `linking` sections
/// right.
fn build_raw_intrinsics() -> Vec<u8> {
    use wasm_encoder::Instruction::*;
    use wasm_encoder::*;

    let mut module = Module::new();

    let mut types = TypeSection::new();
    types.function([], [ValType::I32]);
    types.function([ValType::I32], []);
    module.section(&types);

    // Declare the functions, using the type we just added.
    let mut funcs = FunctionSection::new();
    funcs.function(0);
    funcs.function(1);
    funcs.function(0);
    funcs.function(1);
    module.section(&funcs);

    // Declare the globals.
    let mut globals = GlobalSection::new();
    // internal_state_ptr
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(0),
    );
    // allocation_state
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(0),
    );
    module.section(&globals);

    // Here the `code` section is defined. This is tricky because an offset is
    // needed within the code section itself for the `reloc.CODE` section
    // later. At this time `wasm-encoder` doesn't give enough functionality to
    // use the high-level APIs. so everything is done manually here.
    //
    // First the function bodies are created and then they're appended into a
    // code section.

    let mut code = Vec::new();
    4u32.encode(&mut code); // number of functions

    let global_get = 0x23;
    let global_set = 0x24;

    let encode = |code: &mut _, global, instruction| {
        assert!(global < 0x7F);

        let mut body = Vec::new();
        0u32.encode(&mut body); // no locals
        if instruction == global_set {
            LocalGet(0).encode(&mut body);
        }
        let global_offset = body.len() + 1;
        // global.get $global ;; but with maximal encoding of $global
        body.extend_from_slice(&[instruction, 0x80u8 + global, 0x80, 0x80, 0x80, 0x00]);
        End.encode(&mut body);
        body.len().encode(code); // length of the function
        let offset = code.len() + global_offset;
        code.extend_from_slice(&body); // the function itself
        offset
    };

    let internal_state_ptr_ref1 = encode(&mut code, 0, global_get); // get_state_ptr
    let internal_state_ptr_ref2 = encode(&mut code, 0, global_set); // set_state_ptr
    let allocation_state_ref1 = encode(&mut code, 1, global_get); // get_allocation_state
    let allocation_state_ref2 = encode(&mut code, 1, global_set); // set_allocation_state

    module.section(&RawSection {
        id: SectionId::Code as u8,
        data: &code,
    });

    // Here the linking section is constructed. There is one symbol for each function and global. The injected
    // globals here are referenced in the relocations below.
    //
    // More information about this format is at
    // https://github.com/WebAssembly/tool-conventions/blob/main/Linking.md
    {
        let mut linking = Vec::new();
        linking.push(0x02); // version

        linking.push(0x08); // `WASM_SYMBOL_TABLE`
        let mut subsection = Vec::new();
        6u32.encode(&mut subsection); // 6 symbols (4 functions + 2 globals)

        subsection.push(0x00); // SYMTAB_FUNCTION
        0x00.encode(&mut subsection); // flags
        0u32.encode(&mut subsection); // function index
        "get_state_ptr".encode(&mut subsection); // symbol name

        subsection.push(0x00); // SYMTAB_FUNCTION
        0x00.encode(&mut subsection); // flags
        1u32.encode(&mut subsection); // function index
        "set_state_ptr".encode(&mut subsection); // symbol name

        subsection.push(0x00); // SYMTAB_FUNCTION
        0x00.encode(&mut subsection); // flags
        2u32.encode(&mut subsection); // function index
        "get_allocation_state".encode(&mut subsection); // symbol name

        subsection.push(0x00); // SYMTAB_FUNCTION
        0x00.encode(&mut subsection); // flags
        3u32.encode(&mut subsection); // function index
        "set_allocation_state".encode(&mut subsection); // symbol name

        subsection.push(0x02); // SYMTAB_GLOBAL
        0x02.encode(&mut subsection); // flags (WASM_SYM_BINDING_LOCAL)
        0u32.encode(&mut subsection); // global index
        "internal_state_ptr".encode(&mut subsection); // symbol name

        subsection.push(0x02); // SYMTAB_GLOBAL
        0x00.encode(&mut subsection); // flags
        1u32.encode(&mut subsection); // global index
        "allocation_state".encode(&mut subsection); // symbol name

        subsection.encode(&mut linking);
        module.section(&CustomSection {
            name: "linking".into(),
            data: linking.into(),
        });
    }

    // A `reloc.CODE` section is appended here with relocations for the
    // `global`-referencing instructions that were added.
    {
        let mut reloc = Vec::new();
        3u32.encode(&mut reloc); // target section (code is the 4th section, 3 when 0-indexed)
        4u32.encode(&mut reloc); // 4 relocations

        reloc.push(0x07); // R_WASM_GLOBAL_INDEX_LEB
        internal_state_ptr_ref1.encode(&mut reloc); // offset
        4u32.encode(&mut reloc); // symbol index

        reloc.push(0x07); // R_WASM_GLOBAL_INDEX_LEB
        internal_state_ptr_ref2.encode(&mut reloc); // offset
        4u32.encode(&mut reloc); // symbol index

        reloc.push(0x07); // R_WASM_GLOBAL_INDEX_LEB
        allocation_state_ref1.encode(&mut reloc); // offset
        5u32.encode(&mut reloc); // symbol index

        reloc.push(0x07); // R_WASM_GLOBAL_INDEX_LEB
        allocation_state_ref2.encode(&mut reloc); // offset
        5u32.encode(&mut reloc); // symbol index

        module.section(&CustomSection {
            name: "reloc.CODE".into(),
            data: reloc.into(),
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
fn build_archive(wasm: &[u8]) -> Vec<u8> {
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
    // Here we're building an archive with just a few symbols so it's a bit
    // easier. Note though we don't know the offset of our `intrinsics.o` up
    // front so it's left as 0 for now and filled in later.

    let syms = [
        "get_state_ptr",
        "set_state_ptr",
        "get_allocation_state",
        "set_allocation_state",
        "allocation_state",
    ];

    let mut symbol_table = Vec::new();
    symbol_table.extend_from_slice(bytes_of(&U32Bytes::new(BigEndian, syms.len() as u32)));
    for _ in syms.iter() {
        symbol_table.extend_from_slice(bytes_of(&U32Bytes::new(BigEndian, 0)));
    }
    for s in syms.iter() {
        symbol_table.extend_from_slice(&std::ffi::CString::new(*s).unwrap().into_bytes_with_nul());
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

    // All archive members must start on even offsets
    if archive.len() & 1 == 1 {
        archive.push(0x00);
    }

    // Now that we have the starting offset of the `intrinsics.o` file go back
    // and fill in the offset within the symbol table generated earlier.
    let member_offset = archive.len();
    for (index, _) in syms.iter().enumerate() {
        let index = index + 1;
        archive[symtab_offset + (index * 4)..][..4].copy_from_slice(bytes_of(&U32Bytes::new(
            BigEndian,
            member_offset.try_into().unwrap(),
        )));
    }

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
