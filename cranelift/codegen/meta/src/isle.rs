use std::io::Result;

/// A list of compilations (transformations from ISLE source to
/// generated Rust source) that exist in the repository.
///
/// This list is used either to regenerate the Rust source in-tree (if
/// the `rebuild-isle` feature is enabled), or to verify that the ISLE
/// source in-tree corresponds to the ISLE source that was last used
/// to rebuild the Rust source (if the `rebuild-isle` feature is not
/// enabled).
#[derive(Clone, Debug)]
pub struct IsleCompilations {
    pub items: Vec<IsleCompilation>,
}

impl IsleCompilations {
    pub fn lookup(&self, name: &str) -> Option<&IsleCompilation> {
        for compilation in &self.items {
            if compilation.name == name {
                return Some(compilation);
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct IsleCompilation {
    pub name: String,
    pub output: std::path::PathBuf,
    pub tracked_inputs: Vec<std::path::PathBuf>,
    pub untracked_inputs: Vec<std::path::PathBuf>,
}

impl IsleCompilation {
    /// All inputs to the computation, tracked or untracked. May contain directories.
    pub fn inputs(&self) -> Vec<std::path::PathBuf> {
        self.tracked_inputs
            .iter()
            .chain(self.untracked_inputs.iter())
            .cloned()
            .collect()
    }

    /// All path inputs to the compilation. Directory inputs are expanded to the
    /// list of all ISLE files in the directory.
    pub fn paths(&self) -> Result<Vec<std::path::PathBuf>> {
        let mut paths = Vec::new();
        for input in self.inputs() {
            paths.extend(Self::expand_paths(&input)?);
        }
        Ok(paths)
    }

    fn expand_paths(input: &std::path::PathBuf) -> Result<Vec<std::path::PathBuf>> {
        if input.is_file() {
            return Ok(vec![input.clone()]);
        }

        if !input.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("ISLE input does not exist: {}", input.display()),
            ));
        }

        let mut paths = Vec::new();
        for entry in std::fs::read_dir(input).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "failed to read ISLE input directory {}: {e}",
                    input.display()
                ),
            )
        })? {
            let path = entry?.path();
            if let Some(ext) = path.extension() {
                if ext == "isle" {
                    paths.push(path);
                }
            }
        }
        Ok(paths)
    }
}

pub fn shared_isle_lower_paths(codegen_crate_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let inst_specs_isle = codegen_crate_dir.join("src").join("inst_specs.isle");
    let prelude_isle = codegen_crate_dir.join("src").join("prelude.isle");
    let prelude_lower_isle = codegen_crate_dir.join("src").join("prelude_lower.isle");
    // The shared instruction selector logic.
    vec![
        inst_specs_isle.clone(),
        prelude_isle.clone(),
        prelude_lower_isle.clone(),
    ]
}

