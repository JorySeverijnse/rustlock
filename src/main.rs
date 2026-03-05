mod auth;
mod config;
mod input;
mod lock;
mod render;
mod screenshot;
mod timer;
mod util;

use config::Config;
use lock::LockManager;
use std::error::Error;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load();

    if config.debug {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    log::info!("Starting wayrustlock v{}", env!("CARGO_PKG_VERSION"));
    log::info!("Attempting to lock Wayland session...");
    log::warn!("WARNING: This is a screen locker. To kill it if stuck, use:");
    log::warn!("  Method 1: Switch to another TTY (Ctrl+Alt+F2) and kill the process");
    log::warn!("  Method 2: Use 'pkill -9 wayrustlock' from another terminal");
    log::warn!("  Method 3: Use 'killall wayrustlock'");

    let lock_manager = Arc::new(Mutex::new(LockManager::new(config.clone())));
    let ctrlc_exit = Arc::new(std::sync::atomic::AtomicBool::new(false));

    if let Err(e) = lock_wayland_session(config.clone(), lock_manager.clone(), ctrlc_exit) {
        log::error!("Failed to lock Wayland session: {}", e);
        log::warn!("Falling back to demonstration mode");
        run_demonstration_mode(config, lock_manager)?;
    }

    Ok(())
}

