use image::{GrayImage, Luma, RgbaImage};
use imageproc::edges::canny;

use crate::model::{AnimationSettings, DetectionAlgorithm};

#[derive(Debug, Clone, Default)]
pub struct EdgePaths {
    pub classic: Vec<[f32; 2]>,
    pub adaptive: Vec<[f32; 2]>,
    pub morph_shell: Vec<[f32; 2]>,
    pub canny_plus: Vec<[f32; 2]>,
    pub color_regions: Vec<[f32; 2]>,
    pub real_image: Vec<[f32; 2]>,
}

pub fn build_edge_paths(img: &RgbaImage) -> EdgePaths {
    EdgePaths {
        classic: classic_path(img),
        adaptive: adaptive_path(img),
        morph_shell: morph_shell_path(img),
        canny_plus: canny_path(img),
        color_regions: color_region_path(img),
        real_image: gradient_path(img),
    }
}

pub fn select_edge_path<'a>(paths: &'a EdgePaths, settings: &AnimationSettings) -> &'a [[f32; 2]] {
    if settings.real_image_edges {
        return &paths.real_image;
    }
    if settings.color_region_outlines {
        return &paths.color_regions;
    }
    match settings.detection_algorithm {
        DetectionAlgorithm::Classic => &paths.classic,
        DetectionAlgorithm::Adaptive => &paths.adaptive,
        DetectionAlgorithm::MorphShell => &paths.morph_shell,
        DetectionAlgorithm::CannyPlus => &paths.canny_plus,
    }
}

fn rgba_to_gray(img: &RgbaImage) -> GrayImage {
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            let l = (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32).round() as u8;
            out.put_pixel(x, y, Luma([l]));
        }
    }
    out
}

fn classic_path(img: &RgbaImage) -> Vec<[f32; 2]> {
    let gray = rgba_to_gray(img);
    let (w, h) = gray.dimensions();
    let mut mask = vec![false; (w * h) as usize];
    if w < 3 || h < 3 { return Vec::new(); }
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let p = img.get_pixel(x, y).0;
            if p[3] < 18 { continue; }
            let gx = gray.get_pixel(x + 1, y).0[0] as i16 - gray.get_pixel(x - 1, y).0[0] as i16;
            let gy = gray.get_pixel(x, y + 1).0[0] as i16 - gray.get_pixel(x, y - 1).0[0] as i16;
            let magnitude = gx.unsigned_abs() as u16 + gy.unsigned_abs() as u16;
            let alpha_boundary = [
                img.get_pixel(x - 1, y).0[3], img.get_pixel(x + 1, y).0[3],
                img.get_pixel(x, y - 1).0[3], img.get_pixel(x, y + 1).0[3],
            ].iter().any(|a| *a < 20);
            if magnitude > 52 || alpha_boundary {
                mask[(y * w + x) as usize] = true;
            }
        }
    }
    path_from_mask(&mask, w, h)
}

fn adaptive_path(img: &RgbaImage) -> Vec<[f32; 2]> {
    let gray = rgba_to_gray(img);
    let (w, h) = gray.dimensions();
    if w < 5 || h < 5 { return classic_path(img); }
    let mut mask = vec![false; (w * h) as usize];
    let radius = 3i32;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if img.get_pixel(x, y).0[3] < 18 { continue; }
            let mut sum = 0u32;
            let mut count = 0u32;
            for oy in -radius..=radius {
                for ox in -radius..=radius {
                    let nx = (x as i32 + ox).clamp(0, w as i32 - 1) as u32;
                    let ny = (y as i32 + oy).clamp(0, h as i32 - 1) as u32;
                    sum += gray.get_pixel(nx, ny).0[0] as u32;
                    count += 1;
                }
            }
            let mean = (sum / count.max(1)) as i16;
            let v = gray.get_pixel(x, y).0[0] as i16;
            let gx = gray.get_pixel(x + 1, y).0[0] as i16 - gray.get_pixel(x - 1, y).0[0] as i16;
            let gy = gray.get_pixel(x, y + 1).0[0] as i16 - gray.get_pixel(x, y - 1).0[0] as i16;
            if (v - mean).unsigned_abs() > 18 || gx.unsigned_abs() + gy.unsigned_abs() > 40 {
                mask[(y * w + x) as usize] = true;
            }
        }
    }
    path_from_mask(&mask, w, h)
}