/// Construct the list of compilations (transformations from ISLE
/// source to generated Rust source) that exist in the repository.
pub fn get_isle_compilations(
    codegen_crate_dir: &std::path::Path,
    gen_dir: &std::path::Path,
) -> IsleCompilations {
    // Preludes.
    let numerics_isle = gen_dir.join("numerics.isle");
    let clif_lower_isle = gen_dir.join("clif_lower.isle");
    let clif_opt_isle = gen_dir.join("clif_opt.isle");
    let prelude_isle = codegen_crate_dir.join("src").join("prelude.isle");
    let prelude_opt_isle = codegen_crate_dir.join("src").join("prelude_opt.isle");
    let prelude_lower_isle = codegen_crate_dir.join("src").join("prelude_lower.isle");
    #[cfg(feature = "pulley")]
    let pulley_gen = gen_dir.join("pulley_gen.isle");
    // Verification
    let prelude_spec_isle = codegen_crate_dir
        .join("src")
        .join("spec")
        .join("prelude_spec.isle");
    let inst_specs_isle = codegen_crate_dir
        .join("src")
        .join("spec")
        .join("inst_specs.isle");
    let inst_tags_isle = codegen_crate_dir
        .join("src")
        .join("spec")
        .join("inst_tags.isle");
    let fpconst_isle = codegen_crate_dir
        .join("src")
        .join("spec")
        .join("fpconst.isle");
    let state_isle = codegen_crate_dir
        .join("src")
        .join("spec")
        .join("state.isle");

    // Directory for mid-end optimizations.
    let src_opts = codegen_crate_dir.join("src").join("opts");

    // Directories for lowering backends.
    let src_isa_x64 = codegen_crate_dir.join("src").join("isa").join("x64");
    let src_isa_aarch64 = codegen_crate_dir.join("src").join("isa").join("aarch64");
    let src_isa_s390x = codegen_crate_dir.join("src").join("isa").join("s390x");
    let src_isa_risc_v = codegen_crate_dir.join("src").join("isa").join("riscv64");
    #[cfg(feature = "pulley")]
    let src_isa_pulley_shared = codegen_crate_dir
        .join("src")
        .join("isa")
        .join("pulley_shared");

    // This is a set of ISLE compilation units.
    //
    // The format of each entry is:
    //
    //     (output Rust code file, input ISLE source files)
    //
    // There should be one entry for each backend that uses ISLE for lowering,
    // and if/when we replace our peephole optimization passes with ISLE, there
    // should be an entry for each of those as well.
    //
    // N.B.: add any new compilation outputs to
    // `scripts/force-rebuild-isle.sh` if they do not fit the pattern
    // `cranelift/codegen/src/isa/*/lower/isle/generated_code.rs`!
    IsleCompilations {
        items: vec![
            // The mid-end optimization rules.
            IsleCompilation {
                name: "opt".to_string(),
                output: gen_dir.join("isle_opt.rs"),
                tracked_inputs: vec![
                    prelude_isle.clone(),
                    prelude_opt_isle,
                    prelude_spec_isle.clone(),
                    inst_specs_isle.clone(),
                    inst_tags_isle.clone(),
                    src_opts.join("arithmetic.isle"),
                    src_opts.join("bitops.isle"),
                    src_opts.join("cprop.isle"),
                    src_opts.join("extends.isle"),
                    src_opts.join("icmp.isle"),
                    src_opts.join("remat.isle"),
                    src_opts.join("selects.isle"),
                    src_opts.join("shifts.isle"),
                    src_opts.join("skeleton.isle"),
                    src_opts.join("spaceship.isle"),
                    src_opts.join("spectre.isle"),
                    src_opts.join("vector.isle"),
                ],
                untracked_inputs: vec![numerics_isle.clone(), clif_opt_isle],
            },
            // The x86-64 instruction selector.
            IsleCompilation {
                name: "x64".to_string(),
                output: gen_dir.join("isle_x64.rs"),
                tracked_inputs: vec![
                    prelude_isle.clone(),
                    prelude_lower_isle.clone(),
                    prelude_spec_isle.clone(),
                    inst_specs_isle.clone(),
                    inst_tags_isle.clone(),
                    state_isle.clone(),
                    src_isa_x64.join("inst.isle"),
                    src_isa_x64.join("lower.isle"),
                ],
                untracked_inputs: vec![
                    numerics_isle.clone(),
                    clif_lower_isle.clone(),
                    gen_dir.join("assembler.isle"),
                ],
            },
            // The aarch64 instruction selector.
            IsleCompilation {
                name: "aarch64".to_string(),
                output: gen_dir.join("isle_aarch64.rs"),
                tracked_inputs: vec![
                    prelude_isle.clone(),
                    prelude_lower_isle.clone(),
                    prelude_spec_isle.clone(),
                    inst_specs_isle.clone(),
                    inst_tags_isle.clone(),
                    fpconst_isle.clone(),
                    state_isle.clone(),
                    src_isa_aarch64.join("inst.isle"),
                    src_isa_aarch64.join("inst_neon.isle"),
                    src_isa_aarch64.join("spec"),
                    src_isa_aarch64.join("lower.isle"),
                    src_isa_aarch64.join("lower_dynamic_neon.isle"),
                ],
                untracked_inputs: vec![numerics_isle.clone(), clif_lower_isle.clone()],
            },
            // The s390x instruction selector.
            IsleCompilation {
                name: "s390x".to_string(),
                output: gen_dir.join("isle_s390x.rs"),
                tracked_inputs: vec![
                    prelude_isle.clone(),
                    prelude_lower_isle.clone(),
                    prelude_spec_isle.clone(),
                    inst_specs_isle.clone(),
                    inst_tags_isle.clone(),
                    src_isa_s390x.join("inst.isle"),
                    src_isa_s390x.join("lower.isle"),
                ],
                untracked_inputs: vec![numerics_isle.clone(), clif_lower_isle.clone()],
            },
            // The risc-v instruction selector.
            IsleCompilation {
                name: "riscv64".to_string(),
                output: gen_dir.join("isle_riscv64.rs"),
                tracked_inputs: vec![
                    prelude_isle.clone(),
                    prelude_lower_isle.clone(),
                    prelude_spec_isle.clone(),
                    inst_specs_isle.clone(),
                    inst_tags_isle.clone(),
                    src_isa_risc_v.join("inst.isle"),
                    src_isa_risc_v.join("inst_vector.isle"),
                    src_isa_risc_v.join("lower.isle"),
                ],
                untracked_inputs: vec![numerics_isle.clone(), clif_lower_isle.clone()],
            },
            // The Pulley instruction selector.
            #[cfg(feature = "pulley")]
            IsleCompilation {
                name: "pulley".to_string(),
                output: gen_dir.join("isle_pulley_shared.rs"),
                tracked_inputs: vec![
                    prelude_isle.clone(),
                    prelude_lower_isle.clone(),
                    src_isa_pulley_shared.join("inst.isle"),
                    src_isa_pulley_shared.join("lower.isle"),
                ],
                untracked_inputs: vec![
                    numerics_isle.clone(),
                    pulley_gen.clone(),
                    clif_lower_isle.clone(),
                ],
            },
        ],
    }
}
