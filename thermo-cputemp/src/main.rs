use std::{
    thread,
    time::{Duration, Instant},
};

fn main() {
    loop {
        let start = Instant::now();
        let components = sysinfo::Components::new_with_refreshed_list();
        for component in components.iter() {
            if let Some(temp) = component.temperature() {
                println!("Component: {}, Temperature: {}°C", component.label(), temp);
            } else {
                println!(
                    "Component: {}, Temperature data not available",
                    component.label()
                );
            }
        }
        let elapsed = start.elapsed();
        if elapsed < Duration::from_secs(1) {
            thread::sleep(Duration::from_secs(1) - elapsed);
        }
    }
}