fn morph_shell_path(img: &RgbaImage) -> Vec<[f32; 2]> {
    let (w, h) = img.dimensions();
    if w < 3 || h < 3 { return Vec::new(); }
    let mut solid = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            let spread = p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]);
            solid[(y * w + x) as usize] = p[3] > 28 && (spread > 8 || p[0] < 245 || p[1] < 245 || p[2] < 245);
        }
    }
    let mut shell = vec![false; (w * h) as usize];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y * w + x) as usize;
            if !solid[i] { continue; }
            let eroded = solid[(y * w + x - 1) as usize]
                && solid[(y * w + x + 1) as usize]
                && solid[((y - 1) * w + x) as usize]
                && solid[((y + 1) * w + x) as usize];
            shell[i] = !eroded;
        }
    }
    path_from_mask(&shell, w, h)
}

fn canny_path(img: &RgbaImage) -> Vec<[f32; 2]> {
    let gray = rgba_to_gray(img);
    let edges = canny(&gray, 30.0, 80.0);
    let (w, h) = edges.dimensions();
    let mut mask = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            mask[(y * w + x) as usize] = edges.get_pixel(x, y).0[0] > 0 && img.get_pixel(x, y).0[3] > 12;
        }
    }
    path_from_mask(&mask, w, h)
}

fn color_region_path(img: &RgbaImage) -> Vec<[f32; 2]> {
    let (w, h) = img.dimensions();
    if w < 3 || h < 3 { return Vec::new(); }
    let mut mask = vec![false; (w * h) as usize];
    let quant = |p: [u8; 4]| -> [u8; 3] { [p[0] / 32, p[1] / 32, p[2] / 32] };
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let p = img.get_pixel(x, y).0;
            if p[3] < 20 { continue; }
            let q = quant(p);
            let different = [
                img.get_pixel(x + 1, y).0,
                img.get_pixel(x - 1, y).0,
                img.get_pixel(x, y + 1).0,
                img.get_pixel(x, y - 1).0,
            ].iter().any(|n| n[3] > 20 && quant(*n) != q);
            mask[(y * w + x) as usize] = different;
        }
    }
    path_from_mask(&mask, w, h)
}

fn gradient_path(img: &RgbaImage) -> Vec<[f32; 2]> {
    let gray = rgba_to_gray(img);
    let (w, h) = gray.dimensions();
    if w < 3 || h < 3 { return Vec::new(); }
    let mut strengths = Vec::with_capacity((w * h) as usize);
    let mut values = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let s = if x == 0 || y == 0 || x + 1 >= w || y + 1 >= h { 0u16 } else {
                let gx = gray.get_pixel(x + 1, y).0[0] as i16 - gray.get_pixel(x - 1, y).0[0] as i16;
                let gy = gray.get_pixel(x, y + 1).0[0] as i16 - gray.get_pixel(x, y - 1).0[0] as i16;
                gx.unsigned_abs() as u16 + gy.unsigned_abs() as u16
            };
            values.push(s);
            if s > 0 { strengths.push(s); }
        }
    }
    strengths.sort_unstable();
    let threshold = strengths.get(strengths.len().saturating_mul(68) / 100).copied().unwrap_or(48).max(28);
    let mask: Vec<bool> = values.into_iter().enumerate().map(|(i, v)| {
        let x = (i as u32) % w;
        let y = (i as u32) / w;
        v >= threshold && img.get_pixel(x, y).0[3] > 12
    }).collect();
    path_from_mask(&mask, w, h)
}

fn path_from_mask(mask: &[bool], w: u32, h: u32) -> Vec<[f32; 2]> {
    if w == 0 || h == 0 { return Vec::new(); }
    let mut pts = Vec::new();
    // Serpentine scan keeps adjacent edge fragments locally connected and makes the drawing
    // direction look hand-generated while still allowing the renderer to suppress long jumps.
    for y in 0..h {
        if y % 2 == 0 {
            for x in 0..w {
                if mask[(y * w + x) as usize] { pts.push([x as f32 / w as f32, y as f32 / h as f32]); }
            }
        } else {
            for x in (0..w).rev() {
                if mask[(y * w + x) as usize] { pts.push([x as f32 / w as f32, y as f32 / h as f32]); }
            }
        }
    }
    decimate(pts, 12_000)
}

fn decimate(points: Vec<[f32; 2]>, max_points: usize) -> Vec<[f32; 2]> {
    if points.len() <= max_points { return points; }
    let stride = (points.len() as f32 / max_points as f32).ceil() as usize;
    points.into_iter().step_by(stride.max(1)).collect()
}
