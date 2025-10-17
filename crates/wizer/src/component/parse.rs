use crate::component::ComponentContext;
use anyhow::{Result, bail};
use std::collections::hash_map::Entry;
use wasmparser::{
    CanonicalFunction, ComponentAlias, ComponentExternalKind, ComponentOuterAliasKind, Encoding,
    Instance, Parser, Payload,
};

/// Parse the given Wasm bytes into a `ComponentContext` tree.
///
/// This will parse the input component and build up metadata that Wizer needs
/// to know about the component. At this time there are limitations to this
/// parsing phase which in theory could be lifted in the future but serve as
/// simplifying assumptions for now:
///
/// * Nested components are not supported.
/// * Imported modules or components are not supported.
/// * Instantiating a module twice is not supported.
/// * Component-level start functions are not supported.
///
/// Some of these restrictions are likely to be loosened over time, however.
pub(crate) fn parse<'a>(full_wasm: &'a [u8]) -> anyhow::Result<ComponentContext<'a>> {
    let mut component = ComponentContext::default();
    let mut parser = Parser::new(0).parse_all(full_wasm);

    while let Some(payload) = parser.next() {
        let payload = payload?;

        match &payload {
            // Module sections get parsed with wizer's core wasm support.
            Payload::ModuleSection { .. } => {
                let info = crate::parse::parse_with(&full_wasm, &mut parser)?;
                component.push_module_section(info);
            }

            // All other sections get pushed raw as-is into the component.
            _ => {
                if let Some((id, range)) = payload.as_section() {
                    component.push_raw_section(wasm_encoder::RawSection {
                        id,
                        data: &full_wasm[range],
                    });
                }
            }
        }

        // Further validation/handling of each section, mostly about maintaining
        // index spaces so we know what indices are used when the instrumented
        // component is produced.
        match payload {
            Payload::Version {
                encoding: Encoding::Module,
                ..
            } => {
                bail!("expected a component, found a core module");
            }

            Payload::ComponentSection { .. } => {
                bail!("wizer does not currently support nested components");
            }

            Payload::InstanceSection(reader) => {
                for instance in reader {
                    let instance_index = component.inc_core_instances();

                    if let Instance::Instantiate { module_index, .. } = instance? {
                        match component.core_instantiations.entry(module_index) {
                            Entry::Vacant(entry) => {
                                entry.insert(instance_index);
                            }
                            Entry::Occupied(_) => bail!("modules may be instantiated at most once"),
                        }
                    }
                }
            }
            Payload::ComponentInstanceSection(reader) => {
                for _ in reader {
                    component.inc_instances();
                }
            }

            Payload::ComponentAliasSection(reader) => {
                for alias in reader {
                    match alias? {
                        ComponentAlias::CoreInstanceExport { kind, .. } => {
                            component.inc_core(kind);
                        }
                        ComponentAlias::InstanceExport { kind, .. } => {
                            validate_item_kind(kind, "aliases")?;
                            component.inc(kind);
                        }
                        ComponentAlias::Outer { kind, .. } => match kind {
                            ComponentOuterAliasKind::CoreType => {}
                            ComponentOuterAliasKind::Type => {
                                component.inc_types();
                            }
                            ComponentOuterAliasKind::CoreModule => {
                                bail!("wizer does not currently support module aliases");
                            }
                            ComponentOuterAliasKind::Component => {
                                bail!("wizer does not currently support component aliases");
                            }
                        },
                    }
                }
            }

            Payload::ComponentCanonicalSection(reader) => {
                for function in reader {
                    match function? {
                        CanonicalFunction::Lift { .. } => {
                            component.inc_funcs();
                        }
                        _ => {
                            component.inc_core_funcs();
                        }
                    }
                }
            }

            Payload::ComponentImportSection(reader) => {
                for import in reader {
                    let kind = import?.ty.kind();
                    validate_item_kind(kind, "imports")?;
                    component.inc(kind);
                }
            }

            Payload::ComponentExportSection(reader) => {
                for export in reader {
                    let kind = export?.kind;
                    validate_item_kind(kind, "exports")?;
                    component.inc(kind);
                }
            }

            Payload::ComponentTypeSection(reader) => {
                for _ in reader {
                    component.inc_types();
                }
            }

            // The `start` section for components is itself not stable, so not
            // super urgent to handle. A simplifying assumption is made to just
            // reject it outright for now if it shows up.
            Payload::ComponentStartSection { .. } => {
                bail!("wizer does not currently support component start functions");
            }

            _ => {}
        }
    }

    Ok(component)
}

fn validate_item_kind(kind: ComponentExternalKind, msg: &str) -> Result<()> {
    match kind {
        // Aliasing modules would require keeping track of where a module's
        // original definition is. There's not much usage of this in the wild
        // so reject it for now as a simplifying assumption.
        ComponentExternalKind::Module => {
            bail!("wizer does not currently support module {msg}");
        }

        // Aliasing components deals with nested instantiations or similar,
        // where a simplifying assumption is made to not worry about that for now.
        ComponentExternalKind::Component => {
            bail!("wizer does not currently support component {msg}");
        }

        // Like start sections, support for this is just deferred to some other
        // time, if ever.
        ComponentExternalKind::Value => {
            bail!("wizer does not currently support value {msg}");
        }

        ComponentExternalKind::Func
        | ComponentExternalKind::Type
        | ComponentExternalKind::Instance => Ok(()),
    }
}
