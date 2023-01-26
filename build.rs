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
///         .globaltype internal_global_ptr, i32
///         internal_global_ptr:
///     "
/// );
///
/// #[no_mangle]
/// extern "C" fn get_global_ptr() -> *mut u8 {
///     unsafe {
///         let ret: *mut u8;
///         std::arch::asm!(
///             "
///                 global.get internal_global_ptr
///             ",
///             out(local) ret,
///             options(nostack, readonly)
///         );
///         ret
///     }
/// }
///
/// #[no_mangle]
/// extern "C" fn set_global_ptr(val: *mut u8) {
///     unsafe {
///         std::arch::asm!(
///             "
///                 local.get {}
///                 global.set internal_global_ptr
///             ",
///             in(local) val,
///             options(nostack, readonly)
///         );
///     }
/// }
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
    module.section(&funcs);

    // Declare the globals.
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        &ConstExpr::i32_const(0),
    );
    module.section(&globals);

    // Here the `code` section is defined. This is tricky because an offset is
    // needed within the code section itself for the `reloc.CODE` section
    // later. At this time `wasm-encoder` doesn't give enough functionality to
    // use the high-level APIs. so everything is done manually here.
    //
    // First the function body is created and then it's appended into a code
    // section.

    let mut code = Vec::new();
    2u32.encode(&mut code);

    // get_global_ptr
    let global_offset0 = {
        let mut body = Vec::new();
        0u32.encode(&mut body); // no locals
        let global_offset = body.len() + 1;
        // global.get 0 ;; but with maximal encoding of the 0
        body.extend_from_slice(&[0x23, 0x80, 0x80, 0x80, 0x80, 0x00]);
        End.encode(&mut body);
        body.len().encode(&mut code); // length of the function
        let offset = code.len() + global_offset;
        code.extend_from_slice(&body); // the function itself
        offset
    };

    // set_global_ptr
    let global_offset1 = {
        let mut body = Vec::new();
        0u32.encode(&mut body); // no locals
        LocalGet(0).encode(&mut body);
        let global_offset = body.len() + 1;
        // global.set 0 ;; but with maximal encoding of the 0
        body.extend_from_slice(&[0x24, 0x80, 0x80, 0x80, 0x80, 0x00]);
        End.encode(&mut body);
        body.len().encode(&mut code); // length of the function
        let offset = code.len() + global_offset;
        code.extend_from_slice(&body); // the function itself
        offset
    };

    module.section(&RawSection {
        id: SectionId::Code as u8,
        data: &code,
    });

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
        3u32.encode(&mut subsection); // 3 symbols (2 functions + 1 global)

        subsection.push(0x00); // SYMTAB_FUNCTION
        0x00.encode(&mut subsection); // flags
        0u32.encode(&mut subsection); // function index
        "get_global_ptr".encode(&mut subsection); // symbol name

        subsection.push(0x00); // SYMTAB_FUNCTION
        0x00.encode(&mut subsection); // flags
        1u32.encode(&mut subsection); // function index
        "set_global_ptr".encode(&mut subsection); // symbol name

        subsection.push(0x02); // SYMTAB_GLOBAL
        0x02.encode(&mut subsection); // flags
        0u32.encode(&mut subsection); // global index
        "internal_global_ptr".encode(&mut subsection); // symbol name

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
        2u32.encode(&mut reloc); // 2 relocations

        reloc.push(0x07); // R_WASM_GLOBAL_INDEX_LEB
        global_offset0.encode(&mut reloc); // offset
        2u32.encode(&mut reloc); // symbol index

        reloc.push(0x07); // R_WASM_GLOBAL_INDEX_LEB
        global_offset1.encode(&mut reloc); // offset
        2u32.encode(&mut reloc); // symbol index

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
    // Here we're building an archive with just one symbol so it's a bit
    // easier. Note though we don't know the offset of our `intrinsics.o` up
    // front so it's left as 0 for now and filled in later.
    let mut symbol_table = Vec::new();
    symbol_table.extend_from_slice(bytes_of(&U32Bytes::new(BigEndian, 2)));
    symbol_table.extend_from_slice(bytes_of(&U32Bytes::new(BigEndian, 0)));
    symbol_table.extend_from_slice(bytes_of(&U32Bytes::new(BigEndian, 0)));
    symbol_table.extend_from_slice(b"get_global_ptr\0");
    symbol_table.extend_from_slice(b"set_global_ptr\0");

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
    archive[symtab_offset + 8..][..4].copy_from_slice(bytes_of(&U32Bytes::new(
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
