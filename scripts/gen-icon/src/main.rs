//! 一次性图标生成脚本（不进入正式构建）。
//! 用法：cargo run --manifest-path scripts/gen-icon/Cargo.toml
//! 输出：src-tauri/icons/icon.png

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

const SIZE: u32 = 256;

fn main() {
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];

    let cx = SIZE as f32 / 2.0;
    let cy = SIZE as f32 / 2.0;
    let radius = 48.0_f32; // 圆角半径
    let half = SIZE as f32 / 2.0 - 8.0; // 边距

    for y in 0..SIZE {
        for x in 0..SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            // 圆角矩形 SDF
            let dx = (fx - cx).abs() - (half - radius);
            let dy = (fy - cy).abs() - (half - radius);
            let outside = dx.max(0.0).hypot(dy.max(0.0)) + dx.min(0.0).max(dy.min(0.0)) - radius;
            let alpha = (1.0 - outside).clamp(0.0, 1.0);

            // 渐变背景：左上 #ff8a3d -> 右下 #ff3d6a
            let t = ((x + y) as f32) / ((SIZE * 2) as f32);
            let r = lerp(0xff, 0xff, t);
            let g = lerp(0x8a, 0x3d, t);
            let b = lerp(0x3d, 0x6a, t);

            // 白色 V（双笔画构成）：两条对角粗线
            let v_alpha = v_mark(fx, fy);

            let (rr, gg, bb) = mix(r, g, b, 0xff, 0xff, 0xff, v_alpha);

            let idx = ((y * SIZE + x) * 4) as usize;
            pixels[idx] = rr;
            pixels[idx + 1] = gg;
            pixels[idx + 2] = bb;
            pixels[idx + 3] = (alpha * 255.0) as u8;
        }
    }

    let out = PathBuf::from("src-tauri/icons/icon.png");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let file = File::create(&out).expect("create png");
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, SIZE, SIZE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("write header");
    writer.write_image_data(&pixels).expect("write data");
    println!("已生成: {}", out.display());
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (a as f32 * (1.0 - t) + b as f32 * t) as u8
}

fn mix(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8, t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (r1 as f32 * (1.0 - t) + r2 as f32 * t) as u8,
        (g1 as f32 * (1.0 - t) + g2 as f32 * t) as u8,
        (b1 as f32 * (1.0 - t) + b2 as f32 * t) as u8,
    )
}

/// 在中心绘制白色 V 字，由两条粗线段抗锯齿合成。
fn v_mark(x: f32, y: f32) -> f32 {
    let cx = SIZE as f32 / 2.0;
    let cy = SIZE as f32 / 2.0;

    // V 的三个端点：左上、底中、右上
    let p_left = (cx - 64.0, cy - 56.0);
    let p_bottom = (cx, cy + 64.0);
    let p_right = (cx + 64.0, cy - 56.0);

    let thickness = 22.0;
    let a = segment_alpha(x, y, p_left, p_bottom, thickness);
    let b = segment_alpha(x, y, p_bottom, p_right, thickness);
    a.max(b)
}

fn segment_alpha(x: f32, y: f32, a: (f32, f32), b: (f32, f32), thickness: f32) -> f32 {
    let abx = b.0 - a.0;
    let aby = b.1 - a.1;
    let apx = x - a.0;
    let apy = y - a.1;
    let len_sq = abx * abx + aby * aby;
    let t = ((apx * abx + apy * aby) / len_sq).clamp(0.0, 1.0);
    let px = a.0 + t * abx;
    let py = a.1 + t * aby;
    let d = (x - px).hypot(y - py);
    let half = thickness * 0.5;
    (1.0 - (d - half + 0.5)).clamp(0.0, 1.0)
}
