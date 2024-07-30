//! The module that implements the `wasmtime settings` command.

use anyhow::{anyhow, Result};
use clap::Parser;
use serde::{ser::SerializeMap, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;
use wasmtime_environ::{CompilerBuilder, FlagValue, Setting, SettingKind, Tunables};

/// Displays available Cranelift settings for a target.
#[derive(Parser, PartialEq)]
pub struct SettingsCommand {
    /// The target triple to get the settings for; defaults to the host triple.
    #[arg(long, value_name = "TARGET")]
    target: Option<String>,

    /// Switch output format to JSON
    #[arg(long)]
    json: bool,
}

struct SettingData(Setting);

impl Serialize for SettingData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("name", self.0.name)?;
        map.serialize_entry("description", self.0.description)?;
        map.serialize_entry("values", &self.0.values)?;
        map.end()
    }
}

// Gather together all of the setting data to displays
#[derive(serde_derive::Serialize)]
struct Settings {
    triple: String,

    enums: Vec<SettingData>,
    nums: Vec<SettingData>,
    bools: Vec<SettingData>,
    presets: Vec<SettingData>,

    inferred: Option<Vec<&'static str>>,
}

impl Settings {
    fn from_builder(builder: &Box<dyn CompilerBuilder>) -> Settings {
        let mut settings = Settings {
            triple: builder.triple().to_string(),
            enums: Vec::new(),
            nums: Vec::new(),
            bools: Vec::new(),
            presets: Vec::new(),
            inferred: None,
        };
        settings.add_settings(builder.settings());
        settings
    }

    fn infer(&mut self, builder: &Box<dyn CompilerBuilder>) -> Result<()> {
        let compiler = builder.build()?;
        let values = compiler.isa_flags().into_iter().collect::<BTreeMap<_, _>>();
        let mut result = Vec::new();
        for (name, value) in values {
            if let FlagValue::Bool(true) = value {
                result.push(name);
            }
        }

        self.inferred = Some(result);

        Ok(())
    }

    fn add_setting(&mut self, setting: Setting) {
        let collection = match setting.kind {
            SettingKind::Enum => &mut self.enums,
            SettingKind::Num => &mut self.nums,
            SettingKind::Bool => &mut self.bools,
            SettingKind::Preset => &mut self.presets,
        };
        collection.push(SettingData(setting));
    }

    fn add_settings<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = Setting>,
    {
        for item in iterable.into_iter() {
            self.add_setting(item);
        }
    }

    fn is_empty(&self) -> bool {
        self.enums.is_empty()
            && self.nums.is_empty()
            && self.bools.is_empty()
            && self.presets.is_empty()
    }
}

impl SettingsCommand {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        // Gather settings from the cranelift compiler builder
        let mut builder = wasmtime_cranelift::builder(None)?;
        let tunables = if let Some(target) = &self.target {
            let target = target_lexicon::Triple::from_str(target).map_err(|e| anyhow!(e))?;
            let tunables = Tunables::default_for_target(&target)?;
            builder.target(target)?;
            tunables
        } else {
            Tunables::default_host()
        };

        builder.set_tunables(tunables)?;
        let mut settings = Settings::from_builder(&builder);

        // Add inferred settings if no target specified
        if self.target.is_none() {
            settings.infer(&builder)?;
        }

        // Print settings
        if self.json {
            self.print_json(settings)
        } else {
            self.print_human_readable(settings)
        }
    }

    fn print_json(self, settings: Settings) -> Result<()> {
        println!("{}", serde_json::to_string_pretty(&settings)?);
        Ok(())
    }

    fn print_human_readable(self, settings: Settings) -> Result<()> {
        if settings.is_empty() {
            println!("Target '{}' has no settings.", settings.triple);
            return Ok(());
        }

        println!("Cranelift settings for target '{}':", settings.triple);

        Self::print_settings_human_readable("Boolean settings:", &settings.bools);
        Self::print_settings_human_readable("Enum settings:", &settings.enums);
        Self::print_settings_human_readable("Numerical settings:", &settings.nums);
        Self::print_settings_human_readable("Presets:", &settings.presets);

        if let Some(inferred) = settings.inferred {
            println!();
            println!("Settings inferred for the current host:");

            for name in inferred {
                println!("  {name}");
            }
        }

        Ok(())
    }

    fn print_settings_human_readable(header: &str, settings: &[SettingData]) {
        if settings.is_empty() {
            return;
        }

        println!();
        println!("{header}");

        let width = settings.iter().map(|s| s.0.name.len()).max().unwrap_or(0);

        for setting in settings {
            println!(
                "  {:width$} {}{}",
                setting.0.name,
                setting.0.description,
                setting
                    .0
                    .values
                    .map(|v| format!(" Supported values: {}.", v.join(", ")))
                    .unwrap_or("".to_string()),
                width = width + 2
            );
        }
    }
}
