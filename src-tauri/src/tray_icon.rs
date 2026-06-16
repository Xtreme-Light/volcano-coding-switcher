//! 运行时根据用量比率生成托盘图标（圆形 + 进度环）。
//! ratio: 0~1（>1 视为 1），<=0.8 绿色，0.8~1 橙色，>=1 红色。

use image::{Rgba, RgbaImage};
use tauri::image::Image;

const SIZE: u32 = 64;
const CENTER: f32 = SIZE as f32 / 2.0;
const RADIUS: f32 = 28.0;
const RING_THICKNESS: f32 = 6.0;

#[derive(Clone, Copy)]
struct Color([u8; 4]);

const TRANSPARENT: Color = Color([0, 0, 0, 0]);
const BG_RING: Color = Color([60, 64, 72, 255]);

fn status_color(ratio: f32) -> Color {
    if ratio >= 1.0 {
        Color([0xFF, 0x4D, 0x4F, 0xFF]) // 红
    } else if ratio > 0.8 {
        Color([0xFF, 0xA9, 0x40, 0xFF]) // 橙
    } else {
        Color([0x52, 0xC4, 0x1A, 0xFF]) // 绿
    }
}

/// 生成 PNG bytes（RGBA8）。
pub fn make_tray_image(ratio: f32) -> Image<'static> {
    let r = ratio.clamp(0.0, 1.0);
    let mut img = RgbaImage::from_pixel(SIZE, SIZE, Rgba(TRANSPARENT.0));

    let outer = RADIUS;
    let inner = outer - RING_THICKNESS;

    // 弧线起点：12 点方向（-PI/2），顺时针绘制 r * 2PI 弧度
    let start_angle = -std::f32::consts::FRAC_PI_2;
    let end_angle = start_angle + std::f32::consts::TAU * r;

    let active = status_color(r);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 + 0.5 - CENTER;
            let dy = y as f32 + 0.5 - CENTER;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist >= inner && dist <= outer {
                // 落在环上
                // 计算角度（-PI..PI），换算到 [start_angle, start_angle+2PI)
                let angle = dy.atan2(dx);
                let mut a = angle;
                while a < start_angle {
                    a += std::f32::consts::TAU;
                }
                let color = if a <= end_angle { active } else { BG_RING };
                img.put_pixel(x, y, Rgba(color.0));
            } else if dist < inner - 1.0 {
                // 内部填一层很淡的底色，让图标更明显
                img.put_pixel(x, y, Rgba([24, 27, 34, 255]));
            }
        }
    }

    let raw = img.into_raw();
    Image::new_owned(raw, SIZE, SIZE)
}
