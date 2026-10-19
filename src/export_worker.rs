use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use crate::assets::RuntimeAssets;
use crate::model::Project;
use crate::{native_mp4, native_webm};

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


    pub fn try_finish(&self) -> Option<Result<PathBuf, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Export worker disconnected unexpectedly".to_owned()))
            }
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
            native_webm::export_webm(
                &project,
                &assets,
                &worker_path,
                &worker_progress,
                &worker_cancel,
            )
        } else {
            native_mp4::export_mp4(
                &project,
                &assets,
                &worker_path,
                &worker_progress,
                &worker_cancel,
            )
        };
        let _ = sender.send(result);
    });

    ExportJob {
        path,
        progress,
        cancel,
        receiver,
    }
}
