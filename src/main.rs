mod config;
mod cycle_state;
mod daemon;
mod overlay;
mod x11_manager;

use anyhow::Result;
use config::Config;
use cycle_state::CycleState;
use daemon::Daemon;
use daemonize::Daemonize;
#[allow(deprecated)]
use nix::fcntl::{flock, FlockArg};
use overlay::run_overlay;
use std::env;
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use x11_manager::X11Manager;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");

    let config = Config::load()?;
    let x11 = Arc::new(X11Manager::new()?);

    match command {
        "start" => {
            println!("Starting Nicotine 🚬");

            // Daemonize the process (safe Rust wrapper)
            let daemonize = Daemonize::new().working_directory("/tmp").umask(0o027);

            match daemonize.start() {
                Ok(_) => {
                    // We're now in the daemon process
                    // Start daemon in background thread
                    let x11_daemon = Arc::clone(&x11);
                    std::thread::spawn(move || {
                        let mut daemon = Daemon::new(x11_daemon);
                        if let Err(e) = daemon.run() {
                            eprintln!("Daemon error: {}", e);
                        }
                    });

                    // Wait a bit for daemon to initialize
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    // Run overlay in main thread
                    let state = Arc::new(Mutex::new(CycleState::new()));
                    if let Ok(windows) = x11.get_eve_windows() {
                        state.lock().unwrap().update_windows(windows);
                    }

                    if let Err(e) =
                        run_overlay(x11, state, config.overlay_x, config.overlay_y, config)
                    {
                        eprintln!("Overlay error: {}", e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to daemonize: {}", e);
                    std::process::exit(1);
                }
            }
        }

        "daemon" => {
            println!("Starting EVE Multibox daemon...");
            let mut daemon = Daemon::new(x11);
            daemon.run()?;
        }

        "overlay" => {
            println!("Starting EVE Multibox Overlay...");
            let state = Arc::new(Mutex::new(CycleState::new()));

            // Initialize windows
            if let Ok(windows) = x11.get_eve_windows() {
                state.lock().unwrap().update_windows(windows);
            }

            if let Err(e) = run_overlay(x11, state, config.overlay_x, config.overlay_y, config) {
                eprintln!("Overlay error: {}", e);
                std::process::exit(1);
            }
        }

        "stack" => {
            println!("Stacking EVE windows...");
            let windows = x11.get_eve_windows()?;

            println!(
                "Centering {} EVE clients ({}x{}) on {}x{} display",
                windows.len(),
                config.eve_width,
                config.eve_height_adjusted(),
                config.display_width,
                config.display_height
            );

            x11.stack_windows(
                &windows,
                config.eve_x(),
                config.eve_y(),
                config.eve_width,
                config.eve_height_adjusted(),
            )?;

            println!("✓ Stacked {} windows", windows.len());
        }

        "cycle-forward" | "forward" | "f" => {
            // Try daemon first
            if daemon::send_command("forward").is_ok() {
                return Ok(());
            }

            // Fallback to direct mode

            // Try to acquire lock, exit immediately if already running
            let lock_file = "/tmp/nicotine-cycle.lock";
            let file = match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o644)
                .open(lock_file)
            {
                Ok(f) => f,
                Err(_) => return Ok(()), // Can't get lock, skip
            };

            // Try to lock (non-blocking)
            #[allow(deprecated)]
            if flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock).is_err() {
                return Ok(()); // Already running, skip this cycle
            }

            let mut state = CycleState::new();
            let windows = x11.get_eve_windows()?;

            if windows.is_empty() {
                return Ok(());
            }

            state.update_windows(windows);

            // Sync with current active window
            if let Ok(active) = x11.get_active_window() {
                state.sync_with_active(active);
            }

            state.cycle_forward(&x11)?;

            // Lock is automatically released when file is dropped
        }

        "cycle-backward" | "backward" | "b" => {
            // Try daemon first
            if daemon::send_command("backward").is_ok() {
                return Ok(());
            }

            // Fallback to direct mode

            // Try to acquire lock, exit immediately if already running
            let lock_file = "/tmp/nicotine-cycle.lock";
            let file = match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o644)
                .open(lock_file)
            {
                Ok(f) => f,
                Err(_) => return Ok(()), // Can't get lock, skip
            };

            // Try to lock (non-blocking)
            #[allow(deprecated)]
            if flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock).is_err() {
                return Ok(()); // Already running, skip this cycle
            }

            let mut state = CycleState::new();
            let windows = x11.get_eve_windows()?;

            if windows.is_empty() {
                return Ok(());
            }

            state.update_windows(windows);

            // Sync with current active window
            if let Ok(active) = x11.get_active_window() {
                state.sync_with_active(active);
            }

            state.cycle_backward(&x11)?;

            // Lock is automatically released when file is dropped
        }

        "stop" => {
            println!("Stopping Nicotine...");

            // Kill all nicotine processes
            let _ = std::process::Command::new("pkill")
                .arg("-9")
                .arg("nicotine")
                .output();

            println!("✓ Nicotine stopped");

            // Clean up socket and lock files
            let _ = std::fs::remove_file("/tmp/nicotine.sock");
            let _ = std::fs::remove_file("/tmp/nicotine-cycle.lock");
        }

        "init-config" => {
            Config::save_default()?;
        }

        _ => {
            println!("Nicotine - EVE Online Multiboxing Tool");
            println!();
            println!("Usage:");
            println!("  nicotine start         - Start everything (daemon + overlay)");
            println!("  nicotine stop          - Stop all Nicotine processes");
            println!("  nicotine stack         - Stack all EVE windows");
            println!("  nicotine forward       - Cycle forward");
            println!("  nicotine backward      - Cycle backward");
            println!("  nicotine init-config   - Create default config.toml");
            println!();
            println!("Advanced:");
            println!("  nicotine daemon        - Start daemon only");
            println!("  nicotine overlay       - Start overlay only");
            println!();
            println!("Quick start:");
            println!("  nicotine start         # Starts in background automatically");
        }
    }

    Ok(())
}
