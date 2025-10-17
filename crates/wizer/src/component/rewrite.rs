use crate::component::ComponentContext;
use crate::component::info::RawSection;
use crate::component::snapshot::ComponentSnapshot;
use crate::{FuncRenames, Wizer};

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
                            let rewritten_wasm =
                                self.rewrite(module, snapshot, &FuncRenames::default());
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
                    encoder.section(s);
                }
            }
        }

        encoder.finish()
    }
}
