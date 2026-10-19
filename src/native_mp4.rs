use image::{imageops::FilterType, RgbaImage};
use mp4io::{Codec, InputSample, TrackParams, Writer, WriterConfig};
use rusty_h264::{Encoder, EncoderConfig, Preset, YuvFrame};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::assets::RuntimeAssets;
use crate::export;
use crate::model::{ExportQuality, Project};

pub fn export_mp4(
    project: &Project,
    assets: &RuntimeAssets,
    path: &Path,
    progress: &AtomicU32,
    cancel: &AtomicBool,
) -> Result<PathBuf, String> {
    let fps = project.export.fps.clamp(12, 60);
    let frame_count = (project.duration_seconds.max(0.5) * fps as f32)
        .ceil()
        .max(1.0) as u32;

    let out_w = even(project.export.width.max(320));
    let out_h = even(project.export.height.max(240));

    let mut cfg = EncoderConfig::new(out_w as usize, out_h as usize);
    cfg.preset = match project.export.quality {
        ExportQuality::High => Preset::Balanced,
        ExportQuality::Medium => Preset::Balanced,
        ExportQuality::Low => Preset::Fast,
    };
    cfg.gop_size = (fps * 2).max(1);
    cfg.mbtree = false;
    cfg.framerate = fps as f32;
    cfg.bitrate = bitrate_for(out_w, out_h, fps, project.export.quality);

    let mut encoder = Encoder::new(cfg).map_err(|e| format!("Create H.264 encoder: {e}"))?;
    let timescale = fps.saturating_mul(1000).max(1000);
    let mut writer: Option<(Writer, mp4io::TrackId)> = None;

    for i in 0..frame_count {
        if cancel.load(Ordering::Relaxed) {
            let _ = fs::remove_file(path);
            return Err("Export cancelled".to_owned());
        }

        let p = if frame_count <= 1 {
            1.0
        } else {
            i as f32 / (frame_count - 1) as f32
        };
        let frame = export::render_frame(project, assets, p)
            .map_err(|e| format!("Render frame {i}: {e:#}"))?;
        let frame = if frame.width() != out_w || frame.height() != out_h {
            image::imageops::resize(&frame, out_w, out_h, FilterType::Lanczos3)
        } else {
            frame
        };
        let yuv = rgba_to_i420(&frame);
        let annex_b = encoder.encode(&yuv);
        if annex_b.is_empty() {
            return Err("H.264 encoder unexpectedly buffered a frame".to_owned());
        }

        let parsed = parse_access_unit(&annex_b)?;
        if writer.is_none() {
            let sps = parsed.sps.as_ref().ok_or_else(|| "First H.264 frame did not contain SPS".to_owned())?;
            let pps = parsed.pps.as_ref().ok_or_else(|| "First H.264 frame did not contain PPS".to_owned())?;
            let avcc = make_avcc(sps, pps)?;
            let mut w = Writer::new(WriterConfig::default());
            let track = w.add_track(
                TrackParams::video(Codec::H264, out_w as u16, out_h as u16)
                    .timescale(timescale)
                    .avc_config(avcc),
            );
            writer = Some((w, track));
        }

        let (w, track) = writer.as_mut().expect("writer initialized above");
        w.write_sample(
            *track,
            InputSample {
                data: &parsed.sample,
                duration: 1000,
                composition_offset: 0,
                is_sync: parsed.is_sync,
            },
        )
        .map_err(|e| format!("Mux H.264 frame {i}: {e}"))?;

        progress.store(
            ((((i + 1) as f32 / frame_count as f32) * 980.0).round() as u32).min(980),
            Ordering::Relaxed,
        );
    }

    if cancel.load(Ordering::Relaxed) {
        let _ = fs::remove_file(path);
        return Err("Export cancelled".to_owned());
    }

    let (writer, _) = writer.ok_or_else(|| "No frames were encoded".to_owned())?;
    let bytes = writer.finalize().map_err(|e| format!("Finalize MP4: {e}"))?;
    fs::write(path, bytes).map_err(|e| format!("Write {}: {e}", path.display()))?;
    progress.store(1000, Ordering::Relaxed);
    Ok(path.to_path_buf())
}

struct ParsedAccessUnit {
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
    sample: Vec<u8>,
    is_sync: bool,
}

fn parse_access_unit(data: &[u8]) -> Result<ParsedAccessUnit, String> {
    let nals = split_annex_b(data);
    if nals.is_empty() {
        return Err("H.264 encoder produced no NAL units".to_owned());
    }
    let mut sps = None;
    let mut pps = None;
    let mut sample = Vec::with_capacity(data.len());
    let mut is_sync = false;

    for nal in nals {
        if nal.is_empty() { continue; }
        match nal[0] & 0x1f {
            7 => sps = Some(nal.to_vec()),
            8 => pps = Some(nal.to_vec()),
            5 => {
                is_sync = true;
                push_avcc_nal(&mut sample, nal)?;
            }
            9 => {}
            _ => push_avcc_nal(&mut sample, nal)?,
        }
    }
    if sample.is_empty() {
        return Err("H.264 access unit contained no coded sample data".to_owned());
    }
    Ok(ParsedAccessUnit { sps, pps, sample, is_sync })
}

fn push_avcc_nal(out: &mut Vec<u8>, nal: &[u8]) -> Result<(), String> {
    let len = u32::try_from(nal.len()).map_err(|_| "H.264 NAL is too large".to_owned())?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(nal);
    Ok(())
}

