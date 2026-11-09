//! 按 32 个一块量化成 int8 的权重矩阵（与 GGML 的 Q8_0 同一种分块：每块一个缩放 = 块内最大绝对值 / 127），
//! 乘的时候输入留 f32，权重逐行还原成浮点再做 FMA。
//!
//! 不用 candle 的 `QMatMul`：它的 x86 SIMD 按编译期的 `target_feature` 选，缺省目标只有 SSE2，发行构建走标量，比 f32 还慢；
//! 整个程序开 AVX2 又会让没有 AVX 的赛扬 / 奔腾银牌机器起不来。这里运行时探测，有 AVX2 + FMA 才用
//! （[`Q8Matrix::supported`]），否则调用方留 f32。对比数字见 `docs/notes/crate-notes.md` 的 qingjian-neural 一节。

use rayon::prelude::{IndexedParallelIterator, ParallelIterator, ParallelSliceMut};

/// 每块的元素数。
const BLOCK: usize = 32;

/// 并行时每个任务算的输出列数。
const ROWS_PER_TASK: usize = 32;

pub(crate) struct Q8Matrix {
    /// 输出维（原权重的行数）。
    rows: usize,

    /// 输入维，是 [`BLOCK`] 的倍数。
    cols: usize,

    weights: Vec<i8>,

    /// `rows × cols / BLOCK` 个块各自的缩放。
    scales: Vec<f32>,
}

impl Q8Matrix {
    /// 本机能跑快速路径。
    pub fn supported() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma")
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            false
        }
    }

    /// `weight` 是行优先的 `rows × cols`；`cols` 不是 [`BLOCK`] 的倍数时返回 `None`。
    pub fn quantize(weight: &[f32], rows: usize, cols: usize) -> Option<Self> {
        if !cols.is_multiple_of(BLOCK) || weight.len() != rows * cols {
            return None;
        }
        let mut weights = Vec::with_capacity(weight.len());
        let mut scales = Vec::with_capacity(weight.len() / BLOCK);
        for block in weight.chunks_exact(BLOCK) {
            let max = block.iter().fold(0f32, |m, v| m.max(v.abs()));
            let scale = max / 127.0;
            let inverse = if scale > 0.0 { 1.0 / scale } else { 0.0 };
            weights.extend(block.iter().map(|v| (v * inverse).round() as i8));
            scales.push(scale);
        }
        Some(Self {
            rows,
            cols,
            weights,
            scales,
        })
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    /// `x`（行优先 `m × cols`）乘权重的转置，返回行优先的 `m × rows`。
    /// 每行权重先还原成 f32 放进一行的缓冲，`m` 行输入共用它；再按输出列分块并行，算成 `rows × m` 后转回来。
    pub fn matmul(&self, x: &[f32], m: usize) -> Vec<f32> {
        let blocks = self.cols / BLOCK;
        let (dequantize, dot) = kernels();
        let mut transposed = vec![0f32; self.rows * m];
        transposed
            .par_chunks_mut(ROWS_PER_TASK * m)
            .enumerate()
            .for_each(|(task, out)| {
                let mut row_values = vec![0f32; self.cols];
                for (offset, out) in out.chunks_exact_mut(m).enumerate() {
                    let row = task * ROWS_PER_TASK + offset;
                    dequantize(
                        &self.weights[row * self.cols..(row + 1) * self.cols],
                        &self.scales[row * blocks..(row + 1) * blocks],
                        &mut row_values,
                    );
                    for (i, out) in out.iter_mut().enumerate() {
                        *out = dot(&x[i * self.cols..(i + 1) * self.cols], &row_values);
                    }
                }
            });
        let mut result = vec![0f32; m * self.rows];
        for (row, column) in transposed.chunks_exact(m).enumerate() {
            for (i, &v) in column.iter().enumerate() {
                result[i * self.rows + row] = v;
            }
        }
        result
    }
}

/// 一行 int8 权重按块缩放还原成 f32。
type Dequantize = fn(&[i8], &[f32], &mut [f32]);

/// 两段等长、长度是 [`BLOCK`] 倍数的 f32 点积。
type Dot = fn(&[f32], &[f32]) -> f32;

fn kernels() -> (Dequantize, Dot) {
    #[cfg(target_arch = "x86_64")]
    if Q8Matrix::supported() {
        return (avx2::dequantize_checked, avx2::dot_checked);
    }
    (scalar_dequantize, scalar_dot)
}

fn scalar_dequantize(w: &[i8], scales: &[f32], out: &mut [f32]) {
    for ((w, out), &scale) in w
        .chunks_exact(BLOCK)
        .zip(out.chunks_exact_mut(BLOCK))
        .zip(scales)
    {
        for (&w, out) in w.iter().zip(out) {
            *out = f32::from(w) * scale;
        }
    }
}

fn scalar_dot(x: &[f32], w: &[f32]) -> f32 {
    x.iter().zip(w).map(|(a, b)| a * b).sum()
}

