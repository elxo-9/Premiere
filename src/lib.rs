//! SVG Stroke — Premiere Pro / After Effects 両対応の多重縁取りエフェクト。
//! Premiere では SmartFX が使えないため、通常の PF_Cmd_RENDER + 32f BGRA で実装する。
mod core;

use after_effects as ae;
use ae::*;
use core::{Join, Stroke};

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum P {
    W1, C1, O1,   // 縁取り1（内側）
    W2, C2, O2,   // 縁取り2
    W3, C3, O3,   // 縁取り3（外側）
    Round,        // 角の形状 丸/ミター
}

#[derive(Default)]
struct Plugin;

ae::define_effect!(Plugin, (), P);

impl AdobePluginGlobal for Plugin {
    fn can_load(_host_name: &str, _host_version: &str) -> bool { true }

    fn params_setup(&self, params: &mut Parameters<P>, _in: InData, _out: OutData) -> Result<(), Error> {
        let mut layer = |w: P, c: P, o: P, name: &str, wd: f64, col: (u8, u8, u8), op: f64| -> Result<(), Error> {
            params.add_with_flags(w, &format!("{name} 太さ"), FloatSliderDef::setup(|f| {
                f.set_valid_min(0.0); f.set_valid_max(500.0);
                f.set_slider_min(0.0); f.set_slider_max(100.0);
                f.set_default(wd); f.set_precision(1);
            }), ParamFlag::SUPERVISE, ParamUIFlags::empty())?;
            params.add(c, &format!("{name} 色"), ColorDef::setup(|f| f.set_default(Pixel8 { red: col.0, green: col.1, blue: col.2, alpha: 255 })))?;
            params.add(o, &format!("{name} 不透明度"), FloatSliderDef::setup(|f| {
                f.set_valid_min(0.0); f.set_valid_max(100.0);
                f.set_slider_min(0.0); f.set_slider_max(100.0);
                f.set_default(op); f.set_precision(0);
            }))?;
            Ok(())
        };
        layer(P::W1, P::C1, P::O1, "縁1", 8.0, (255, 255, 255), 100.0)?;
        layer(P::W2, P::C2, P::O2, "縁2", 0.0, (0, 0, 0), 100.0)?;
        layer(P::W3, P::C3, P::O3, "縁3", 0.0, (255, 0, 0), 100.0)?;
        params.add(P::Round, "角を丸くする", CheckBoxDef::setup(|f| { f.set_default(true); f.set_label("丸"); }))?;
        Ok(())
    }

    fn handle_command(&mut self, cmd: Command, in_data: InData, mut out_data: OutData, params: &mut Parameters<P>) -> Result<(), Error> {
        match cmd {
            Command::GlobalSetup => {
                // Premiere に「32bit float BGRA で受け取る」と宣言する
                if in_data.is_premiere() {
                    let pf = PixelFormatSuite::new()?;
                    pf.clear_supported_pixel_formats(in_data.effect_ref())?;
                    pf.add_supported_pixel_format(in_data.effect_ref(), pr::PixelFormat::Bgra4444_32f)?;
                }
            }
            Command::FrameSetup { .. } => {
                // 縁取りが広がる分だけ出力バッファを拡張
                let grow = [P::W1, P::W2, P::W3].iter()
                    .map(|&p| params.get(p, in_data.current_time(), in_data.time_step(), in_data.time_scale()).ok()
                        .and_then(|x| x.as_float_slider().ok()).map(|s| s.value()).unwrap_or(0.0))
                    .fold(0.0f64, f64::max).ceil() as i32 + 2;
                out_data.set_width(in_data.width() + grow * 2);
                out_data.set_height(in_data.height() + grow * 2);
                out_data.set_origin(Point { h: grow, v: grow });
            }
            Command::Render { in_layer, mut out_layer } => {
                let t = (in_data.current_time(), in_data.time_step(), in_data.time_scale());
                let get_f = |p: P| params.get(p, t.0, Some(t.1), Some(t.2)).ok().and_then(|x| x.as_float_slider().ok()).map(|s| s.value() as f32).unwrap_or(0.0);
                let get_c = |p: P| params.get(p, t.0, Some(t.1), Some(t.2)).ok().and_then(|x| x.as_color().ok()).map(|c| c.value()).unwrap();
                let join = if params.get(P::Round, t.0, Some(t.1), Some(t.2))?.as_checkbox()?.value() { Join::Round } else { Join::Miter };
                let mk = |w: P, c: P, o: P| { let c = get_c(c); Stroke {
                    width: get_f(w), opacity: get_f(o) / 100.0, join,
                    color: [c.red as f32 / 255.0, c.green as f32 / 255.0, c.blue as f32 / 255.0] } };
                let strokes: Vec<Stroke> = [mk(P::W1, P::C1, P::O1), mk(P::W2, P::C2, P::O2), mk(P::W3, P::C3, P::O3)]
                    .into_iter().filter(|s| s.width > 0.0 && s.opacity > 0.0).collect();

                // 入力(中央配置)を出力サイズのバッファに読み込み → 計算 → 書き戻し
                let (ow, oh) = (out_layer.width(), out_layer.height());
                let (iw, ih) = (in_layer.width(), in_layer.height());
                let (dx, dy) = ((ow - iw) / 2, (oh - ih) / 2);
                let mut buf = vec![0.0f32; ow * oh * 4];
                for y in 0..ih { for x in 0..iw {
                    let p = in_layer.as_pixel32(x, y); // Premiere の 32f は BGRA 並び
                    let i = ((y + dy) * ow + x + dx) * 4;
                    // Premiere の 32f は乗算済み(premultiplied) → straight に戻す
                    let a = p.alpha;
                    let inv = if a > 1e-6 { 1.0 / a } else { 0.0 };
                    buf[i] = p.red * inv; buf[i + 1] = p.green * inv; buf[i + 2] = p.blue * inv; buf[i + 3] = a;
                }}
                let res = core::apply(&buf, ow, oh, &strokes);
                for y in 0..oh { for x in 0..ow {
                    let i = (y * ow + x) * 4;
                    let a = res[i + 3];
                    let o = out_layer.as_pixel32_mut(x, y);
                    o.red = res[i] * a; o.green = res[i + 1] * a; o.blue = res[i + 2] * a; o.alpha = a;
                }}
            }
            _ => {}
        }
        Ok(())
    }
}
