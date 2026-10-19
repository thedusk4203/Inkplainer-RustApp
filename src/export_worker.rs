use image::imageops::FilterType;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use crate::assets::RuntimeAssets;
use crate::export;
use crate::model::{ExportQuality, Project};
use crate::native_mp4;

pub struct ExportJob {
    pub path: PathBuf,
    progress: Arc<AtomicU32>,
    cancel: Arc<AtomicBool>,
    receiver: mpsc::Receiver<Result<PathBuf, String>>,
}

impl ExportJob {
    pub fn progress(&self) -> f32 {
        self.progress.load(Ordering::Relaxed).min(1000) as f32 / 1000.0
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn try_finish(&self) -> Option<Result<PathBuf, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Export worker disconnected unexpectedly".to_owned())),
        }
    }
}

pub fn start(project: Project, assets: RuntimeAssets, path: PathBuf, webm: bool) -> ExportJob {
    let progress = Arc::new(AtomicU32::new(0));
    let cancel = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    let worker_progress = Arc::clone(&progress);
    let worker_cancel = Arc::clone(&cancel);
    let worker_path = path.clone();

    thread::spawn(move || {
        let result = if webm {
            run_webm_export(project, assets, worker_path, &worker_progress, &worker_cancel)
        } else {
            native_mp4::export_mp4(&project, &assets, &worker_path, &worker_progress, &worker_cancel)
        };
        let _ = sender.send(result);
    });

    ExportJob { path, progress, cancel, receiver }
}

fn run_webm_export(
    project: Project,
    assets: RuntimeAssets,
    path: PathBuf,
    progress: &AtomicU32,
    cancel: &AtomicBool,
) -> Result<PathBuf, String> {
    let fps = project.export.fps.clamp(12, 60);
    let frame_count = (project.duration_seconds.max(0.5) * fps as f32).ceil().max(1.0) as u32;
    let temp = std::env::temp_dir().join(format!("inkplainer-export-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp).map_err(|e| format!("Create export temp directory: {e}"))?;
    let out_w = project.export.width.max(320);
    let out_h = project.export.height.max(240);

    let result = (|| -> Result<PathBuf, String> {
        for i in 0..frame_count {
            if cancel.load(Ordering::Relaxed) {
                return Err("Export cancelled".to_owned());
            }
            let p = if frame_count <= 1 { 1.0 } else { i as f32 / (frame_count - 1) as f32 };
            let frame = export::render_frame(&project, &assets, p).map_err(|e| format!("Render frame {i}: {e:#}"))?;
            let frame = if frame.width() != out_w || frame.height() != out_h {
                image::imageops::resize(&frame, out_w, out_h, FilterType::Lanczos3)
            } else { frame };
            frame.save(temp.join(format!("frame-{i:06}.png"))).map_err(|e| format!("Save frame {i}: {e}"))?;
            progress.store((((i + 1) as f32 / frame_count as f32) * 900.0).round() as u32, Ordering::Relaxed);
        }

        if cancel.load(Ordering::Relaxed) {
            return Err("Export cancelled".to_owned());
        }

        let input = temp.join("frame-%06d.png");
        let crf = match project.export.quality {
            ExportQuality::High => "18",
            ExportQuality::Medium => "23",
            ExportQuality::Low => "30",
        };
        let mut command = Command::new("ffmpeg");
        command.args(["-y", "-hide_banner", "-loglevel", "error", "-framerate", &fps.to_string(), "-i"])
            .arg(&input)
            .args(["-c:v", "libvpx-vp9", "-crf", crf, "-b:v", "0", "-pix_fmt", "yuv420p"])
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|e| format!("Start ffmpeg for WebM: {e}. MP4 export is fully self-contained; install ffmpeg only for WebM until the native AV1 muxer is enabled."))?;
        progress.store(950, Ordering::Relaxed);

        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&path);
                return Err("Export cancelled".to_owned());
            }
            match child.try_wait().map_err(|e| format!("Poll ffmpeg: {e}"))? {
                Some(status) if status.success() => break,
                Some(status) => {
                    let mut message = format!("ffmpeg exited with status {status}");
                    if let Some(mut stderr) = child.stderr.take() {
                        use std::io::Read;
                        let mut text = String::new();
                        let _ = stderr.read_to_string(&mut text);
                        if !text.trim().is_empty() { message.push_str(": "); message.push_str(text.trim()); }
                    }
                    return Err(message);
                }
                None => thread::sleep(Duration::from_millis(50)),
            }
        }

        progress.store(1000, Ordering::Relaxed);
        Ok(path.clone())
    })();

    let _ = fs::remove_dir_all(&temp);
    result
}
