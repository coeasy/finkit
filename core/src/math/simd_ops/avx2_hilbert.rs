//! AVX2 kernels for the Hilbert-transform cycle indicators.

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn ht_smooth_avx2(input: &[f64], out: &mut [f64], len: usize) {
    use core::arch::x86_64::*;
    let w4 = _mm256_set1_pd(4.0);
    let w3 = _mm256_set1_pd(3.0);
    let w2 = _mm256_set1_pd(2.0);
    let w1 = _mm256_set1_pd(1.0);
    let scale = _mm256_set1_pd(0.1);

    // 处理 4 的整数倍，剩余尾部走标量
    let chunks = (len - 3) / 4;
    for c in 0..chunks {
        let i = 3 + c * 4;
        // 加载 4 个连续 bar 的 4 个滞后值
        let x0 = _mm256_loadu_pd(input.as_ptr().add(i));
        let x1 = _mm256_loadu_pd(input.as_ptr().add(i - 1));
        let x2 = _mm256_loadu_pd(input.as_ptr().add(i - 2));
        let x3 = _mm256_loadu_pd(input.as_ptr().add(i - 3));
        // 累加：4*x0 + 3*x1 + 2*x2 + 1*x3
        let mut acc = _mm256_mul_pd(w4, x0);
        acc = _mm256_fmadd_pd(w3, x1, acc);
        acc = _mm256_fmadd_pd(w2, x2, acc);
        acc = _mm256_fmadd_pd(w1, x3, acc);
        acc = _mm256_mul_pd(scale, acc);
        _mm256_storeu_pd(out.as_mut_ptr().add(i), acc);
    }
    // 尾部
    let tail_start = 3 + chunks * 4;
    for i in tail_start..len {
        out[i] = 0.1 * (4.0 * input[i] + 3.0 * input[i - 1] + 2.0 * input[i - 2] + input[i - 3]);
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn ht_detrender_avx2(smooth: &[f64], out: &mut [f64], len: usize) {
    use core::arch::x86_64::*;
    // 系数
    let c_a_pos1 = _mm256_set1_pd(0.0962);
    let c_a_pos2 = _mm256_set1_pd(0.5769);
    let c_a_neg2 = _mm256_set1_pd(-0.5769);
    let c_a_neg1 = _mm256_set1_pd(-0.0962);
    let c_b_p1 = _mm256_set1_pd(0.075);
    let c_b_p3 = _mm256_set1_pd(0.54);
    let c_b_p5 = _mm256_set1_pd(0.075);

    let chunks = (len - 10) / 4;
    for c in 0..chunks {
        let i = 10 + c * 4;
        // 加载 4 个连续 bar 所需的 7 个滞后值
        let s0 = _mm256_loadu_pd(smooth.as_ptr().add(i));
        let s1 = _mm256_loadu_pd(smooth.as_ptr().add(i - 1));
        let s2 = _mm256_loadu_pd(smooth.as_ptr().add(i - 2));
        let s3 = _mm256_loadu_pd(smooth.as_ptr().add(i - 3));
        let s4 = _mm256_loadu_pd(smooth.as_ptr().add(i - 4));
        let s5 = _mm256_loadu_pd(smooth.as_ptr().add(i - 5));
        let s6 = _mm256_loadu_pd(smooth.as_ptr().add(i - 6));

        // a = 0.0962*s[i] + 0.5769*s[i-2] - 0.5769*s[i-4] - 0.0962*s[i-6]
        let mut a = _mm256_mul_pd(c_a_pos1, s0);
        a = _mm256_fmadd_pd(c_a_pos2, s2, a);
        a = _mm256_fmadd_pd(c_a_neg2, s4, a);
        a = _mm256_fmadd_pd(c_a_neg1, s6, a);

        // b = 0.075*s[i-1] + 0.54*s[i-3] + 0.075*s[i-5]
        let mut b = _mm256_mul_pd(c_b_p1, s1);
        b = _mm256_fmadd_pd(c_b_p3, s3, b);
        b = _mm256_fmadd_pd(c_b_p5, s5, b);

        // detrender = a * b
        let d = _mm256_mul_pd(a, b);
        _mm256_storeu_pd(out.as_mut_ptr().add(i), d);
    }
    let tail_start = 10 + chunks * 4;
    for i in tail_start..len {
        let a = 0.0962 * smooth[i] + 0.5769 * smooth[i - 2]
            - 0.5769 * smooth[i - 4]
            - 0.0962 * smooth[i - 6];
        let b = 0.075 * smooth[i - 1] + 0.54 * smooth[i - 3] + 0.075 * smooth[i - 5];
        out[i] = a * b;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn ht_components_avx2(detrender: &[f64], phase_out: &mut [f64], len: usize) {
    use core::arch::x86_64::*;
    // 系数
    let c_pos1 = _mm256_set1_pd(0.0962);
    let c_pos2 = _mm256_set1_pd(0.5769);
    let c_neg2 = _mm256_set1_pd(-0.5769);
    let c_neg1 = _mm256_set1_pd(-0.0962);

    let chunks = (len - 16) / 4;
    for c in 0..chunks {
        let i = 16 + c * 4;
        // 加载 4 个连续 bar 的所有所需滞后值
        // d0=d[i], d1=d[i-1], d2=d[i-2], d3=d[i-3], d4=d[i-4], d5=d[i-5]
        // d6=d[i-6], d7=d[i-7], d8=d[i-8], d9=d[i-9], d10=d[i-10], d11=d[i-11], d12=d[i-12]
        let d0 = _mm256_loadu_pd(detrender.as_ptr().add(i));
        let d2 = _mm256_loadu_pd(detrender.as_ptr().add(i - 2));
        let d4 = _mm256_loadu_pd(detrender.as_ptr().add(i - 4));
        let d6 = _mm256_loadu_pd(detrender.as_ptr().add(i - 6));
        let d8 = _mm256_loadu_pd(detrender.as_ptr().add(i - 8));
        let d10 = _mm256_loadu_pd(detrender.as_ptr().add(i - 10));
        let d12 = _mm256_loadu_pd(detrender.as_ptr().add(i - 12));

        // in_phase[i] = detrender[i-6]
        let ip = d6;

        // quadrature[i] = 0.0962*d[i] + 0.5769*d[i-2] - 0.5769*d[i-4] - 0.0962*d[i-6]
        let mut q = _mm256_mul_pd(c_pos1, d0);
        q = _mm256_fmadd_pd(c_pos2, d2, q);
        q = _mm256_fmadd_pd(c_neg2, d4, q);
        q = _mm256_fmadd_pd(c_neg1, d6, q);

        // j1[i] = 0.0962*ip[i] + 0.5769*ip[i-2] - 0.5769*ip[i-4] - 0.0962*ip[i-6]
        //        = 0.0962*d[i-6] + 0.5769*d[i-8] - 0.5769*d[i-10] - 0.0962*d[i-12]
        let mut j1 = _mm256_mul_pd(c_pos1, d6);
        j1 = _mm256_fmadd_pd(c_pos2, d8, j1);
        j1 = _mm256_fmadd_pd(c_neg2, d10, j1);
        j1 = _mm256_fmadd_pd(c_neg1, d12, j1);

        // i2 = ip - j1
        let i2 = _mm256_sub_pd(ip, j1);
        // j2 = q + ip
        let j2 = _mm256_add_pd(q, ip);
        // re = i2*ip + j2*q
        let mut re = _mm256_mul_pd(i2, ip);
        re = _mm256_fmadd_pd(j2, q, re);
        // im = i2*q - j2*ip
        let mut im = _mm256_mul_pd(i2, q);
        im = _mm256_fnmadd_pd(j2, ip, im);

        // 逐 bar 算 atan2（无法 SIMD 化）
        let mut re_arr = [0.0f64; 4];
        let mut im_arr = [0.0f64; 4];
        _mm256_storeu_pd(re_arr.as_mut_ptr(), re);
        _mm256_storeu_pd(im_arr.as_mut_ptr(), im);
        for k in 0..4 {
            let re_v = re_arr[k];
            let im_v = im_arr[k];
            phase_out[i + k] = if re_v.abs() > 1e-10 {
                im_v.atan2(re_v)
            } else {
                0.0
            };
        }
    }
    // 尾部
    let tail_start = 16 + chunks * 4;
    for i in tail_start..len {
        let ip = detrender[i - 6];
        let q = 0.0962 * detrender[i] + 0.5769 * detrender[i - 2]
            - 0.5769 * detrender[i - 4]
            - 0.0962 * detrender[i - 6];
        let j1 = 0.0962 * detrender[i - 6] + 0.5769 * detrender[i - 8]
            - 0.5769 * detrender[i - 10]
            - 0.0962 * detrender[i - 12];
        let i2 = ip - j1;
        let j2 = q + ip;
        let re = i2 * ip + j2 * q;
        let im = i2 * q - j2 * ip;
        phase_out[i] = if re.abs() > 1e-10 { im.atan2(re) } else { 0.0 };
    }
}
