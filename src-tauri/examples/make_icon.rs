//! DiskBytes brand icon generator (doc 03 M11.1): draws the 1024×1024
//! source PNG programmatically (no design tool dependency) — a rounded
//! ink square with a storage-ring mark (the "disk" half of DiskBytes).
//! `scripts/make_icon.ps1` runs this, then `npx tauri icon` fans the
//! result out to every platform size.
//!
//! Run: `cargo run --example make_icon` (writes
//! `../packaging/icon/icon-source.png` next to the app root).

use std::f32::consts::PI;
use std::path::Path;

fn main() -> Result<(), String> {
    const SIZE: u32 = 1024;
    // Brand constants (spec §14 light palette: ink #FF6B4A; paper #FFF).
    const INK: [u8; 3] = [0xFF, 0x6B, 0x4A];
    const PAPER: [u8; 3] = [0xFF, 0xFF, 0xFF];
    const INK_DEEP: [u8; 3] = [0xD4, 0x50, 0x2F];
    // Supersample 3×3 per pixel for smooth edges (distance fields give
    // the geometry; the sampling gives the anti-aliasing).
    const SS: u32 = 3;

    let mut rgba = vec![0u8; (SIZE as usize) * (SIZE as usize) * 4];
    let s = SIZE as f32;
    let center = s / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut ink_a = 0u32; // rounded-square coverage
            let mut ring_a = 0u32;
            let mut sector_a = 0u32;
            let mut dot_a = 0u32;
            for dy in 0..SS {
                for dx in 0..SS {
                    let px = x as f32 + (dx as f32 + 0.5) / SS as f32;
                    let py = y as f32 + (dy as f32 + 0.5) / SS as f32;
                    // Rounded square: inside when |p-c|∞ < half AND the
                    // corner distance < radius (squared rounded rect).
                    let half = s * 0.5 - 8.0;
                    let radius = s * 0.164;
                    let qx = (px - center).abs();
                    let qy = (py - center).abs();
                    let in_square = if qx.max(qy) < half - radius {
                        true
                    } else {
                        let cx = (qx - (half - radius)).max(0.0);
                        let cy = (qy - (half - radius)).max(0.0);
                        cx * cx + cy * cy < radius * radius
                    };
                    if in_square {
                        ink_a += 1;
                        // Ring: |dist - R| < stroke/2 (the storage gauge).
                        let dist = ((px - center).powi(2) + (py - center).powi(2)).sqrt();
                        if (dist - s * 0.30).abs() < s * 0.044 {
                            ring_a += 1;
                        }
                        // Sector: a 104° wedge in the ring's upper right —
                        // the "used" part of the gauge.
                        let ang = ((py - center).atan2(px - center) + 2.0 * PI) % (2.0 * PI);
                        let in_sector_angle = ang < 1.82;
                        let in_sector_band = (dist > s * 0.255) && (dist < s * 0.345);
                        if in_sector_angle && in_sector_band {
                            sector_a += 1;
                        }
                        // Center dot (the axis).
                        if dist < s * 0.055 {
                            dot_a += 1;
                        }
                    }
                }
            }
            let samples = SS * SS;
            let i = ((y * SIZE + x) * 4) as usize;
            if ink_a == 0 {
                rgba[i..i + 4].copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            // Composite over the ink base: deep ink where the sector
            // sits (contrast for the white ring), white ring + dot.
            let sector = sector_a as f32 / samples as f32;
            let base = mix(INK, INK_DEEP, sector * 0.55);
            rgba[i] = base[0];
            rgba[i + 1] = base[1];
            rgba[i + 2] = base[2];
            rgba[i + 3] = ((ink_a * 255) / samples) as u8;
            // Ring and dot draw on top (alpha from coverage).
            for (cov, color) in [
                (ring_a as f32 / samples as f32, PAPER),
                (dot_a as f32 / samples as f32, PAPER),
            ] {
                if cov > 0.0 {
                    rgba[i] = u8::from_f32s(cov, color[0], rgba[i]);
                    rgba[i + 1] = u8::from_f32s(cov, color[1], rgba[i + 1]);
                    rgba[i + 2] = u8::from_f32s(cov, color[2], rgba[i + 2]);
                    rgba[i + 3] = rgba[i + 3].max(byte01(cov));
                }
            }
        }
    }

    // Where the source lands (repo layout: app/packaging/icon/).
    let out = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("packaging")
        .join("icon")
        .join("icon-source.png");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::create(&out).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(file, SIZE, SIZE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
    println!("wrote {}", out.display());
    Ok(())
}

/// Quantize a value clamped to [0, 1] into a u8 byte (0–255).
/// try_from avoids the float→unsigned `as` cast (clippy cast_sign_loss);
/// the clamp guarantees the range, unwrap_or(255) is unreachable.
fn byte01(x: f32) -> u8 {
    u8::try_from((x.clamp(0.0, 1.0) * 255.0).round() as i64).unwrap_or(255)
}

/// Linear mix of two RGB colors.
fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [
        u8::from_f32s(t, b[0], a[0]),
        u8::from_f32s(t, b[1], a[1]),
        u8::from_f32s(t, b[2], a[2]),
    ]
}

/// Blend helper: `over` with coverage `cov` onto `base`.
trait FromF32s {
    fn from_f32s(cov: f32, over: u8, base: u8) -> u8;
}
impl FromF32s for u8 {
    fn from_f32s(cov: f32, over: u8, base: u8) -> u8 {
        let over = f32::from(over) / 255.0;
        let base = f32::from(base) / 255.0;
        let mixed = over * cov + base * (1.0 - cov);
        byte01(mixed)
    }
}
