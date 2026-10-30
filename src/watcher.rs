//! Debounced filesystem watching with a polling fallback.

use crate::app::Event;
use anyhow::Result;
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct WatcherHandle {
    _native: Option<Box<dyn std::any::Any>>,
    stop: Option<Arc<AtomicBool>>,
    poller: Option<JoinHandle<()>>,
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop {
            stop.store(true, Ordering::SeqCst);
        }
        if let Some(poller) = self.poller.take() {
            let _ = poller.join();
        }
    }
}

pub fn start(directory: PathBuf, sender: Sender<Event>) -> Result<WatcherHandle> {
    match start_native(&directory, sender.clone()) {
        Ok(handle) => Ok(handle),
        Err(_) => start_polling(directory, sender),
    }
}

fn start_native(directory: &Path, sender: Sender<Event>) -> Result<WatcherHandle> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(400),
        None,
        move |result: DebounceEventResult| {
            if let Ok(events) = result {
                if events.iter().any(|event| is_relevant(&event.event)) {
                    let _ = sender.send(Event::FilesChanged);
                }
            }
        },
    )?;
    debouncer
        .watcher()
        .watch(directory, RecursiveMode::NonRecursive)?;
    debouncer
        .cache()
        .add_root(directory, RecursiveMode::NonRecursive);
    Ok(WatcherHandle {
        _native: Some(Box::new(debouncer)),
        stop: None,
        poller: None,
    })
}

fn start_polling(directory: PathBuf, sender: Sender<Event>) -> Result<WatcherHandle> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let poller = std::thread::spawn(move || {
        let mut previous = pdf_snapshot(&directory);
        while !flag.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(2));
            let current = pdf_snapshot(&directory);
            if current != previous {
                previous = current;
                if sender.send(Event::FilesChanged).is_err() {
                    break;
                }
            }
        }
    });
    Ok(WatcherHandle {
        _native: None,
        stop: Some(stop),
        poller: Some(poller),
    })
}

fn pdf_snapshot(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_pdf_path(path))
        .collect();
    paths.sort();
    paths
}

pub fn is_relevant(event: &notify::Event) -> bool {
    event.paths.iter().any(|path| is_pdf_path(path))
}

pub fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}