fn make_avcc(sps: &[u8], pps: &[u8]) -> Result<Vec<u8>, String> {
    if sps.len() < 4 || pps.is_empty() {
        return Err("Invalid SPS/PPS returned by H.264 encoder".to_owned());
    }
    let sps_len = u16::try_from(sps.len()).map_err(|_| "SPS is too large".to_owned())?;
    let pps_len = u16::try_from(pps.len()).map_err(|_| "PPS is too large".to_owned())?;
    let mut out = Vec::with_capacity(sps.len() + pps.len() + 16);
    out.push(1);
    out.push(sps[1]);
    out.push(sps[2]);
    out.push(sps[3]);
    out.push(0xff);
    out.push(0xe1);
    out.extend_from_slice(&sps_len.to_be_bytes());
    out.extend_from_slice(sps);
    out.push(1);
    out.extend_from_slice(&pps_len.to_be_bytes());
    out.extend_from_slice(pps);
    Ok(out)
}

fn split_annex_b(data: &[u8]) -> Vec<&[u8]> {
    fn start_code(data: &[u8], from: usize) -> Option<(usize, usize)> {
        let mut i = from;
        while i + 3 <= data.len() {
            if i + 4 <= data.len() && data[i..i + 4] == [0, 0, 0, 1] {
                return Some((i, 4));
            }
            if data[i..i + 3] == [0, 0, 1] {
                return Some((i, 3));
            }
            i += 1;
        }
        None
    }

    let mut out = Vec::new();
    let Some((mut pos, mut prefix)) = start_code(data, 0) else { return out; };
    loop {
        let nal_start = pos + prefix;
        match start_code(data, nal_start) {
            Some((next, next_prefix)) => {
                if next > nal_start { out.push(&data[nal_start..next]); }
                pos = next;
                prefix = next_prefix;
            }
            None => {
                if nal_start < data.len() { out.push(&data[nal_start..]); }
                break;
            }
        }
    }
    out
}

fn rgba_to_i420(image: &RgbaImage) -> YuvFrame {
    let w = image.width() as usize;
    let h = image.height() as usize;
    debug_assert_eq!(w % 2, 0);
    debug_assert_eq!(h % 2, 0);
    let mut frame = YuvFrame::black(w, h);

    for y in 0..h {
        for x in 0..w {
            let p = image.get_pixel(x as u32, y as u32).0;
            let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
            frame.y[y * w + x] = clamp_u8(16.0 + 0.257 * r + 0.504 * g + 0.098 * b);
        }
    }

    let cw = w / 2;
    for cy in 0..(h / 2) {
        for cx in 0..cw {
            let mut r = 0.0f32;
            let mut g = 0.0f32;
            let mut b = 0.0f32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let p = image.get_pixel((cx * 2 + dx) as u32, (cy * 2 + dy) as u32).0;
                    r += p[0] as f32;
                    g += p[1] as f32;
                    b += p[2] as f32;
                }
            }
            r *= 0.25;
            g *= 0.25;
            b *= 0.25;
            let i = cy * cw + cx;
            frame.u[i] = clamp_u8(128.0 - 0.148 * r - 0.291 * g + 0.439 * b);
            frame.v[i] = clamp_u8(128.0 + 0.439 * r - 0.368 * g - 0.071 * b);
        }
    }
    frame
}

fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn even(v: u32) -> u32 {
    if v % 2 == 0 { v } else { v + 1 }
}

fn bitrate_for(width: u32, height: u32, fps: u32, quality: ExportQuality) -> u32 {
    let bits_per_pixel_frame = match quality {
        ExportQuality::High => 0.15,
        ExportQuality::Medium => 0.09,
        ExportQuality::Low => 0.05,
    };
    let estimate = width as f64 * height as f64 * fps as f64 * bits_per_pixel_frame;
    estimate.clamp(600_000.0, 24_000_000.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annex_b_parser_and_avcc_conversion() {
        let stream = [
            0, 0, 0, 1, 0x67, 0x64, 0, 31, 1, 2,
            0, 0, 1, 0x68, 0xaa, 0xbb,
            0, 0, 1, 0x65, 9, 8, 7,
        ];
        let parsed = parse_access_unit(&stream).expect("parse");
        assert_eq!(parsed.sps.as_deref(), Some(&[0x67, 0x64, 0, 31, 1, 2][..]));
        assert_eq!(parsed.pps.as_deref(), Some(&[0x68, 0xaa, 0xbb][..]));
        assert!(parsed.is_sync);
        assert_eq!(&parsed.sample[..4], &(4u32.to_be_bytes()));
        let avcc = make_avcc(parsed.sps.as_ref().unwrap(), parsed.pps.as_ref().unwrap()).expect("avcc");
        assert_eq!(&avcc[..6], &[1, 0x64, 0, 31, 0xff, 0xe1]);
    }

    #[test]
    fn rgba_to_i420_has_expected_plane_sizes() {
        let image = RgbaImage::from_pixel(4, 2, image::Rgba([255, 0, 0, 255]));
        let frame = rgba_to_i420(&image);
        assert_eq!(frame.width, 4);
        assert_eq!(frame.height, 2);
        assert_eq!(frame.y.len(), 8);
        assert_eq!(frame.u.len(), 2);
        assert_eq!(frame.v.len(), 2);
    }
}
