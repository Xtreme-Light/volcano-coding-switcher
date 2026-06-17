//! 一次性脚本：从 icon.png 生成 Windows 的 icon.ico。
//! 运行：cargo run --manifest-path scripts/gen-ico/Cargo.toml

use std::fs;
use std::io::Cursor;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::ExtendedColorType;

fn main() {
    let png_path = "src-tauri/icons/icon.png";
    let ico_path = "src-tauri/icons/icon.ico";

    let img = image::open(png_path).expect("读取 icon.png 失败");
    let rgba = img.to_rgba8();

    // 生成 ICO：包含多个尺寸，Windows 会按需选用。
    let sizes: [u32; 6] = [16, 32, 48, 64, 128, 256];
    let mut frames: Vec<IcoFrame<'_>> = Vec::new();
    for &s in &sizes {
        let resized = image::imageops::resize(
            &rgba,
            s,
            s,
            image::imageops::FilterType::Lanczos3,
        );
        // IcoFrame::as_png 接收原始 RGBA 像素数据，内部编码为 PNG 后封装进 ICO 容器。
        frames.push(
            IcoFrame::as_png(&resized, s, s, ExtendedColorType::Rgba8)
                .expect("构造 IcoFrame 失败"),
        );
    }

    let mut buf = Cursor::new(Vec::new());
    let encoder = IcoEncoder::new(&mut buf);
    encoder
        .encode_images(&frames)
        .expect("编码 ICO 失败");
    fs::write(ico_path, buf.into_inner()).expect("写入 icon.ico 失败");
    println!("✅ 已生成 {}", ico_path);
}

