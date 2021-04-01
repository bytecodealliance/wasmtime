//! The module that implements the `wasmtime settings` command.

use anyhow::{anyhow, Result};
use std::str::FromStr;
use structopt::StructOpt;
use wasmtime_environ::settings::{self, Setting, SettingKind};
use wasmtime_jit::native;

/// Displays available Cranelift settings for a target.
#[derive(StructOpt)]
#[structopt(name = "run")]
pub struct SettingsCommand {
    /// The target triple to get the settings for; defaults to the host triple.
    #[structopt(long, value_name = "TARGET")]
    target: Option<String>,
}

impl SettingsCommand {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        let settings = match &self.target {
            Some(target) => {
                native::lookup(target_lexicon::Triple::from_str(target).map_err(|e| anyhow!(e))?)?
            }
            None => native::builder(),
        };

        let mut enums = (Vec::new(), 0, "Enum settings:");
        let mut nums = (Vec::new(), 0, "Numerical settings:");
        let mut bools = (Vec::new(), 0, "Boolean settings:");
        let mut presets = (Vec::new(), 0, "Presets:");

        for setting in settings.iter() {
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
            println!("Target '{}' has no settings.", settings.triple());
            return Ok(());
        }

        println!("Cranelift settings for target '{}':", settings.triple());

        for (collection, max, header) in &mut [enums, nums, bools, presets] {
            if collection.is_empty() {
                continue;
            }

            collection.sort_by_key(|k| k.name);
            println!();
            Self::print_settings(header, collection, *max);
        }

        if self.target.is_none() {
            let isa = settings.finish(settings::Flags::new(settings::builder()));
            println!();
            println!("Settings inferred for the current host:");

            let mut enabled = isa.enabled_isa_flags();
            enabled.sort();

            for flag in enabled {
                println!("  {}", flag);
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
