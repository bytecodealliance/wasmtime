use crate::component::ComponentContext;
use crate::component::info::RawSection;
use crate::component::snapshot::ComponentSnapshot;
use crate::{FuncRenames, Wizer};
use wasm_encoder::reencode::{Reencode, ReencodeComponent};

impl Wizer {
    /// Helper method which is the equivalent of [`Wizer::rewrite`], but for
    /// components.
    ///
    /// This effectively plumbs through all non-module sections as-is and
    /// updates module sections with whatever [`Wizer::rewrite`] returns.
    pub(crate) fn rewrite_component(
        &self,
        component: &mut ComponentContext<'_>,
        snapshot: &ComponentSnapshot,
    ) -> Vec<u8> {
        let mut encoder = wasm_encoder::Component::new();
        let mut reencoder = Reencoder {
            funcs: 0,
            removed_func: None,
            wizer: self,
        };

        let mut module_index = 0;
        for section in component.sections.iter_mut() {
            match section {
                RawSection::Module(module) => {
                    let snapshot = snapshot
                        .modules
                        .iter()
                        .find(|(i, _)| *i == module_index)
                        .map(|(_, s)| s);
                    module_index += 1;
                    match snapshot {
                        // This module's snapshot is used for [`Wizer::rewrite`]
                        // and the results of that are spliced into the
                        // component.
                        Some(snapshot) => {
                            let rewritten_wasm = self.rewrite(
                                module,
                                snapshot,
                                &FuncRenames::default(),
                                false,
                                false,
                            );
                            encoder.section(&wasm_encoder::RawSection {
                                id: wasm_encoder::ComponentSectionId::CoreModule as u8,
                                data: &rewritten_wasm,
                            });
                        }

                        // This module wasn't instantiated and has no snapshot,
                        // plumb it through as-is.
                        None => {
                            let mut module_encoder = wasm_encoder::Module::new();
                            for section in module.raw_sections() {
                                module_encoder.section(section);
                            }
                            encoder.section(&wasm_encoder::ModuleSection(&module_encoder));
                        }
                    }
                }
                RawSection::Raw(s) => {
                    reencoder.raw_section(&mut encoder, s);
                }
            }
        }

        encoder.finish()
    }
}

struct Reencoder<'a> {
    /// Number of defined functions encountered so far.
    funcs: u32,
    /// Index of the start function that's being removed, used to renumber all
    /// other functions.
    removed_func: Option<u32>,
    /// Wizer configuration.
    wizer: &'a Wizer,
}

impl Reencoder<'_> {
    fn raw_section(
        &mut self,
        encoder: &mut wasm_encoder::Component,
        section: &wasm_encoder::RawSection,
    ) {
        match section.id {
            // These can't define component functions so the sections are
            // plumbed as-is.
            id if id == wasm_encoder::ComponentSectionId::CoreCustom as u8
                || id == wasm_encoder::ComponentSectionId::CoreInstance as u8
                || id == wasm_encoder::ComponentSectionId::CoreType as u8
                || id == wasm_encoder::ComponentSectionId::Component as u8
                || id == wasm_encoder::ComponentSectionId::Type as u8 =>
            {
                encoder.section(section);
            }

            id if id == wasm_encoder::ComponentSectionId::CoreModule as u8 => {
                panic!("should happen in caller");
            }
            id if id == wasm_encoder::ComponentSectionId::Start as u8 => {
                // Component start sections aren't supported yet anyway
                todo!()
            }

            // These sections all might affect or refer to component function
            // indices so they're reencoded here, optionally updating function
            // indices in case the index is higher than the one that we're
            // removing.
            id if id == wasm_encoder::ComponentSectionId::Instance as u8 => {
                self.rewrite(
                    encoder,
                    section.data,
                    Self::parse_component_instance_section,
                );
            }
            id if id == wasm_encoder::ComponentSectionId::Alias as u8 => {
                self.rewrite(encoder, section.data, Self::parse_component_alias_section);
            }
            id if id == wasm_encoder::ComponentSectionId::CanonicalFunction as u8 => {
                self.rewrite(
                    encoder,
                    section.data,
                    Self::parse_component_canonical_section,
                );
            }
            id if id == wasm_encoder::ComponentSectionId::Import as u8 => {
                self.rewrite(encoder, section.data, Self::parse_component_import_section);
            }
            id if id == wasm_encoder::ComponentSectionId::Export as u8 => {
                self.rewrite(encoder, section.data, Self::parse_component_export_section);
            }
            other => panic!("unexpected component section id: {other}"),
        }
    }

    fn rewrite<'a, T, S>(
        &mut self,
        encoder: &mut wasm_encoder::Component,
        data: &'a [u8],
        f: fn(&mut Self, dst: &mut S, wasmparser::SectionLimited<'a, T>) -> Result<(), Error>,
    ) where
        T: wasmparser::FromReader<'a>,
        S: Default + wasm_encoder::ComponentSection,
    {
        let mut section = S::default();
        f(
            self,
            &mut section,
            wasmparser::SectionLimited::new(wasmparser::BinaryReader::new(data, 0)).unwrap(),
        )
        .unwrap();
        encoder.section(&section);
    }
}

