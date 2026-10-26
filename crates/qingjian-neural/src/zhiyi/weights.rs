use std::path::Path;

use candle_core::quantized::{GgmlDType, QMatMul, QTensor};
use candle_core::safetensors::MmapedSafetensors;
use candle_core::{DType, Device, Module, Result, Tensor};

/// safetensors 文件 mmap 着，张量按名一个个取出来；线性层取的时候量化成 Q8_0（模型说明要求这么跑才是它量过的准确率）。
/// 不整份读进内存：文件 155MB，整份读再逐个转换，加载那一下的峰值要多出好几百 MB。
pub(super) struct Weights {
    file: MmapedSafetensors,
    device: Device,
}

impl Weights {
    pub fn load(path: &Path, device: &Device) -> Result<Self> {
        // SAFETY：mmap 的权重文件在加载期间不改动；张量都拷到设备上，加载完就解除映射
        let file = unsafe { MmapedSafetensors::new(path)? };
        Ok(Self {
            file,
            device: device.clone(),
        })
    }

    /// 按 f32 取（范数、偏置这些不量化的）。
    pub fn get(&self, name: &str) -> Result<Tensor> {
        self.raw(name)?.to_dtype(DType::F32)
    }

    /// 按文件里存的类型取：词嵌入留 F16（11 万行 × 384，转 F32 要多占 80MB，每次只查几十行）。
    pub fn raw(&self, name: &str) -> Result<Tensor> {
        self.file.load(name, &self.device)
    }

    /// 读到 CPU 上的整数表（Metal 不支持 I32 转 U32）。
    pub fn table(&self, name: &str) -> Result<Vec<u32>> {
        self.file
            .load(name, &Device::Cpu)?
            .to_dtype(DType::U32)?
            .to_vec1()
    }

    /// 线性层，权重量化成 Q8_0；`bias` 是偏置名，没有偏置给 `None`。
    pub fn linear(&self, weight: &str, bias: Option<&str>) -> Result<QLinear> {
        let w = self.get(weight)?;
        Ok(QLinear {
            weight: QMatMul::from_qtensor(QTensor::quantize(&w, GgmlDType::Q8_0)?)?,
            bias: bias.map(|name| self.get(name)).transpose()?,
        })
    }
}

/// `x · wᵀ (+ b)`，`w` 是 Q8_0。
pub(super) struct QLinear {
    weight: QMatMul,
    bias: Option<Tensor>,
}

impl QLinear {
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let y = self.weight.forward(x)?;
        match &self.bias {
            Some(bias) => y.broadcast_add(bias),
            None => Ok(y),
        }
    }
}

/// LayerNorm（最后一维），`bias` 可以没有（编码器里的都没有）。
pub(super) fn layer_norm(x: &Tensor, weight: &Tensor, bias: Option<&Tensor>) -> Result<Tensor> {
    let mean = x.mean_keepdim(candle_core::D::Minus1)?;
    let centered = x.broadcast_sub(&mean)?;
    let var = centered.sqr()?.mean_keepdim(candle_core::D::Minus1)?;
    let y = centered
        .broadcast_div(&(var + 1e-5)?.sqrt()?)?
        .broadcast_mul(weight)?;
    match bias {
        Some(bias) => y.broadcast_add(bias),
        None => Ok(y),
    }
}
