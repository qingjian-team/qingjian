//! 本地模型跑在哪：缺省 CPU，开了 `metal` feature 且系统支持时用 Apple GPU。含章与知意共用。

use candle_core::Device;

use crate::NeuralError;

#[cfg(not(feature = "metal"))]
pub(crate) fn default_device() -> Result<Device, NeuralError> {
    Ok(Device::Cpu)
}

#[cfg(feature = "metal")]
pub(crate) fn default_device() -> Result<Device, NeuralError> {
    // candle-metal 在创建设备时会使用此类；旧版 macOS 缺少它会直接 panic。
    if objc2::runtime::AnyClass::get(c"MTLResidencySetDescriptor").is_none() {
        tracing::warn!("系统不支持 Metal residency set，本地模型改用 CPU");
        return Ok(Device::Cpu);
    }
    Ok(Device::new_metal(0)?)
}