#[cfg(target_arch = "x86_64")]
mod avx2 {
    use std::arch::x86_64::{
        __m256, _mm_add_ps, _mm_add_ss, _mm_cvtss_f32, _mm_loadl_epi64, _mm_movehl_ps,
        _mm_shuffle_ps, _mm256_add_ps, _mm256_castps256_ps128, _mm256_cvtepi8_epi32,
        _mm256_cvtepi32_ps, _mm256_extractf128_ps, _mm256_fmadd_ps, _mm256_loadu_ps, _mm256_mul_ps,
        _mm256_set1_ps, _mm256_setzero_ps, _mm256_storeu_ps,
    };

    use super::BLOCK;

    /// 只经 `kernels` 在查过 AVX2 与 FMA 之后取用。
    pub(super) fn dequantize_checked(w: &[i8], scales: &[f32], out: &mut [f32]) {
        // SAFETY：kernels 查过本机支持 AVX2 与 FMA；长度由 matmul 按同一个 cols 切出
        unsafe { dequantize(w, scales, out) }
    }

    /// 只经 `kernels` 在查过 AVX2 与 FMA 之后取用。
    pub(super) fn dot_checked(x: &[f32], w: &[f32]) -> f32 {
        // SAFETY：同上
        unsafe { dot(x, w) }
    }

    #[target_feature(enable = "avx2,fma")]
    unsafe fn dequantize(w: &[i8], scales: &[f32], out: &mut [f32]) {
        for (b, &scale) in scales.iter().enumerate() {
            let scale = _mm256_set1_ps(scale);
            for part in 0..4 {
                let at = b * BLOCK + part * 8;
                // SAFETY：`at + 8 <= w.len() == out.len()`（块数 × BLOCK = cols）
                unsafe {
                    let bytes = _mm_loadl_epi64(w.as_ptr().add(at).cast());
                    let values = _mm256_cvtepi32_ps(_mm256_cvtepi8_epi32(bytes));
                    _mm256_storeu_ps(out.as_mut_ptr().add(at), _mm256_mul_ps(values, scale));
                }
            }
        }
    }

    /// 四路累加器错开 FMA 的延迟。
    #[target_feature(enable = "avx2,fma")]
    unsafe fn dot(x: &[f32], w: &[f32]) -> f32 {
        let mut acc = [_mm256_setzero_ps(); 4];
        for at in (0..x.len()).step_by(BLOCK) {
            for (lane, acc) in acc.iter_mut().enumerate() {
                let at = at + lane * 8;
                // SAFETY：`at + 8 <= x.len() == w.len()`，长度是 BLOCK 的倍数
                unsafe {
                    let a = _mm256_loadu_ps(x.as_ptr().add(at));
                    let b = _mm256_loadu_ps(w.as_ptr().add(at));
                    *acc = _mm256_fmadd_ps(a, b, *acc);
                }
            }
        }
        horizontal_sum(_mm256_add_ps(
            _mm256_add_ps(acc[0], acc[1]),
            _mm256_add_ps(acc[2], acc[3]),
        ))
    }

    #[target_feature(enable = "avx2,fma")]
    fn horizontal_sum(v: __m256) -> f32 {
        let low = _mm256_castps256_ps128(v);
        let high = _mm256_extractf128_ps(v, 1);
        let quad = _mm_add_ps(low, high);
        let pair = _mm_add_ps(quad, _mm_movehl_ps(quad, quad));
        let single = _mm_add_ss(pair, _mm_shuffle_ps(pair, pair, 0b01));
        _mm_cvtss_f32(single)
    }
}

#[cfg(test)]
mod tests {
    use super::{Q8Matrix, scalar_dequantize, scalar_dot};

    /// 量化后的乘积与 f32 相差在量化误差内；快速路径与标量路径一致。
    #[test]
    fn matmul_tracks_the_float_product() {
        let (rows, cols, m) = (70, 96, 3);
        let weight: Vec<f32> = (0..rows * cols)
            .map(|i| ((i * 37 % 101) as f32 - 50.0) / 40.0)
            .collect();
        let x: Vec<f32> = (0..m * cols)
            .map(|i| ((i * 13 % 29) as f32 - 14.0) / 10.0)
            .collect();
        let q = Q8Matrix::quantize(&weight, rows, cols).unwrap();
        let got = q.matmul(&x, m);
        for i in 0..m {
            for j in 0..rows {
                let xi = &x[i * cols..(i + 1) * cols];
                let exact: f32 = xi
                    .iter()
                    .zip(&weight[j * cols..(j + 1) * cols])
                    .map(|(a, b)| a * b)
                    .sum();
                let mut row = vec![0f32; cols];
                scalar_dequantize(
                    &q.weights[j * cols..(j + 1) * cols],
                    &q.scales[j * cols / 32..(j + 1) * cols / 32],
                    &mut row,
                );
                let scalar = scalar_dot(xi, &row);
                let fast = got[i * rows + j];
                // 每个权重最多差半个量化步长（块内最大值 / 254）
                let bound: f32 = xi.iter().map(|a| a.abs() * 1.25 / 254.0).sum();
                assert!((fast - exact).abs() <= bound, "{i},{j}: {fast} vs {exact}");
                assert!((fast - scalar).abs() < 1e-3, "{i},{j}: {fast} vs {scalar}");
            }
        }
        assert!(Q8Matrix::quantize(&weight, rows * 2, cols / 2).is_none());
    }
}
