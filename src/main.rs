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
use screenshot::{CaptureData, Screenshot, ScreenshotManager};
use std::collections::HashSet;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Proxy;
use wayland_client::QueueHandle;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    Flags, ZwlrScreencopyFrameV1,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;
use zeroize::Zeroizing;

fn setup_file_logging() {
    let log_path =
        std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()) + "/.wayrustlock.log";

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "=== wayrustlock log ===");
        let _ = writeln!(file, "Started at: {:?}", std::time::SystemTime::now());
    }
    eprintln!("Logging to: {}", log_path);
}

fn log_to_file(msg: &str) {
    let log_path =
        std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()) + "/.wayrustlock.log";

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "[{:?}] {}", std::time::SystemTime::now(), msg);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    setup_file_logging();
    log_to_file("Program starting");

    let config = Config::load();

    if config.debug {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }

    log::info!("Starting wayrustlock v{}", env!("CARGO_PKG_VERSION"));
    log_to_file("Starting wayrustlock");
    log::info!("Attempting to lock Wayland session...");
    log_to_file("Attempting to lock Wayland session");
    log::warn!("WARNING: This is a screen locker. To kill it if stuck, use:");
    log::warn!("  Method 1: Switch to another TTY (Ctrl+Alt+F2) and kill the process");
    log::warn!("  Method 2: Use 'pkill -9 wayrustlock' from another terminal");
    log::warn!("  Method 3: Use 'killall wayrustlock'");
    log_to_file("Warnings printed");

    let lock_manager = Arc::new(Mutex::new(LockManager::new(config.clone())));
    let ctrlc_exit = Arc::new(std::sync::atomic::AtomicBool::new(false));

    if let Err(e) = lock_wayland_session(config.clone(), lock_manager.clone(), ctrlc_exit) {
        log::error!("Failed to lock Wayland session: {}", e);
        log_to_file(&format!("Failed to lock Wayland session: {}", e));
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
    use smithay_client_toolkit::reexports::calloop;

    use smithay_client_toolkit::{
        compositor::{CompositorHandler, CompositorState},
        output::{OutputHandler, OutputState},
        reexports::{
            calloop::{channel, timer, EventLoop, LoopHandle},
            calloop_wayland_source::WaylandSource,
        },
        registry::{ProvidesRegistryState, RegistryState},
        registry_handlers,
        seat::{
            keyboard::{KeyEvent, KeyboardHandler, Modifiers},
            pointer::PointerHandler,
            SeatHandler, SeatState,
        },
        session_lock::{
            SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
            SessionLockSurfaceConfigure,
        },
        shm::{slot::SlotPool, Shm, ShmHandler},
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
        globals: GlobalList,
        session_lock_state: SessionLockState,
        seat_state: SeatState,
        shm_state: Shm,
        pool: SlotPool,
        session_lock: Option<SessionLock>,
        lock_surfaces: Vec<SessionLockSurface>,
        lock_manager: Arc<Mutex<LockManager>>,
        config: Config,
        ctrlc_exit: Arc<std::sync::atomic::AtomicBool>,
        exit: bool,
        auth_tx: Option<channel::Sender<Zeroizing<String>>>,
        unlocking: bool,
        screenshot_manager: Option<ScreenshotManager>,
        screenshot_frames: Vec<Option<ZwlrScreencopyFrameV1>>,
        captured_backgrounds: Vec<Option<cairo::ImageSurface>>,
        locked_outputs: HashSet<WlOutput>,
        outputs: Vec<WlOutput>,
        lock_surface_outputs: Vec<WlOutput>,
    }

    impl SessionLockHandler for WaylandLock {
        fn locked(
            &mut self,
            _conn: &Connection,
            qh: &QueueHandle<Self>,
            mut session_lock: SessionLock,
        ) {
            log::info!("===========================================");
            log::info!("Session LOCKED SUCCESSFULLY!");
            log::info!("===========================================");
            log_to_file("Session LOCKED - creating surfaces");
            eprintln!("SESSION LOCKED - creating lock surfaces");

            let lock_ref = &mut session_lock;

            let outputs: Vec<WlOutput> = self.output_state.outputs().collect();
            let output_count = outputs.len();

            self.outputs = outputs.clone();
            self.screenshot_frames = vec![None; output_count];
            self.captured_backgrounds = vec![None; output_count];

            for output in &outputs {
                log_to_file(&format!("Creating lock surface for output"));
                let surface = self.compositor_state.create_surface(qh);
                let lock_surface = lock_ref.create_lock_surface(surface, output, qh);
                self.lock_surfaces.push(lock_surface);
                self.lock_surface_outputs.push(output.clone());
                self.locked_outputs.insert(output.clone());
            }

            log_to_file(&format!(
                "Created {} lock surfaces",
                self.lock_surfaces.len()
            ));

            self.session_lock = Some(session_lock);

            if self.config.screenshots {
                log::info!("Initializing screenshot capture...");
                match ScreenshotManager::new(&self.globals, qh) {
                    Ok(manager) => {
                        self.screenshot_manager = Some(manager);
                        log::info!("Screenshot manager initialized");

                        for (idx, output) in self.outputs.iter().enumerate() {
                            log::info!("Starting screenshot capture for output {}...", idx);
                            match self.screenshot_manager.as_mut().unwrap().capture_output(
                                output,
                                qh,
                                CaptureData::new(idx),
                            ) {
                                Ok(frame) => {
                                    self.screenshot_frames[idx] = Some(frame);
                                    log::debug!("Screenshot capture initiated for output {}", idx);
                                }
                                Err(e) => {
                                    log::error!("Failed to capture output {}: {}", idx, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Screenshot capture not available: {}", e);
                        self.screenshot_manager = None;
                    }
                }
            }
        }

        fn finished(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _session_lock: SessionLock,
        ) {
            if self.unlocking {
                log::info!("✅ Session successfully unlocked - compositor confirmed");
                log_to_file("✅ Session successfully unlocked - compositor confirmed");
                self.unlocking = false;
            } else {
                log::warn!("Session finished without unlock request - lock denied or cancelled");
                log_to_file("Session finished without unlock request");
            }
            self.session_lock = None;
            self.exit = true;
            log_to_file("Setting exit=true from finished callback");
        }

        fn configure(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            session_lock_surface: SessionLockSurface,
            configure: SessionLockSurfaceConfigure,
            _serial: u32,
        ) {
            let (width, height) = configure.new_size;
            log::info!("===========================================");
            log::info!("CONFIGURE callback: {}x{}", width, height);
            log::info!("===========================================");
            log_to_file(&format!("CONFIGURE: {}x{}", width, height));
            eprintln!("CONFIGURE: {}x{}", width, height);

            let surface_added = if let Ok(mut lock_manager) = self.lock_manager.lock() {
                let surface_wl_id = session_lock_surface.wl_surface().id();
                let output_idx = self
                    .lock_surfaces
                    .iter()
                    .position(|s| s.wl_surface().id() == surface_wl_id);

                if let Some(idx) = output_idx {
                    if idx < self.lock_surface_outputs.len() {
                        let output = &self.lock_surface_outputs[idx];
                        let (width, height) = configure.new_size;
                        lock_manager.add_surface(width as i32, height as i32, output.clone())
                    } else {
                        log::error!(
                            "lock_surface_outputs index out of bounds: {} vs {}",
                            idx,
                            self.lock_surface_outputs.len()
                        );
                        false
                    }
                } else {
                    log::error!("Could not find lock surface in self.lock_surfaces");
                    false
                }
            } else {
                false
            };

            if !surface_added {
                log::error!("Failed to add surface to lock manager");
                session_lock_surface.wl_surface().commit();
                return;
            }

            log::debug!("Surface added to lock manager");

            if let Ok(mut lock_manager) = self.lock_manager.lock() {
                let surface_count = lock_manager.surface_count();
                if surface_count == 0 {
                    log::error!("No surfaces in lock manager");
                    session_lock_surface.wl_surface().commit();
                    return;
                }

                if let Some(locked_surface) = lock_manager.get_surface_mut(surface_count - 1) {
                    locked_surface.set_wayland_surface(session_lock_surface.wl_surface().clone());

                    if self.config.screenshots {
                        let output = locked_surface.output();
                        let output_idx = self.outputs.iter().position(|o| o.id() == output.id());

                        if let Some(idx) = output_idx {
                            if let Some(background) = self
                                .captured_backgrounds
                                .get_mut(idx)
                                .and_then(|b| b.take())
                            {
                                log::info!(
                                    "Using captured screenshot background for output {}",
                                    idx
                                );
                                locked_surface.set_background(background);
                            } else {
                                log::warn!("No screenshot available for output {}, will render without background", idx);
                            }
                        } else {
                            log::warn!(
                                "Output not found in self.outputs, cannot assign screenshot"
                            );
                        }
                    }
                }

                if let Some(locked_surface) = lock_manager.get_surface_mut(surface_count - 1) {
                    locked_surface.set_wayland_surface(session_lock_surface.wl_surface().clone());

                    // Render the surface
                    locked_surface.renderer.render();

                    match locked_surface.renderer.get_pixel_data() {
                        Ok(pixel_data) => {
                            let (_renderer_width, _renderer_height, stride) =
                                locked_surface.renderer.surface_info();
                            let actual_width = width as i32;
                            let actual_height = height as i32;
                            let actual_stride = stride;

                            // Create a buffer from the pool
                            match self.pool.create_buffer(
                                actual_width,
                                actual_height,
                                actual_stride,
                                wayland_client::protocol::wl_shm::Format::Argb8888,
                            ) {
                                Ok((buffer, canvas)) => {
                                    // Copy pixel data to the buffer
                                    let data_len = canvas.len();
                                    let copy_len = pixel_data.len().min(data_len);
                                    canvas[..copy_len].copy_from_slice(&pixel_data[..copy_len]);

                                    // Damage the entire surface
                                    session_lock_surface.wl_surface().damage_buffer(
                                        0,
                                        0,
                                        actual_width,
                                        actual_height,
                                    );

                                    // Attach buffer and commit
                                    if let Err(e) =
                                        buffer.attach_to(session_lock_surface.wl_surface())
                                    {
                                        log::error!("Failed to attach buffer: {:?}", e);
                                    } else {
                                        session_lock_surface.wl_surface().commit();
                                        log::debug!(
                                            "Surface committed with buffer {}x{}",
                                            actual_width,
                                            actual_height
                                        );
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to create buffer from pool: {:?}", e);
                                    session_lock_surface.wl_surface().commit();
                                }
                            }
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

                if let Some(_locked_surface) = lock_manager.find_surface_by_wayland_surface(surface)
                {
                    // For now, just log that we would update the surface
                    // TODO: Implement proper buffer creation for animation
                    log::debug!("Frame update for surface");
                }
            }
        }

        fn surface_enter(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _surface: &wl_surface::WlSurface,
            _output: &wl_output::WlOutput,
        ) {
            log::debug!("Surface entered");
        }

        fn surface_leave(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _surface: &wl_surface::WlSurface,
            _output: &wl_output::WlOutput,
        ) {
            log::debug!("Surface left");
        }
    }

    impl OutputHandler for WaylandLock {
        fn output_state(&mut self) -> &mut OutputState {
            &mut self.output_state
        }

        fn new_output(
            &mut self,
            _conn: &Connection,
            qh: &QueueHandle<Self>,
            output: wl_output::WlOutput,
        ) {
            log::info!("New output detected: creating lock surface");
            if self.locked_outputs.contains(&output) {
                log::debug!("Output already has lock surface, ignoring");
                return;
            }
            if let Some(ref session_lock) = self.session_lock {
                let surface = self.compositor_state.create_surface(qh);
                let _lock_surface = session_lock.create_lock_surface(surface, &output, qh);
                self.locked_outputs.insert(output.clone());
                self.lock_surface_outputs.push(output.clone());
            }
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
            log::info!("Keyboard entered surface (serial: {})", _serial);
            log_to_file(&format!("Keyboard entered surface (serial: {})", _serial));
        }

        fn leave(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _surface: &wl_surface::WlSurface,
            _serial: u32,
        ) {
            log::info!("Keyboard left surface (serial: {})", _serial);
            log_to_file(&format!("Keyboard left surface (serial: {})", _serial));
        }

        fn press_key(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _serial: u32,
            event: KeyEvent,
        ) {
            log::info!(
                "Key pressed (serial: {}, keysym: {:?})",
                _serial,
                event.keysym
            );
            log_to_file(&format!("Key pressed: keysym={:?}", event.keysym));
            self.handle_key_event(event);
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
            _layout: u32,
        ) {
            log::debug!("Modifiers updated (serial: {})", _serial);
        }

        fn update_keymap(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
            _keymap: smithay_client_toolkit::seat::keyboard::Keymap<'_>,
        ) {
            log::info!("Keymap updated");
            log_to_file("Keymap updated");
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
        fn handle_key_event(&mut self, event: KeyEvent) {
            use crate::input::InputAction;
            use smithay_client_toolkit::seat::keyboard::Keysym;

            log_to_file(&format!(
                "handle_key_event called: keysym={:?}",
                event.keysym
            ));
            log::debug!("Key event: {:?}", event.keysym);

            if event.keysym == Keysym::Return {
                log::info!("Enter pressed - submitting password for authentication");
                log_to_file("ENTER pressed - submitting password");

                if let Ok(mut lock_manager) = self.lock_manager.lock() {
                    if let Some(InputAction::SubmitPassword(password)) =
                        lock_manager.handle_key_event(event.clone())
                    {
                        // Send password to auth thread for processing
                        if let Some(tx) = &self.auth_tx {
                            let _ = tx.send(password);
                        }
                        log::debug!("Password submitted for authentication");
                    }
                }
            } else {
                // Distribute to all surfaces (BackSpace, characters, etc.)
                let _ = self
                    .lock_manager
                    .lock()
                    .map(|mut lm| lm.handle_key_event(event));
            }
        }

        /// Handle authentication result from the auth thread
        fn handle_auth_result(&mut self, success: bool) {
            if success {
                log::info!("✅ Authentication successful - unlocking session");
                log_to_file("✅ Authentication successful - unlocking session");

                // Call unlock on the session_lock, but keep it to receive finished event
                if let Some(session_lock) = &self.session_lock {
                    session_lock.unlock();
                    self.unlocking = true;
                    log_to_file("Unlock requested - waiting for finished event");
                } else {
                    log::error!("No session_lock available to unlock!");
                }
            } else {
                log::warn!("❌ Authentication failed - wrong password");
                log_to_file("❌ Authentication failed - wrong password");
                // Show wrong password feedback on all surfaces
                if let Ok(mut lock_manager) = self.lock_manager.lock() {
                    for surface in &mut lock_manager.surfaces {
                        surface.show_wrong_password();
                    }
                }
            }
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
            qh: &QueueHandle<Self>,
            seat: wayland_client::protocol::wl_seat::WlSeat,
            capability: smithay_client_toolkit::seat::Capability,
        ) {
            log::info!("New capability: {:?}", capability);
            log_to_file(&format!("New capability: {:?}", capability));

            if capability == smithay_client_toolkit::seat::Capability::Keyboard {
                log::info!("Setting up keyboard");
                log_to_file("Setting up keyboard - trying to get keyboard");

                match self.seat_state.get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|_state, _wl_kbd, event| {
                        log::info!("Keyboard repeat event: {:?}", event);
                        log_to_file(&format!("Keyboard repeat: {:?}", event));
                    }),
                ) {
                    Ok(_keyboard) => {
                        log::info!("Keyboard created successfully");
                        log_to_file("Keyboard created successfully");
                    }
                    Err(e) => {
                        log::error!("Failed to create keyboard: {:?}", e);
                        log_to_file(&format!("Failed to create keyboard: {:?}", e));
                    }
                }
            }
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

    impl wayland_client::Dispatch<ZwlrScreencopyManagerV1, ()> for WaylandLock {
        fn event(
            state: &mut Self,
            _manager: &ZwlrScreencopyManagerV1,
            _event: <ZwlrScreencopyManagerV1 as wayland_client::Proxy>::Event,
            _data: &(),
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
        ) {
            // No events expected from the manager
        }
    }

    impl wayland_client::Dispatch<ZwlrScreencopyFrameV1, CaptureData> for WaylandLock {
        fn event(
            state: &mut Self,
            frame: &ZwlrScreencopyFrameV1,
            event: <ZwlrScreencopyFrameV1 as wayland_client::Proxy>::Event,
            _data: &CaptureData,
            _conn: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            let output_idx = _data.output_idx;
            log::debug!("Screencopy event for output {}: {:?}", output_idx, event);

            match event {
                wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event::Buffer {
                    format,
                    width,
                    height,
                    stride,
                } => {
                    log::debug!("Buffer event: {}x{}, stride={}, format={:?}", width, height, stride, format);

                    // Extract format value from the compositor's Buffer event
                    let format_value = match format {
                        wayland_client::WEnum::Value(f) => f,
                        wayland_client::WEnum::Unknown(_) => {
                            log::error!("Unknown format value");
                            return;
                        }
                    };

                    // Store buffer info including the actual format
                    if let Ok(mut info_guard) = _data.info.lock() {
                        *info_guard = Some(screenshot::BufferInfo {
                            width,
                            height,
                            stride,
                            format: format_value,
                        });
                    }

                    // Create SHM buffer with the EXACT format the compositor specified
                    if let Ok(mut buffer_guard) = _data.buffer.lock() {
                        if let Ok((buf, _canvas)) = state.pool.create_buffer(
                            i32::try_from(width).unwrap(),
                            i32::try_from(height).unwrap(),
                            i32::try_from(stride).unwrap(),
                            format_value,
                        ) {
                            // Store the buffer for later use in Ready event
                            *buffer_guard = Some(buf);
                            log::debug!("Created SHM buffer for screencopy with format {:?}", format_value);
                            // Immediately send copy request (protocol requires this after buffer creation)
                            if let Some(buffer) = buffer_guard.as_ref() {
                                frame.copy(buffer.wl_buffer());
                                log::debug!("Sent copy request for screenshot");
                            }
                        } else {
                            log::error!("Failed to create SHM buffer");
                        }
                    }
                }
                wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event::Flags { flags } => {
                    log::debug!("Flags: {:?}", flags);
                    if let Ok(mut flags_guard) = _data.flags.lock() {
                        let flags_value = match flags {
                            wayland_client::WEnum::Value(f) => f,
                            wayland_client::WEnum::Unknown(_) => {
                                log::warn!("Unknown flags value");
                                return;
                            }
                        };
                        *flags_guard = Some(flags_value);
                    }
                }
                wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event::Ready { 
                    tv_sec_hi: _, 
                    tv_sec_lo: _, 
                    tv_nsec: _ 
                } => {
                    log::info!("Ready event: processing screenshot for output {}", output_idx);

                    // Extract all data from CaptureData
                    let info_opt = _data.info.lock().ok().and_then(|i| i.clone());
                    let buffer_opt = _data.buffer.lock().ok().and_then(|mut b| b.take());
                    let flags_opt = _data.flags.lock().ok().and_then(|f| f.clone());

                    if let (Some(info), Some(buffer), Some(flags)) = (info_opt, buffer_opt, flags_opt) {
                        // Determine if Y-inversion is needed (Y_INVERT flag is bit 0)
                        let y_invert = flags.bits() & 1 != 0;

                        let handle = screenshot::ScreencopyBufferHandle {
                            buffer,
                            info: screenshot::BufferInfo {
                                width: info.width,
                                height: info.height,
                                stride: info.stride,
                                format: info.format,
                            },
                            y_invert,
                        };

                        match state.screenshot_manager.as_ref().unwrap().buffer_to_surface(handle, &mut state.pool) {
                            Ok(surface) => {
                                // Apply effects if configured
                                let mut screenshot = Screenshot::new(surface);
                                if let Err(e) = screenshot.apply_effects(&state.config) {
                                    log::error!("Failed to apply effects: {}", e);
                                }
                                let surface = screenshot.into_inner();

                                // Save screenshot to file for debugging if debug mode enabled
                                if state.config.debug {
                                    let path = format!("/tmp/wayrustlock_output{}.png", output_idx);
                                            match std::fs::File::create(&path) {
                                                Ok(mut file) => {
                                                    if let Err(e) = surface.write_to_png(&mut file) {
                                                        log::error!("Failed to write PNG to {}: {}", path, e);
                                                    } else {
                                                        log::info!("Saved screenshot to {}", path);
                                                    }
                                                }
                                                Err(e) => {
                                                    log::error!("Failed to create file {}: {}", path, e);
                                                }
                                            }
                                }

                                if output_idx < state.outputs.len() {
                                    let output = &state.outputs[output_idx];
                                    if let Ok(mut lock_manager) = state.lock_manager.lock() {
                                        if let Some(locked_surface) = lock_manager.find_surface_by_output(output) {
                                            log::debug!("Setting background directly on lock surface for output {}", output_idx);
                                            locked_surface.set_background(surface);
                                        } else {
                                            log::debug!("Storing background in captured_backgrounds for output {}", output_idx);
                                            state.captured_backgrounds[output_idx] = Some(surface);
                                        }
                                    } else {
                                        log::debug!("Failed to lock manager, storing background");
                                        state.captured_backgrounds[output_idx] = Some(surface);
                                    }
                                } else {
                                    log::error!("output_idx {} out of bounds (outputs len {})", output_idx, state.outputs.len());
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to convert buffer to surface: {}", e);
                            }
                        }
                    } else {
                        log::error!("Missing buffer data for Ready event");
                    }

                    // Clean up frame
                    frame.destroy();
                }
                wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event::Failed => {
                    log::error!("Screenshot capture failed for output {}", output_idx);
                    frame.destroy();
                }
                _ => {}
            }
        }
    }

    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init(&conn)?;
    let qh: QueueHandle<WaylandLock> = event_queue.handle();
    let mut event_loop: EventLoop<WaylandLock> = EventLoop::try_new()?;

    // Create the authentication loop in a separate thread
    let (auth_tx, auth_rx) = auth::create_and_run_auth_loop();

    let mut state = WaylandLock {
        loop_handle: event_loop.handle(),
        conn: conn.clone(),
        lock_manager,
        config,
        ctrlc_exit: ctrlc_exit.clone(),
        auth_tx: Some(auth_tx),
        compositor_state: CompositorState::bind(&globals, &qh)?,
        output_state: OutputState::new(&globals, &qh),
        registry_state: RegistryState::new(&globals),
        session_lock_state: SessionLockState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        shm_state: Shm::bind(&globals, &qh).map_err(|_| "wl_shm protocol not supported")?,
        pool: SlotPool::new(
            256 * 1024 * 1024, // 256 MB pool to support high-resolution displays and screenshots
            &Shm::bind(&globals, &qh).map_err(|_| "wl_shm protocol not supported")?,
        )
        .map_err(|e| format!("Failed to create slot pool: {:?}", e))?,
        globals,
        session_lock: None,
        lock_surfaces: Vec::new(),
        exit: false,
        unlocking: false,
        screenshot_manager: None,
        screenshot_frames: Vec::new(),
        captured_backgrounds: Vec::new(),
        locked_outputs: HashSet::new(),
        outputs: Vec::new(),
        lock_surface_outputs: Vec::new(),
    };

    state.session_lock = Some(
        state
            .session_lock_state
            .lock(&qh)
            .map_err(|_| "ext-session-lock-v1 protocol not supported by compositor")?,
    );

    log::info!("Session lock requested, waiting for compositor...");
    log_to_file("Session lock requested");
    eprintln!("Session lock requested - running event loop");

    WaylandSource::new(conn, event_queue).insert(event_loop.handle())?;

    // Insert the authentication channel into the event loop
    event_loop.handle().insert_source(
        auth_rx,
        |event: channel::Event<bool>, _metadata, wayland_lock| {
            if let channel::Event::Msg(result) = event {
                wayland_lock.handle_auth_result(result);
            }
        },
    )?;

    // Insert a timer to update and commit surfaces at 60fps
    let render_timer = timer::Timer::from_duration(Duration::from_millis(16));
    event_loop.handle().insert_source(
        render_timer,
        |_event, _metadata, state: &mut WaylandLock| {
            // Update lock manager (renders to Cairo surfaces)
            if let Ok(mut lock_manager) = state.lock_manager.lock() {
                lock_manager.update();
                // Commit each surface that has a Wayland surface
                for surface in lock_manager.surfaces.iter_mut() {
                    if let Some(_wl_surface) = surface.wayland_surface() {
                        if let Err(e) = surface.commit(&mut state.pool) {
                            log::error!("Failed to commit surface: {:?}", e);
                        }
                    }
                }
            }
            // Reschedule timer to fire again in 16ms (60fps)
            calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(16))
        },
    )?;

    log_to_file("Starting event loop");
    eprintln!("Starting event loop - press Enter to unlock");

    while !state.exit && !state.ctrlc_exit.load(std::sync::atomic::Ordering::SeqCst) {
        event_loop.dispatch(Duration::from_millis(16), &mut state)?;
    }

    log_to_file("Event loop exited");
    log::info!("Event loop exited, exit={}", state.exit);

    log_to_file("Function completed");
    Ok(())
}

fn run_demonstration_mode(
    _config: Config,
    lock_manager: Arc<Mutex<LockManager>>,
) -> Result<(), Box<dyn Error>> {
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
        log::info!(
            "Initialized {} lock surface(s)",
            lock_manager.surface_count()
        );
    }

    log::info!("Demonstration mode: Press Ctrl+C to exit");

    // For demonstration mode, just run for a short time then exit
    let _running = Arc::new(std::sync::atomic::AtomicBool::new(true));

    std::thread::sleep(Duration::from_secs(30));

    log::info!("Exiting demonstration mode");
    Ok(())
}
