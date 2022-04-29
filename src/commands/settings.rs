//! The module that implements the `wasmtime settings` command.

use anyhow::{anyhow, Result};
use clap::Parser;
use std::collections::BTreeMap;
use std::str::FromStr;
use wasmtime_environ::{FlagValue, Setting, SettingKind};

/// Displays available Cranelift settings for a target.
#[derive(Parser)]
#[clap(name = "run")]
pub struct SettingsCommand {
    /// The target triple to get the settings for; defaults to the host triple.
    #[clap(long, value_name = "TARGET")]
    target: Option<String>,
}

impl SettingsCommand {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        let mut builder = wasmtime_cranelift::builder();
        if let Some(target) = &self.target {
            let target = target_lexicon::Triple::from_str(target).map_err(|e| anyhow!(e))?;
            builder.target(target)?;
        }

        let mut enums = (Vec::new(), 0, "Enum settings:");
        let mut nums = (Vec::new(), 0, "Numerical settings:");
        let mut bools = (Vec::new(), 0, "Boolean settings:");
        let mut presets = (Vec::new(), 0, "Presets:");

        for setting in builder.settings() {
            let (collection, max, _) = match setting.kind {
                SettingKind::Enum => &mut enums,
                SettingKind::Num => &mut nums,
                SettingKind::Bool => &mut bools,
                SettingKind::Preset => &mut presets,
            };

            if setting.name.len() > *max {
                *max = setting.name.len();
            }

            collection.push(setting);
        }

        if enums.0.is_empty() && nums.0.is_empty() && bools.0.is_empty() && presets.0.is_empty() {
            println!("Target '{}' has no settings.", builder.triple());
            return Ok(());
        }

        println!("Cranelift settings for target '{}':", builder.triple());

        for (collection, max, header) in &mut [enums, nums, bools, presets] {
            if collection.is_empty() {
                continue;
            }

            collection.sort_by_key(|k| k.name);
            println!();
            Self::print_settings(header, collection, *max);
        }

        if self.target.is_none() {
            let compiler = builder.build()?;
            println!();
            println!("Settings inferred for the current host:");

            let values = compiler.isa_flags().into_iter().collect::<BTreeMap<_, _>>();

            for (name, value) in values {
                if let FlagValue::Bool(true) = value {
                    println!("  {}", name);
                }
            }
        }

        Ok(())
    }

    fn print_settings(header: &str, settings: &[Setting], width: usize) {
        println!("{}", header);
        for setting in settings {
            println!(
                "  {:width$} {}{}",
                setting.name,
                setting.description,
                setting
                    .values
                    .map(|v| format!(" Supported values: {}.", v.join(", ")))
                    .unwrap_or("".to_string()),
                width = width + 2
            );
        }
    }
}
