use wasmtime::*;
use wasmtime_test_macros::wasmtime_test;

#[wasmtime_test(strategies(not(CraneliftNative)))]
#[cfg_attr(miri, ignore)]
fn ensure_compatibility_between_winch_and_table_lazy_init(config: &mut Config) -> Result<()> {
    config.table_lazy_init(false);
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

#[wasmtime_test(strategies(not(CraneliftNative)))]
#[cfg_attr(miri, ignore)]
fn ensure_compatibility_between_winch_and_signals_based_traps(config: &mut Config) -> Result<()> {
    config.signals_based_traps(false);
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

#[wasmtime_test(strategies(not(CraneliftNative)))]
#[cfg_attr(miri, ignore)]
fn ensure_compatibility_between_winch_and_generate_native_debuginfo(
    config: &mut Config,
) -> Result<()> {
    config.debug_info(true);
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