impl Reencode for Reencoder<'_> {
    type Error = std::convert::Infallible;
}
type Error = wasm_encoder::reencode::Error<std::convert::Infallible>;

impl ReencodeComponent for Reencoder<'_> {
    fn component_func_index(&mut self, original_index: u32) -> u32 {
        match self.removed_func {
            None => original_index,
            Some(removed) => {
                if original_index < removed {
                    original_index
                } else if original_index == removed {
                    panic!("referenced removed function")
                } else {
                    original_index - 1
                }
            }
        }
    }

    fn parse_component_alias_section(
        &mut self,
        aliases: &mut wasm_encoder::ComponentAliasSection,
        section: wasmparser::ComponentAliasSectionReader<'_>,
    ) -> Result<(), Error> {
        for alias in section.clone() {
            let alias = alias?;
            if let wasmparser::ComponentAlias::InstanceExport {
                kind: wasmparser::ComponentExternalKind::Func,
                ..
            } = alias
            {
                self.funcs += 1;
            }
        }

        wasm_encoder::reencode::component_utils::parse_component_alias_section(
            self, aliases, section,
        )
    }

    fn parse_component_canonical_section(
        &mut self,
        canonicals: &mut wasm_encoder::CanonicalFunctionSection,
        section: wasmparser::ComponentCanonicalSectionReader<'_>,
    ) -> Result<(), Error> {
        for canonical in section.clone() {
            let canonical = canonical?;
            if let wasmparser::CanonicalFunction::Lift { .. } = canonical {
                self.funcs += 1;
            }
        }

        wasm_encoder::reencode::component_utils::parse_component_canonical_section(
            self, canonicals, section,
        )
    }

    fn parse_component_import_section(
        &mut self,
        imports: &mut wasm_encoder::ComponentImportSection,
        section: wasmparser::ComponentImportSectionReader<'_>,
    ) -> Result<(), Error> {
        for import in section.clone() {
            let import = import?;
            if let wasmparser::ComponentExternalKind::Func = import.ty.kind() {
                self.funcs += 1;
            }
        }

        wasm_encoder::reencode::component_utils::parse_component_import_section(
            self, imports, section,
        )
    }

    fn parse_component_export_section(
        &mut self,
        exports: &mut wasm_encoder::ComponentExportSection,
        section: wasmparser::ComponentExportSectionReader<'_>,
    ) -> Result<(), Error> {
        for export in section {
            let export = export?;
            if !self.wizer.get_keep_init_func() && export.name.0 == self.wizer.get_init_func() {
                self.removed_func = Some(self.funcs);
            } else {
                if export.kind == wasmparser::ComponentExternalKind::Func {
                    self.funcs += 1;
                }
                self.parse_component_export(exports, export)?;
            }
        }
        Ok(())
    }
}
