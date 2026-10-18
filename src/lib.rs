//! SVG Stroke — Premiere Pro 用の多重縁取りエフェクト（32bit float BGRA）。
#[path = "core.rs"]
mod stroke;

use after_effects as ae;
use ae::*;
use stroke::{Join, Stroke};

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum Params {
    W1, C1, O1,
    W2, C2, O2,
    W3, C3, O3,
    Round,
}

#[derive(Default)]
struct Plugin;

ae::define_effect!(Plugin, (), Params);

fn add_stroke_params(
    params: &mut Parameters<Params>,
    w: Params, c: Params, o: Params,
    name: &str, width: f64, col: (u8, u8, u8),
) -> Result<(), Error> {
    params.add(w, &format!("{name} 太さ"), FloatSliderDef::setup(|f| {
        f.set_valid_min(0.0);
        f.set_valid_max(500.0);
        f.set_slider_min(0.0);
        f.set_slider_max(100.0);
        f.set_default(width);
        f.set_precision(1);
    }))?;
    params.add(c, &format!("{name} 色"), ColorDef::setup(|f| {
        f.set_default(Pixel8 { alpha: 255, red: col.0, green: col.1, blue: col.2 });
    }))?;
    params.add(o, &format!("{name} 不透明度"), FloatSliderDef::setup(|f| {
        f.set_valid_min(0.0);
        f.set_valid_max(100.0);
        f.set_slider_min(0.0);
        f.set_slider_max(100.0);
        f.set_default(100.0);
        f.set_precision(0);
    }))?;
    Ok(())
}

fn get_f(params: &Parameters<Params>, p: Params) -> f32 {
    params.get(p).ok()
        .and_then(|x| x.as_float_slider().ok().map(|s| s.value() as f32))
        .unwrap_or(0.0)
}

fn get_c(params: &Parameters<Params>, p: Params) -> [f32; 3] {
    params.get(p).ok()
        .and_then(|x| x.as_color().ok().map(|c| {
            let v = c.value();
            [v.red as f32 / 255.0, v.green as f32 / 255.0, v.blue as f32 / 255.0]
        }))
        .unwrap_or([0.0; 3])
}

impl AdobePluginGlobal for Plugin {
    fn can_load(_host_name: &str, _host_version: &str) -> bool {
        true
    }

    fn params_setup(&self, params: &mut Parameters<Params>, _in_data: InData, _out_data: OutData) -> Result<(), Error> {
        add_stroke_params(params, Params::W1, Params::C1, Params::O1, "縁1", 8.0, (255, 255, 255))?;
        add_stroke_params(params, Params::W2, Params::C2, Params::O2, "縁2", 0.0, (0, 0, 0))?;
        add_stroke_params(params, Params::W3, Params::C3, Params::O3, "縁3", 0.0, (255, 0, 0))?;
        params.add(Params::Round, "角を丸くする", CheckBoxDef::setup(|f| {
            f.set_default(true);
            f.set_label("丸");
        }))?;
        Ok(())
    }

    fn handle_command(&mut self, command: Command, in_data: InData, _out_data: OutData, params: &mut Parameters<Params>) -> Result<(), Error> {
        match command {
            Command::GlobalSetup => {
                // Premiere に「32bit float BGRA で受け取る」と宣言する
                if in_data.is_premiere() {
                    let pf = premiere::suites::PixelFormat::new()?;
                    pf.clear_supported_pixel_formats(in_data.effect_ref())?;
                    pf.add_supported_pixel_format(in_data.effect_ref(), ae::pr::PixelFormat::Bgra4444_32f)?;
                }
            }
            Command::Render { in_layer, mut out_layer } => {
                if !in_data.is_premiere() {
                    return Err(Error::BadCallbackParameter);
                }
                let join = if params.get(Params::Round)?.as_checkbox()?.value() { Join::Round } else { Join::Miter };
                let mk = |w: Params, c: Params, o: Params| Stroke {
                    width: get_f(params, w),
                    opacity: get_f(params, o) / 100.0,
                    color: get_c(params, c),
                    join,
                };
                let strokes: Vec<Stroke> = [
                    mk(Params::W1, Params::C1, Params::O1),
                    mk(Params::W2, Params::C2, Params::O2),
                    mk(Params::W3, Params::C3, Params::O3),
                ]
                .into_iter()
                .filter(|s| s.width > 0.0 && s.opacity > 0.0)
                .collect();

                let w = in_layer.width().min(out_layer.width());
                let h = in_layer.height().min(out_layer.height());
                let in_stride = in_layer.row_bytes();
                let out_stride = out_layer.row_bytes();
                let in_ptr = unsafe { in_layer.data_ptr_mut() } as *const u8;
                let out_ptr = unsafe { out_layer.data_ptr_mut() };

                // Premiere の 32f BGRA はメモリ上 B,G,R,A の順・乗算済みアルファ。
                // 計算は straight(非乗算) RGBA で行う。
                let mut buf = vec![0.0f32; w * h * 4];
                for y in 0..h {
                    let row = unsafe { in_ptr.offset(y as isize * in_stride) } as *const f32;
                    for x in 0..w {
                        let (b, g, r, a) = unsafe { (*row.add(x * 4), *row.add(x * 4 + 1), *row.add(x * 4 + 2), *row.add(x * 4 + 3)) };
                        let inv = if a > 1e-6 { 1.0 / a } else { 0.0 };
                        let i = (y * w + x) * 4;
                        buf[i] = r * inv;
                        buf[i + 1] = g * inv;
                        buf[i + 2] = b * inv;
                        buf[i + 3] = a;
                    }
                }

                let res = stroke::apply(&buf, w, h, &strokes);

                for y in 0..h {
                    let row = unsafe { out_ptr.offset(y as isize * out_stride) } as *mut f32;
                    for x in 0..w {
                        let i = (y * w + x) * 4;
                        let a = res[i + 3];
                        unsafe {
                            *row.add(x * 4) = res[i + 2] * a;
                            *row.add(x * 4 + 1) = res[i + 1] * a;
                            *row.add(x * 4 + 2) = res[i] * a;
                            *row.add(x * 4 + 3) = a;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