fn lock_wayland_session(
    config: Config,
    lock_manager: Arc<Mutex<LockManager>>,
    ctrlc_exit: Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), Box<dyn Error>> {
    use smithay_client_toolkit::{
        compositor::{CompositorHandler, CompositorState},
        output::{OutputHandler, OutputState},
        reexports::{
            calloop::{EventLoop, LoopHandle},
            calloop_wayland_source::WaylandSource,
        },
        registry::{ProvidesRegistryState, RegistryState},
        registry_handlers,
        seat::{
            keyboard::{KeyEvent, KeyboardData, KeyboardHandler, Modifiers, RepeatInfo},
            pointer::PointerHandler,
            SeatHandler, SeatState,
        },
        session_lock::{
            SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
            SessionLockSurfaceConfigure,
        },
        shm::{Shm, ShmHandler},
    };
    use std::time::Duration;
    use wayland_client::{
        globals::registry_queue_init,
        protocol::{wl_output, wl_surface},
        Connection, QueueHandle,
    };

    struct WaylandLock {
        loop_handle: LoopHandle<'static, Self>,
        conn: Connection,
        compositor_state: CompositorState,
        output_state: OutputState,
        registry_state: RegistryState,
        session_lock_state: SessionLockState,
        seat_state: SeatState,
        shm_state: Shm,
        session_lock: Option<SessionLock>,
        lock_surfaces: Vec<SessionLockSurface>,
        lock_manager: Arc<Mutex<LockManager>>,
        config: Config,
        ctrlc_exit: Arc<std::sync::atomic::AtomicBool>,
        exit: bool,
    }

    impl SessionLockHandler for WaylandLock {
        fn locked(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _session_lock: SessionLock,
        ) {
            log::info!("Session locked successfully!");

            if let Ok(mut lock_manager) = self.lock_manager.lock() {
                lock_manager.initialize_lock_surfaces();
            }
        }

        fn finished(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _session_lock: SessionLock,
        ) {
            log::info!("Session unlocked or lock denied");
            self.exit = true;
        }

        fn configure(
            &mut self,
            _conn: &Connection,
            qh: &QueueHandle<Self>,
            session_lock_surface: SessionLockSurface,
            configure: SessionLockSurfaceConfigure,
            _serial: u32,
        ) {
            let (width, height) = configure.new_size;
            log::debug!("Configuring lock surface: {}x{}", width, height);

            let surface_added = if let Ok(mut lock_manager) = self.lock_manager.lock() {
                lock_manager.add_surface(width as i32, height as i32)
            } else {
                false
            };

            if !surface_added {
                log::error!("Failed to add surface to lock manager");
                session_lock_surface.wl_surface().commit();
                return;
            }

            log::debug!("Surface added to lock manager");
            self.lock_surfaces.push(session_lock_surface.clone());

            if let Ok(mut lock_manager) = self.lock_manager.lock() {
                let surface_count = lock_manager.surface_count();
                if surface_count == 0 {
                    log::error!("No surfaces in lock manager");
                    session_lock_surface.wl_surface().commit();
                    return;
                }

                if let Some(locked_surface) = lock_manager.get_surface_mut(surface_count - 1) {
                    locked_surface.set_wayland_surface(session_lock_surface.wl_surface().clone());

                    match locked_surface.renderer.get_pixel_data() {
                        Ok(_pixel_data) => {
                            // For now, just commit the surface without buffer
                            // TODO: Implement proper buffer creation
                            session_lock_surface.wl_surface().commit();
                            log::debug!("Surface committed (buffer creation disabled)");
                        }
                        Err(e) => {
                            log::error!("Failed to get pixel data from Cairo surface: {:?}", e);
                            session_lock_surface.wl_surface().commit();
                        }
                    }
                }
            }
        }
    }

    impl CompositorHandler for WaylandLock {
        fn scale_factor_changed(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _surface: &wl_surface::WlSurface,
            _new_factor: i32,
        ) {
            log::debug!("Scale factor changed");
        }

        fn transform_changed(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _surface: &wl_surface::WlSurface,
            _new_transform: wl_output::Transform,
        ) {
            log::debug!("Transform changed");
        }

        fn frame(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            surface: &wl_surface::WlSurface,
            _time: u32,
        ) {
            if let Ok(mut lock_manager) = self.lock_manager.lock() {
                lock_manager.update();

                if let Some(locked_surface) = lock_manager.find_surface_by_wayland_surface(surface)
                {
                    // For now, just log that we would update the surface
                    // TODO: Implement proper buffer creation for animation
                    log::debug!("Frame update for surface");
                }
            }
        }
    }

    impl OutputHandler for WaylandLock {
        fn output_state(&mut self) -> &mut OutputState {
            &mut self.output_state
        }

        fn new_output(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
            log::info!("New output detected");
        }

        fn update_output(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
            log::debug!("Output updated");
        }

        fn output_destroyed(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
            log::info!("Output destroyed");
        }
    }

    impl ShmHandler for WaylandLock {
        fn shm_state(&mut self) -> &mut Shm {
            &mut self.shm_state
        }
    }

    impl KeyboardHandler for WaylandLock {
        fn enter(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _surface: &wl_surface::WlSurface,
            _serial: u32,
            _keys: &[u32],
            _layout_keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
        ) {
            log::debug!("Keyboard entered surface (serial: {})", _serial);
        }

        fn leave(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _surface: &wl_surface::WlSurface,
            _serial: u32,
        ) {
            log::debug!("Keyboard left surface (serial: {})", _serial);
        }

        fn press_key(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _serial: u32,
            _event: KeyEvent,
        ) {
            log::debug!("Key pressed (serial: {})", _serial);
        }

        fn release_key(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _serial: u32,
            _event: KeyEvent,
        ) {
            log::debug!("Key released (serial: {})", _serial);
        }

        fn update_modifiers(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _serial: u32,
            _modifiers: Modifiers,
        ) {
            log::debug!("Modifiers updated (serial: {})", _serial);
        }

        fn update_repeat_info(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _repeat_info: RepeatInfo,
        ) {
            log::debug!("Repeat info updated");
        }
    }

    impl PointerHandler for WaylandLock {
        fn pointer_frame(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _pointer: &wayland_client::protocol::wl_pointer::WlPointer,
            _events: &[smithay_client_toolkit::seat::pointer::PointerEvent],
        ) {
            // Screen locker doesn't need pointer events
        }
    }

    impl WaylandLock {
        fn handle_key_event(
            &mut self,
            keysym: smithay_client_toolkit::seat::keyboard::Keysym,
            pressed: bool,
        ) {
            if !pressed {
                return; // Only handle key presses for now
            }

            // Convert keysym to character
            let ch = self.keysym_to_char(keysym);

            if let Ok(mut lock_manager) = self.lock_manager.lock() {
                if let Some(ch) = ch {
                    log::info!("Key pressed: '{}'", ch);

                    // Handle special keys
                    match ch {
                        '\n' | '\r' => {
                            // Enter key - submit password
                            log::info!("Enter pressed - would submit password");
                        }
                        '\x1b' => {
                            // Escape key
                            log::info!("Escape pressed");
                        }
                        'p' | 'P' => {
                            // 'p' key - temp screenshot peek
                            log::info!("'p' pressed - would show temp screenshot");
                            lock_manager.toggle_peek();
                        }
                        _ => {
                            // Regular character - add to password
                            log::debug!("Character '{}' added to password buffer", ch);
                        }
                    }
                } else {
                    log::debug!("Non-character keysym pressed");
                }
            }
        }

        fn keysym_to_char(
            &self,
            _keysym: smithay_client_toolkit::seat::keyboard::Keysym,
        ) -> Option<char> {
            None
        }
    }

    impl SeatHandler for WaylandLock {
        fn seat_state(&mut self) -> &mut SeatState {
            &mut self.seat_state
        }

        fn new_seat(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _seat: wayland_client::protocol::wl_seat::WlSeat,
        ) {
            log::debug!("New seat detected");
        }

        fn new_capability(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _seat: wayland_client::protocol::wl_seat::WlSeat,
            capability: smithay_client_toolkit::seat::Capability,
        ) {
            log::debug!("New capability: {:?}", capability);
        }

        fn remove_capability(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _seat: wayland_client::protocol::wl_seat::WlSeat,
            capability: smithay_client_toolkit::seat::Capability,
        ) {
            log::debug!("Capability removed: {:?}", capability);
        }

        fn remove_seat(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _seat: wayland_client::protocol::wl_seat::WlSeat,
        ) {
            log::debug!("Seat removed");
        }
    }

    impl ProvidesRegistryState for WaylandLock {
        fn registry(&mut self) -> &mut RegistryState {
            &mut self.registry_state
        }
        registry_handlers![OutputState, SeatState,];
    }

    smithay_client_toolkit::delegate_compositor!(WaylandLock);
    smithay_client_toolkit::delegate_output!(WaylandLock);
    smithay_client_toolkit::delegate_session_lock!(WaylandLock);
    smithay_client_toolkit::delegate_registry!(WaylandLock);
    smithay_client_toolkit::delegate_shm!(WaylandLock);
    smithay_client_toolkit::delegate_seat!(WaylandLock);
    smithay_client_toolkit::delegate_keyboard!(WaylandLock);
    smithay_client_toolkit::delegate_pointer!(WaylandLock);

    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init(&conn)?;
    let qh: QueueHandle<WaylandLock> = event_queue.handle();
    let mut event_loop: EventLoop<WaylandLock> = EventLoop::try_new()?;

    let mut state = WaylandLock {
        loop_handle: event_loop.handle(),
        conn: conn.clone(),
        compositor_state: CompositorState::bind(&globals, &qh)?,
        output_state: OutputState::new(&globals, &qh),
        registry_state: RegistryState::new(&globals),
        session_lock_state: SessionLockState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),

        shm_state: Shm::bind(&globals, &qh).map_err(|_| "wl_shm protocol not supported")?,
        session_lock: None,
        lock_surfaces: Vec::new(),
        lock_manager,
        config,
        ctrlc_exit: ctrlc_exit.clone(),
        exit: false,
    };

    state.session_lock = Some(
        state
            .session_lock_state
            .lock(&qh)
            .map_err(|_| "ext-session-lock-v1 protocol not supported by compositor")?,
    );

    log::info!("Session lock requested, waiting for compositor...");

    WaylandSource::new(conn, event_queue).insert(event_loop.handle())?;

    while !state.exit && !state.ctrlc_exit.load(std::sync::atomic::Ordering::SeqCst) {
        event_loop.dispatch(Duration::from_millis(16), &mut state)?;
    }

    if state.ctrlc_exit.load(std::sync::atomic::Ordering::SeqCst) {
        log::warn!("Exiting due to Ctrl+C");
        // The session lock will be destroyed when the SessionLock object is dropped
    }

    log::info!("Exiting wayrustlock");
    Ok(())
}

