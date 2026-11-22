use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::{Shared, store};

const OVERDUE_GRACE_MS: u64 = 5_000;
/// A sleep/wake gap longer than this is treated as a pause, not a missed reminder.
const SLEEP_GAP_TOLERANCE_MS: u64 = 60_000;

/// Restart the nudge clock. Any older timer notices the generation bump and exits.
pub fn reschedule(app: &AppHandle, shared: &Arc<Shared>) {
    let generation = shared.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    let shared = shared.clone();

    tauri::async_runtime::spawn(async move {
        let (task, set_at, interval) = {
            let data = shared.data.lock().unwrap();
            (
                data.task.clone(),
                data.task_set_at_ms,
                data.interval_minutes,
            )
        };

        if task.is_empty() || interval == 0 {
            return;
        }

        let interval_ms = u64::from(interval) * 60_000;
        let elapsed = (store::now_ms() - set_at).max(0) as u64;
        let first_wait = if elapsed >= interval_ms {
            OVERDUE_GRACE_MS
        } else {
            interval_ms - elapsed
        };

        tokio::time::sleep(Duration::from_millis(first_wait)).await;
        if !is_current(&shared, generation) {
            return;
        }
        if !fire(&app, &shared) {
            return;
        }

        let mut last_fired = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
            if !is_current(&shared, generation) {
                return;
            }

            let gap = last_fired.elapsed();
            last_fired = Instant::now();
            if gap.as_millis() as u64 > interval_ms + SLEEP_GAP_TOLERANCE_MS {
                // The Mac was asleep. Treat wake-up as a fresh start.
                continue;
            }

            if !fire(&app, &shared) {
                return;
            }
        }
    });
}

fn is_current(shared: &Shared, generation: u64) -> bool {
    shared.generation.load(Ordering::SeqCst) == generation
}

fn fire(app: &AppHandle, shared: &Shared) -> bool {
    let task = {
        let mut data = shared.data.lock().unwrap();
        if data.task.is_empty() || data.interval_minutes == 0 {
            return false;
        }
        data.task_set_at_ms = store::now_ms();
        data.task.clone()
    };

    persist(shared);

    let _ = app.emit("nudge", ());
    let _ = app
        .notification()
        .builder()
        .title("nudge")
        .body(format!("Still working on: {task}?"))
        .show();

    true
}

pub fn persist(shared: &Shared) {
    let data = shared.data.lock().unwrap().clone();
    store::save(&shared.dir, &data);
}
