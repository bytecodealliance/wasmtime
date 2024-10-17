use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn ensure_compatibility_between_winch_and_table_lazy_init() -> Result<()> {
    let mut config = Config::new();
    config.table_lazy_init(false);
    config.strategy(Strategy::Winch);
    let result = Engine::new(&config);
    match result {
        Ok(_) => {
            anyhow::bail!("Expected incompatibility between the `table_lazy_init` option and Winch")
        }
        Err(e) => {
            assert_eq!(
                e.to_string(),
                "Winch requires the table-lazy-init option to be enabled"
            );
        }
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn ensure_compatibility_between_winch_and_signals_based_traps() -> Result<()> {
    let mut config = Config::new();
    config.signals_based_traps(false);
    config.strategy(Strategy::Winch);
    let result = Engine::new(&config);
    match result {
        Ok(_) => {
            anyhow::bail!(
                "Expected incompatibility between the `signals_based_traps` option and Winch"
            )
        }
        Err(e) => {
            assert_eq!(
                e.to_string(),
                "Winch requires the signals-based-traps option to be enabled"
            );
        }
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn ensure_compatibility_between_winch_and_generate_native_debuginfo() -> Result<()> {
    let mut config = Config::new();
    config.debug_info(true);
    config.strategy(Strategy::Winch);
    let result = Engine::new(&config);
    match result {
        Ok(_) => {
            anyhow::bail!(
                "Expected incompatibility between the `generate_native_debuginfo` option and Winch"
            )
        }
        Err(e) => {
            assert_eq!(
                e.to_string(),
                "Winch does not currently support generating native debug information"
            );
        }
    }

    Ok(())
}
