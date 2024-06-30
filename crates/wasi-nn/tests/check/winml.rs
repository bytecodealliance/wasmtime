use anyhow::{anyhow, Result};
use windows::AI::MachineLearning::{LearningModelDevice, LearningModelDeviceKind};

/// Return `Ok` if we can use WinML.
pub fn is_available() -> Result<()> {
    match std::panic::catch_unwind(|| {
        println!(
            "> WinML learning device is available: {:?}",
            LearningModelDevice::Create(LearningModelDeviceKind::Default)
        )
    }) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("WinML learning device is not available: {:?}", e)),
    }
}
