//! 线性层 `x · wᵀ (+ b)`：权重 f32（或 Metal 上 f16）原样乘，或量化成 [`Q8Matrix`]。

use candle_core::{DType, Module, Result, Tensor};
use candle_nn::{Linear, VarBuilder};

use super::q8_matrix::Q8Matrix;

pub(super) enum Projection {
    Dense(Linear),

    Quantized {
        matrix: Q8Matrix,

        bias: Option<Tensor>,
    },
}

impl Projection {
    /// 带偏置的线性层，权重名 `weight`、偏置名 `bias`（与 `candle_nn::linear` 一致）。
    pub fn load(vb: VarBuilder, input: usize, output: usize, quantize: bool) -> Result<Self> {
        let weight = vb.get((output, input), "weight")?;
        let bias = vb.get(output, "bias")?;
        Self::new(weight, Some(bias), quantize)
    }

    /// 不带偏置、直接给权重（输出层复用词嵌入）。
    pub fn tied(weight: Tensor, quantize: bool) -> Result<Self> {
        Self::new(weight, None, quantize)
    }

    fn new(weight: Tensor, bias: Option<Tensor>, quantize: bool) -> Result<Self> {
        if quantize {
            let (rows, cols) = weight.dims2()?;
            let values = weight
                .to_dtype(DType::F32)?
                .flatten_all()?
                .to_vec1::<f32>()?;
            if let Some(matrix) = Q8Matrix::quantize(&values, rows, cols) {
                return Ok(Self::Quantized { matrix, bias });
            }
        }
        Ok(Self::Dense(Linear::new(weight, bias)))
    }

    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let (matrix, bias) = match self {
            Self::Dense(linear) => return linear.forward(x),
            Self::Quantized { matrix, bias } => (matrix, bias),
        };
        let mut dims = x.dims().to_vec();
        let m = x.elem_count() / matrix.cols();
        let values = x.flatten_all()?.to_vec1::<f32>()?;
        if let Some(last) = dims.last_mut() {
            *last = matrix.rows();
        }
        let y = Tensor::from_vec(matrix.matmul(&values, m), dims, x.device())?;
        match bias {
            Some(bias) => y.broadcast_add(bias),
            None => Ok(y),
        }
    }
}
