use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crate::{Measurement, safe_mpsc};

pub fn cputemp_thread(running: Arc<AtomicBool>, sink: safe_mpsc::SafeSender<Measurement>) {
    while running.load(Ordering::Relaxed) {
        let start = Instant::now();
        let components = sysinfo::Components::new_with_refreshed_list();
        let mut meas = components
            .iter()
            .enumerate()
            .filter_map(|(idx, component)| component.temperature().map(|temp| (idx as u32, temp)))
            .collect::<Vec<_>>();
        meas.truncate(10); // Limit to 10 measurements
        if !meas.is_empty() {
            let measurement = Measurement::Temperature(meas);
            if let Err(e) = sink.send(measurement) {
                log::error!("[CPU] Failed to send measurement: {e:?}");
                continue; // we are probably shutting down
            }
        } else {
            log::warn!("[CPU] No temperature data available");
        }
        let elapsed = start.elapsed();
        if elapsed < Duration::from_secs(1) {
            thread::sleep(Duration::from_secs(1) - elapsed);
        }
    }
}