fn run_demonstration_mode(
    _config: Config,
    lock_manager: Arc<Mutex<LockManager>>,
) -> Result<(), Box<dyn Error>> {
    use std::thread;
    use std::time::Duration;

    println!("wayrustlock - Wayland Screen Locker (Demonstration Mode)");
    println!("=========================================================");
    println!("Note: Running in demonstration mode because:");
    println!("1. Not running on Wayland compositor, OR");
    println!("2. ext-session-lock-v1 protocol not available, OR");
    println!("3. Wayland connection failed");
    println!();
    println!("To actually lock your screen, ensure:");
    println!("1. You're running on sway, niri, or another Wayland compositor");
    println!("2. The compositor supports ext-session-lock-v1 protocol");
    println!("3. You have the required Wayland libraries installed");
    println!();

    {
        let mut lock_manager = lock_manager.lock().unwrap();
        lock_manager.initialize_lock_surfaces();
        log::info!(
            "Initialized {} lock surface(s)",
            lock_manager.surface_count()
        );
    }

    log::info!("Demonstration mode: Press Ctrl+C to exit");

    // Set up Ctrl+C handler for demonstration mode
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        log::warn!("Ctrl+C received - exiting demonstration mode");
        running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Failed to set Ctrl+C handler");

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));

        {
            let mut lock_manager = lock_manager.lock().unwrap();
            lock_manager.update();
        }
    }

    log::info!("Exiting demonstration mode");
    Ok(())
}
