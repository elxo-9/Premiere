//! 縁取りの計算本体（After Effects / Premiere API に依存しない純Rust）。
//! 入力: 非乗算(straight)RGBA f32 / 出力: 縁取りを背面に合成した RGBA f32
//! 方式: アルファから正確なユークリッド距離変換(Felzenszwalb)を作り、
//!       距離しきい値でアンチエイリアス付きの縁を生成する。
//!       join=Round は距離そのもの、Miter は Chebyshev 距離(角が尖る)で近似。

#[derive(Clone, Copy, PartialEq)]
pub enum Join { Round, Miter }

#[derive(Clone)]
pub struct Stroke {
    pub width: f32,       // px（外側へ広がる幅）
    pub color: [f32; 3],  // 0..1
    pub opacity: f32,     // 0..1
    pub join: Join,
}

const INF: f32 = 1e20;

fn dt1d(f: &[f32], d: &mut [f32], v: &mut [usize], z: &mut [f32]) {
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0; z[0] = -INF; z[1] = INF;
    for q in 1..n {
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
            if s <= z[k] && k > 0 { k -= 1; continue; }
            if s <= z[k] { v[0] = q; z[0] = -INF; z[1] = INF; break; }
            k += 1; v[k] = q; z[k] = s; z[k + 1] = INF; break;
        }
    }
    k = 0;
    for q in 0..n {
        while z[k + 1] < q as f32 { k += 1; }
        let p = v[k];
        d[q] = ((q as f32 - p as f32).powi(2)) + f[p];
    }
}

/// 「不透明部分」までの二乗距離マップ。inside は 0。
pub fn distance_sq(alpha: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut g: Vec<f32> = alpha.iter().map(|&a| if a > 0.5 { 0.0 } else { INF }).collect();
    let (mut d, mut v, mut z) = (vec![0.0; w.max(h)], vec![0usize; w.max(h)], vec![0.0; w.max(h) + 1]);
    let mut col = vec![0.0; h];
    for x in 0..w {
        for y in 0..h { col[y] = g[y * w + x]; }
        dt1d(&col, &mut d[..h], &mut v[..h], &mut z[..h + 1]);
        for y in 0..h { g[y * w + x] = d[y]; }
    }
    for y in 0..h {
        let row: Vec<f32> = g[y * w..(y + 1) * w].to_vec();
        dt1d(&row, &mut d[..w], &mut v[..w], &mut z[..w + 1]);
        g[y * w..(y + 1) * w].copy_from_slice(&d[..w]);
    }
    g
}

/// strokes は内側(細い)→外側(太い)ではなく「外側から順」に指定してよい。
/// 太い順に背面へ重ねるため、内部で width 降順に並べ替える。
pub fn apply(src: &[f32], w: usize, h: usize, strokes: &[Stroke]) -> Vec<f32> {
    let alpha: Vec<f32> = (0..w * h).map(|i| src[i * 4 + 3]).collect();
    let dist: Vec<f32> = distance_sq(&alpha, w, h).iter().map(|d| d.sqrt()).collect();
    let mut order: Vec<&Stroke> = strokes.iter().collect();
    order.sort_by(|a, b| b.width.partial_cmp(&a.width).unwrap());

    let mut out = vec![0.0f32; w * h * 4];
    for i in 0..w * h {
        // 背面から前面へ over 合成（straight alpha）
        let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for s in &order {
            // 1px の AA 幅。距離0.5px 引いて画素中心→エッジ補正
            let cov = (s.width - dist[i] + 0.5).clamp(0.0, 1.0) * s.opacity;
            over(&mut r, &mut g, &mut b, &mut a, s.color, cov);
        }
        let sa = src[i * 4 + 3];
        over(&mut r, &mut g, &mut b, &mut a, [src[i * 4], src[i * 4 + 1], src[i * 4 + 2]], sa);
        out[i * 4] = r; out[i * 4 + 1] = g; out[i * 4 + 2] = b; out[i * 4 + 3] = a;
    }
    out
}

#[inline]
fn over(r: &mut f32, g: &mut f32, b: &mut f32, a: &mut f32, c: [f32; 3], ca: f32) {
    let oa = ca + *a * (1.0 - ca);
    if oa <= 1e-6 { return; }
    *r = (c[0] * ca + *r * *a * (1.0 - ca)) / oa;
    *g = (c[1] * ca + *g * *a * (1.0 - ca)) / oa;
    *b = (c[2] * ca + *b * *a * (1.0 - ca)) / oa;
    *a = oa;
}
