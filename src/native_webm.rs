use ebml_webm::mux::Muxer;
use ebml_webm::types::TrackInfo;
use image::{imageops::FilterType, RgbaImage};
use rav1e::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::assets::RuntimeAssets;
use crate::export;
use crate::model::{ExportQuality, Project};

pub fn export_webm(
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

    let speed = match project.export.quality {
        ExportQuality::High => 6,
        ExportQuality::Medium => 8,
        ExportQuality::Low => 10,
    };
    let quantizer = match project.export.quality {
        ExportQuality::High => 70,
        ExportQuality::Medium => 100,
        ExportQuality::Low => 140,
    };

    let mut enc = EncoderConfig::with_speed_preset(speed);
    enc.width = out_w as usize;
    enc.height = out_h as usize;
    enc.time_base = Rational::new(1, fps as u64);
    enc.low_latency = true;
    enc.quantizer = quantizer;
    enc.bitrate = 0;
    enc.set_key_frame_interval(0, (fps as u64 * 2).max(1));

    let cfg = Config::new().with_encoder_config(enc);
    let mut encoder: Context<u8> = cfg
        .new_context()
        .map_err(|e| format!("Create AV1 encoder: {e}"))?;

    let mut muxer = Muxer::new();
    muxer
        .add_track(TrackInfo {
            track_number: 1,
            track_type: 1,
            codec_id: "V_AV1".to_owned(),
            codec_private: None,
            width: out_w as u64,
            height: out_h as u64,
            sample_rate: 0.0,
            channels: 0,
        })
        .map_err(|e| format!("Create WebM video track: {e}"))?;
    let mut muxer = muxer.begin();

    let mut encoded_packets = 0u32;
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

        let (y, u, v) = rgba_to_i420(&frame);
        let mut av1_frame = encoder.new_frame();
        av1_frame.planes[0].copy_from_raw_u8(&y, out_w as usize, 1);
        av1_frame.planes[1].copy_from_raw_u8(&u, (out_w / 2) as usize, 1);
        av1_frame.planes[2].copy_from_raw_u8(&v, (out_w / 2) as usize, 1);
        av1_frame.planes[0].pad(out_w as usize, out_h as usize);
        av1_frame.planes[1].pad((out_w / 2) as usize, (out_h / 2) as usize);
        av1_frame.planes[2].pad((out_w / 2) as usize, (out_h / 2) as usize);
        encoder
            .send_frame(av1_frame)
            .map_err(|e| format!("Queue AV1 frame {i}: {e}"))?;

        drain_packets(&mut encoder, &mut muxer, fps, &mut encoded_packets)?;
        progress.store(
            ((((i + 1) as f32 / frame_count as f32) * 900.0).round() as u32).min(900),
            Ordering::Relaxed,
        );
    }

    encoder.flush();
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = fs::remove_file(path);
            return Err("Export cancelled".to_owned());
        }
        match encoder.receive_packet() {
            Ok(packet) => {
                push_packet(&mut muxer, fps, packet)?;
                encoded_packets += 1;
                let flush_progress = 900
                    + ((encoded_packets.min(frame_count) as f32 / frame_count as f32) * 90.0)
                        as u32;
                progress.store(flush_progress.min(990), Ordering::Relaxed);
            }
            Err(EncoderStatus::LimitReached) => break,
            Err(EncoderStatus::NeedMoreData) | Err(EncoderStatus::Encoded) => continue,
            Err(e) => return Err(format!("Drain AV1 encoder: {e}")),
        }
    }

    muxer.flush();
    let mut bytes = Vec::new();
    muxer.poll_bytes(&mut bytes);
    if bytes.is_empty() {
        return Err("WebM muxer produced no output".to_owned());
    }
    fs::write(path, bytes).map_err(|e| format!("Write {}: {e}", path.display()))?;
    progress.store(1000, Ordering::Relaxed);
    Ok(path.to_path_buf())
}

fn drain_packets(
    encoder: &mut Context<u8>,
    muxer: &mut Muxer<ebml_webm::mux::Live>,
    fps: u32,
    encoded_packets: &mut u32,
) -> Result<(), String> {
    loop {
        match encoder.receive_packet() {
            Ok(packet) => {
                push_packet(muxer, fps, packet)?;
                *encoded_packets += 1;
            }
            Err(EncoderStatus::NeedMoreData)
            | Err(EncoderStatus::Encoded)
            | Err(EncoderStatus::LimitReached) => break,
            Err(e) => return Err(format!("Receive AV1 packet: {e}")),
        }
    }
    Ok(())
}

fn push_packet(
    muxer: &mut Muxer<ebml_webm::mux::Live>,
    fps: u32,
    packet: Packet<u8>,
) -> Result<(), String> {
    let timecode_ms = packet.input_frameno.saturating_mul(1000) / fps as u64;
    let keyframe = packet.frame_type == FrameType::KEY;
    muxer
        .push_frame(1, timecode_ms, keyframe, &packet.data)
        .map_err(|e| format!("Mux AV1 packet: {e}"))?;
    Ok(())
}

fn rgba_to_i420(image: &RgbaImage) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let w = image.width() as usize;
    let h = image.height() as usize;
    debug_assert_eq!(w % 2, 0);
    debug_assert_eq!(h % 2, 0);
    let mut y_plane = vec![0u8; w * h];
    let mut u_plane = vec![0u8; (w / 2) * (h / 2)];
    let mut v_plane = vec![0u8; (w / 2) * (h / 2)];

    for y in 0..h {
        for x in 0..w {
            let p = image.get_pixel(x as u32, y as u32).0;
            let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
            y_plane[y * w + x] = clamp_u8(16.0 + 0.257 * r + 0.504 * g + 0.098 * b);
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
                    let p = image
                        .get_pixel((cx * 2 + dx) as u32, (cy * 2 + dy) as u32)
                        .0;
                    r += p[0] as f32;
                    g += p[1] as f32;
                    b += p[2] as f32;
                }
            }
            r *= 0.25;
            g *= 0.25;
            b *= 0.25;
            let i = cy * cw + cx;
            u_plane[i] = clamp_u8(128.0 - 0.148 * r - 0.291 * g + 0.439 * b);
            v_plane[i] = clamp_u8(128.0 + 0.439 * r - 0.368 * g - 0.071 * b);
        }
    }
    (y_plane, u_plane, v_plane)
}

fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn even(v: u32) -> u32 {
    if v % 2 == 0 { v } else { v + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_to_i420_produces_expected_plane_sizes() {
        let image = RgbaImage::from_pixel(4, 2, image::Rgba([0, 255, 0, 255]));
        let (y, u, v) = rgba_to_i420(&image);
        assert_eq!(y.len(), 8);
        assert_eq!(u.len(), 2);
        assert_eq!(v.len(), 2);
    }
}
