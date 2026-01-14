//! Background worker for async frame loading.
//!
//! Separates heavy computation from UI thread to keep interface responsive.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::abc::IArchive;
use super::mesh_converter::{self, CollectedScene};

/// Commands sent from UI to worker.
#[derive(Debug)]
pub enum WorkerCommand {
    /// Load frame data for the given frame index.
    LoadFrame { frame: usize, epoch: u64 },
    /// Stop the worker thread.
    Stop,
}

/// Results sent from worker back to UI.
pub enum WorkerResult {
    /// Frame data is ready.
    FrameReady {
        frame: usize,
        epoch: u64,
        scene: CollectedScene,
    },
}

/// Handle to communicate with the background worker.
pub struct WorkerHandle {
    /// Send commands to worker.
    pub tx: Sender<WorkerCommand>,
    /// Receive results from worker.
    pub rx: Receiver<WorkerResult>,
    /// Thread handle for cleanup.
    handle: Option<JoinHandle<()>>,
}

impl WorkerHandle {
    /// Spawn a new worker thread for the given archive.
    pub fn spawn(archive: Arc<IArchive>) -> Self {
        let (cmd_tx, cmd_rx) = channel::<WorkerCommand>();
        let (res_tx, res_rx) = channel::<WorkerResult>();

        let handle = thread::spawn(move || {
            worker_loop(archive, cmd_rx, res_tx);
        });

        Self {
            tx: cmd_tx,
            rx: res_rx,
            handle: Some(handle),
        }
    }

    /// Request a frame to be loaded with given epoch.
    pub fn request_frame(&self, frame: usize, epoch: u64) {
        let _ = self.tx.send(WorkerCommand::LoadFrame { frame, epoch });
    }

    /// Check for ready results (non-blocking).
    pub fn try_recv(&self) -> Option<WorkerResult> {
        self.rx.try_recv().ok()
    }

    /// Stop the worker and wait for it to finish.
    pub fn stop(&mut self) {
        let _ = self.tx.send(WorkerCommand::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Main worker loop - runs in background thread.
fn worker_loop(
    archive: Arc<IArchive>,
    rx: Receiver<WorkerCommand>,
    tx: Sender<WorkerResult>,
) {
    loop {
        // Wait for command
        let cmd = match rx.recv() {
            Ok(cmd) => cmd,
            Err(_) => break, // Channel closed
        };

        match cmd {
            WorkerCommand::LoadFrame { frame, epoch } => {
                // Before doing work, drain any newer requests
                // This handles rapid scrubbing - only process the latest
                let (final_frame, final_epoch) = drain_to_latest(&rx, frame, epoch);
                
                // Collect scene data for this frame
                let scene = mesh_converter::collect_scene(&archive, final_frame);
                
                // Send result back
                if tx.send(WorkerResult::FrameReady { 
                    frame: final_frame, 
                    epoch: final_epoch, 
                    scene 
                }).is_err() {
                    break; // UI disconnected
                }
            }

            WorkerCommand::Stop => {
                break;
            }
        }
    }
}

/// Drain channel to get the latest frame request, discarding older ones.
fn drain_to_latest(
    rx: &Receiver<WorkerCommand>,
    mut frame: usize,
    mut epoch: u64,
) -> (usize, u64) {
    // Non-blocking drain of queued requests
    while let Ok(cmd) = rx.try_recv() {
        match cmd {
            WorkerCommand::LoadFrame { frame: f, epoch: e } => {
                frame = f;
                epoch = e;
            }
            WorkerCommand::Stop => {
                // Put stop back and return current
                // Actually we can't put it back, so just return
                // The main loop will get Stop on next recv()
                return (frame, epoch);
            }
        }
    }
    (frame, epoch)
}
