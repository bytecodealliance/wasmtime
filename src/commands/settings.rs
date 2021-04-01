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

        let mut enums = (Vec::new(), 0);
        let mut nums = (Vec::new(), 0);
        let mut bools = (Vec::new(), 0);
        let mut presets = (Vec::new(), 0);

        for setting in settings.iter() {
            let (collection, max) = match setting.kind {
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

        if !enums.0.is_empty() {
            println!();
            Self::print_settings("Enum settings:", enums.0, enums.1);
        }

        if !nums.0.is_empty() {
            println!();
            Self::print_settings("Numerical settings:", nums.0, nums.1);
        }

        if !bools.0.is_empty() {
            println!();
            Self::print_settings("Boolean settings:", bools.0, bools.1);
        }

        if !presets.0.is_empty() {
            println!();
            Self::print_settings("Presets:", presets.0, presets.1);
        }

        if self.target.is_none() {
            let isa = settings.finish(settings::Flags::new(settings::builder()));
            println!();
            println!("Settings enabled for this host:");

            for flag in isa.enabled_isa_flags() {
                println!("  - {}", flag);
            }
        }

        Ok(())
    }

    fn print_settings(header: &str, settings: impl IntoIterator<Item = Setting>, width: usize) {
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
