use anyhow::{Context, Result};
use std::path::Path;
use std::{fs, path::PathBuf};

use crate::{Engine, Module};

/// A builder for combining DWARF `.dwp` files with WebAssembly modules.
pub struct ModuleBuilder<'a> {
    engine: &'a Engine,
    dwp_bytes: Option<Vec<u8>>,
    dwp_path: Option<PathBuf>,
}

/// Construction struct for creating `Module`s with DWARF loaded from a separate package, either froms a byte array
/// or from a file.  
///
/// If the file is not specified `ModuleBuilder` will attempt to auto-load from `path.dwp`
///
/// Note: `.dwo` files are not supported.
impl<'a> ModuleBuilder<'a> {
    /// Creates a new builder with the supplied ['Engine']
    /// DWARF packages can be added to the builder before compilation.
    pub fn new(engine: &'a Engine) -> Self {
        ModuleBuilder {
            engine,
            dwp_bytes: None,
            dwp_path: None,
        }
    }

    /// Explicitly specify DWARF packa
    pub fn dwarf_package(&mut self, bytes: &[u8]) -> &mut Self {
        self.dwp_bytes = Some(bytes.to_vec());

        self
    }

    /// Explicitly specify DWARF `.dwp` path.
    pub fn dwarf_package_file(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<&mut Self, &'static str> {
        self.dwp_path = Some(path.as_ref().to_path_buf());
        if !path.as_ref().exists() {
            return Err("DWARF dwp file does not exist");
        }

        Ok(self)
    }

    /// Attempt to find the DWARF package file from the wasm path, changing the extension to `.dwp`.
    pub fn wasm_path(&mut self, path: impl AsRef<Path>) -> &mut Self {
        let dwp_path = path.as_ref().with_extension("dwp");

        if dwp_path.exists() {
            self.dwp_path = Some(dwp_path);
        }

        self
    }

    fn read_dwarf_package(&mut self) {
        if self.dwp_bytes.is_none() {
            if let Some(dwp_path) = &self.dwp_path {
                self.dwp_bytes = Some(fs::read(dwp_path).expect("Failed to read DWARF dwp file."));
            }
        }
    }

    /// Finish the compilation by reading wasm from `path`
    ///
    /// Auto-loads `path.dwp` if `dwarf_package` or `dwarf_package_file`` weren't called
    pub fn compile_path(&mut self, path: impl AsRef<Path>) -> Result<Module> {
        self.read_dwarf_package();

        if self.dwp_bytes.is_none() {
            if self.dwp_path.is_none() {
                let dwp_path = path.as_ref().with_extension("dwp");

                self.dwp_bytes =
                    Some(fs::read(dwp_path).with_context(|| "Failed to read DWARF dwp file.")?);
            }
        }

        let module_bytes = fs::read(path)?;

        let module = Module::from_binary(&self.engine, &module_bytes, self.dwp_bytes.as_deref());

        module
    }

    /// Finish compilation by using the bytes provided as wasm.
    pub fn compile(&mut self, wasm: &[u8]) -> Result<Module> {
        self.read_dwarf_package();

        let module = Module::from_binary(&self.engine, wasm, self.dwp_bytes.as_deref());

        module
    }

    /// Builds the supplied WebAssembly module, combining with the DWARF package if one is found.
    pub fn precompile_module(&mut self, wasm: &[u8]) -> Result<Vec<u8>> {
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(&wasm)?;
        self.read_dwarf_package();
        let (v, _) = crate::compile::build_artifacts::<Vec<u8>>(
            &self.engine,
            &bytes,
            self.dwp_bytes.as_deref(),
        )?;
        Ok(v)
    }
}
