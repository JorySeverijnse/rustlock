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
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    Flags, ZwlrScreencopyFrameV1,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;
use zeroize::Zeroizing;

use smithay_client_toolkit::reexports::calloop::{self, EventLoop, LoopHandle};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Modifiers},
        SeatHandler, SeatState,
    },
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};

static FILE_LOGGER: std::sync::LazyLock<std::sync::Mutex<Option<std::fs::File>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

fn setup_file_logging(_config: &Config) {
    let log_path = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()) + "/.rustlock.log";

    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
    {
        Ok(file) => {
            *FILE_LOGGER.lock().unwrap() = Some(file);
            eprintln!("Logging to: {}", log_path);
        }
        Err(e) => {
            eprintln!("Failed to open log file {}: {}", log_path, e);
        }
    }
}

fn write_to_file(msg: &str) {
    if let Ok(mut guard) = FILE_LOGGER.lock() {
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "{}", msg);
        }
    }
}

struct DualLogger;

impl log::Log for DualLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let msg = format!(
                "[{}] {}: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.args()
            );
            eprintln!("{}", msg);
            write_to_file(&msg);
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = FILE_LOGGER.lock() {
            if let Some(ref mut file) = *guard {
                let _ = file.flush();
            }
        }
    }
}

struct WaylandLock {
    loop_handle: LoopHandle<'static, Self>,
    lock_manager: Arc<Mutex<LockManager>>,
    config: Config,
    ctrlc_exit: Arc<std::sync::atomic::AtomicBool>,
    auth_tx: Option<calloop::channel::Sender<Zeroizing<String>>>,
    compositor_state: CompositorState,
    output_state: OutputState,
    registry_state: RegistryState,
    session_lock_state: SessionLockState,
    seat_state: SeatState,
    shm_state: Shm,
    pool: SlotPool,
    session_lock: Option<SessionLock>,
    lock_surfaces: Vec<SessionLockSurface>,
    outputs: Vec<WlOutput>,
    screenshot_frames: Vec<Option<ZwlrScreencopyFrameV1>>,
    captured_backgrounds: Vec<Option<cairo::ImageSurface>>,
    pending_screenshots: usize,
    exit: bool,
    unlocking: bool,
    screenshot_manager: Option<ScreenshotManager>,
    grace_until: Option<Instant>,
}

impl WaylandLock {
    fn handle_auth_result(&mut self, success: bool) {
        // Clear grace period on any auth result
        self.grace_until = None;

        if success {
            log::info!("✅ Authentication successful - unlocking session");
            if let Some(session_lock) = &self.session_lock {
                session_lock.unlock();
                self.unlocking = true;
                log::debug!("Unlock requested - waiting for compositor finished event");
            } else {
                log::error!("No session_lock available to unlock!");
                self.exit = true;
            }
        } else {
            log::warn!("❌ Authentication failed - wrong password");
            if let Ok(mut lock_manager) = self.lock_manager.lock() {
                for surface in &mut lock_manager.surfaces {
                    surface.show_wrong_password();
                }
            }
        }
    }

    fn get_output_dimensions(&self, output: &WlOutput) -> (i32, i32) {
        if let Some(info) = self.output_state.info(output) {
            if let Some(mode) = info.modes.first() {
                let (w, h) = mode.dimensions;
                return (w as i32, h as i32);
            }
        }
        (1920, 1080)
    }

    fn handle_key_event(&mut self, event: KeyEvent) {
        // Check if we're in the grace period (any key unlocks without password)
        if let Some(grace_until) = self.grace_until {
            if Instant::now() < grace_until {
                self.handle_auth_result(true);
                return;
            } else {
                self.grace_until = None;
            }
        }

        use crate::input::InputAction;
        use smithay_client_toolkit::seat::keyboard::Keysym;

        if event.keysym == Keysym::Return {
            log::info!("Enter pressed - submitting password");
            if let Ok(mut lock_manager) = self.lock_manager.lock() {
                let mut password = Zeroizing::new(String::new());
                for surface in &mut lock_manager.surfaces {
                    if let Some(InputAction::SubmitPassword(p)) =
                        surface.handle_key_event(event.clone())
                    {
                        password = p;
                    }
                }
                if !password.is_empty() {
                    if let Some(tx) = &self.auth_tx {
                        let _ = tx.send(password);
                    }
                }
            }
        } else {
            let action = self
                .lock_manager
                .lock()
                .map(|mut lm| lm.handle_key_event(event))
                .unwrap_or(None);

            if let Some(InputAction::TempScreenshot) = action {
                if let Ok(mut lm) = self.lock_manager.lock() {
                    lm.toggle_peek();
                }
            }
        }
    }
}

impl ProvidesRegistryState for WaylandLock {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

impl CompositorHandler for WaylandLock {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _time: u32,
    ) {
    }
    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
    }
}

impl OutputHandler for WaylandLock {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}
    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}
    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
    }
}

impl ShmHandler for WaylandLock {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl SessionLockHandler for WaylandLock {
    fn locked(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
        log::info!("Session LOCKED confirmed by compositor");
        self.grace_until = Some(Instant::now() + Duration::from_secs_f32(self.config.grace));
    }

    fn finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _lock: SessionLock) {
        log::info!("Session lock finished");
        self.exit = true;
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
        log::debug!("CONFIGURE callback: {}x{}", width, height);

        if let Ok(mut lock_manager) = self.lock_manager.lock() {
            if let Some(locked_surface) =
                lock_manager.find_surface_by_wayland_surface(session_lock_surface.wl_surface())
            {
                locked_surface.resize(width as i32, height as i32);
                locked_surface.set_configured();

                let output = locked_surface.output();
                let output_id = Proxy::id(output);
                let output_idx = self.outputs.iter().position(|o| Proxy::id(o) == output_id);

                if let Some(idx) = output_idx {
                    if let Some(bg) = self.captured_backgrounds.get(idx).and_then(|b| b.as_ref()) {
                        log::info!(
                            "Applying background for output {} (ID: {:?})",
                            idx,
                            output_id
                        );
                        locked_surface.set_background(bg.clone());
                    }
                }

                locked_surface.update();
                let _ = locked_surface.commit(&mut self.pool);
            }
        }
    }
}

impl KeyboardHandler for WaylandLock {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
    }
    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
    ) {
    }
    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
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
    }
    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wayland_client::protocol::wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        if capability == smithay_client_toolkit::seat::Capability::Keyboard {
            let _ = self.seat_state.get_keyboard_with_repeat(
                qh,
                &seat,
                None,
                self.loop_handle.clone(),
                Box::new(|_state, _kbd, _event| {}),
            );
        }
    }
    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wayland_client::protocol::wl_seat::WlSeat,
        _capability: smithay_client_toolkit::seat::Capability,
    ) {
    }
    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wayland_client::protocol::wl_seat::WlSeat,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for WaylandLock {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyManagerV1,
        _event: <ZwlrScreencopyManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, CaptureData> for WaylandLock {
    fn event(
        state: &mut Self,
        frame: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as Proxy>::Event,
        data: &CaptureData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event;
        match event {
            Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                let format = format.into_result().unwrap();
                let mut info = data.info.lock().unwrap();
                *info = Some(screenshot::BufferInfo {
                    width,
                    height,
                    stride,
                    format,
                });
                match state
                    .pool
                    .create_buffer(width as i32, height as i32, stride as i32, format)
                {
                    Ok((buffer, _canvas)) => {
                        frame.copy(buffer.wl_buffer());
                        *data.buffer.lock().unwrap() = Some(buffer);
                    }
                    Err(e) => log::error!("Screencopy: Buffer creation failed: {:?}", e),
                }
            }
            Event::Flags { flags } => {
                *data.flags.lock().unwrap() = Some(flags.into_result().unwrap());
            }
            Event::Ready { .. } => {
                log::info!("Screencopy: Ready for output {}", data.output_idx);
                if let Some(mgr) = &state.screenshot_manager {
                    let buffer = data.buffer.lock().unwrap().take();
                    let info = data.info.lock().unwrap().take();
                    let flags = data.flags.lock().unwrap().take();
                    if let (Some(buffer), Some(info), Some(flags)) = (buffer, info, flags) {
                        let handle = screenshot::ScreencopyBufferHandle {
                            buffer,
                            info,
                            y_invert: flags.contains(Flags::YInvert),
                        };
                        if let Ok(surface) = mgr.buffer_to_surface(handle, &mut state.pool) {
                            let mut ss = Screenshot::new(surface);
                            let _ = ss.apply_effects(&state.config);
                            if data.output_idx < state.captured_backgrounds.len() {
                                state.captured_backgrounds[data.output_idx] = Some(ss.into_inner());
                            }
                        }
                    }
                }
                state.pending_screenshots -= 1;
                frame.destroy();
            }
            Event::Failed => {
                log::warn!("Screencopy: Failed for output {}", data.output_idx);
                state.pending_screenshots -= 1;
                frame.destroy();
            }
            _ => {}
        }
    }
}

impl wayland_client::Dispatch<wayland_client::protocol::wl_registry::WlRegistry, ()>
    for WaylandLock
{
    fn event(
        _state: &mut Self,
        _proxy: &wayland_client::protocol::wl_registry::WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

smithay_client_toolkit::delegate_compositor!(WaylandLock);
smithay_client_toolkit::delegate_output!(WaylandLock);
smithay_client_toolkit::delegate_shm!(WaylandLock);
smithay_client_toolkit::delegate_seat!(WaylandLock);
smithay_client_toolkit::delegate_keyboard!(WaylandLock);
smithay_client_toolkit::delegate_registry!(WaylandLock);
smithay_client_toolkit::delegate_session_lock!(WaylandLock);
wayland_client::delegate_noop!(WaylandLock: ignore wayland_client::protocol::wl_buffer::WlBuffer);

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load();
    setup_file_logging(&config);
    static LOGGER: DualLogger = DualLogger;
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug))?;

    log::info!("Starting rustlock v{}", env!("CARGO_PKG_VERSION"));
    let lock_manager = Arc::new(Mutex::new(LockManager::new(config.clone())));
    let ctrlc_exit = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) =
        wayland_client::globals::registry_queue_init::<WaylandLock>(&conn)?;
    let qh: QueueHandle<WaylandLock> = event_queue.handle();

    let shm_state = Shm::bind(&globals, &qh).map_err(|_| "wl_shm not supported")?;
    let pool = SlotPool::new(1920 * 1080 * 4, &shm_state)?;

    let (auth_tx_actual, auth_feedback_rx_actual) = auth::create_and_run_auth_loop();
    let mut event_loop: EventLoop<WaylandLock> = EventLoop::try_new()?;

    let mut state = WaylandLock {
        loop_handle: event_loop.handle(),
        lock_manager: lock_manager.clone(),
        config: config.clone(),
        ctrlc_exit: ctrlc_exit.clone(),
        auth_tx: Some(auth_tx_actual),
        compositor_state: CompositorState::bind(&globals, &qh)?,
        output_state: OutputState::new(&globals, &qh),
        registry_state: RegistryState::new(&globals),
        session_lock_state: SessionLockState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        shm_state,
        pool,
        session_lock: None,
        lock_surfaces: Vec::new(),
        outputs: Vec::new(),
        screenshot_frames: Vec::new(),
        captured_backgrounds: Vec::new(),
        pending_screenshots: 0,
        exit: false,
        unlocking: false,
        screenshot_manager: ScreenshotManager::new(&globals, &qh).ok(),
        grace_until: None,
    };

    event_queue.blocking_dispatch(&mut state)?;

    let _wayland_source =
        WaylandSource::new(conn.clone(), event_queue).insert(event_loop.handle())?;

    event_loop
        .handle()
        .insert_source(auth_feedback_rx_actual, |event, _, state| {
            if let calloop::channel::Event::Msg(success) = event {
                state.handle_auth_result(success);
                if success {
                    state.lock_surfaces.clear();
                }
            }
        })?;

    let timer = calloop::timer::Timer::from_duration(Duration::from_millis(16));
    event_loop.handle().insert_source(timer, |_, _, state| {
        // Clear expired grace period
        if let Some(grace_until) = state.grace_until {
            if Instant::now() >= grace_until {
                state.grace_until = None;
            }
        }

        if state.unlocking {
            return calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(100));
        }

        if let Ok(mut lm) = state.lock_manager.lock() {
            lm.update();
            for surface in &mut lm.surfaces {
                let _ = surface.commit(&mut state.pool);
            }
        }
        if state.exit {
            calloop::timer::TimeoutAction::Drop
        } else {
            calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(16))
        }
    })?;

    if state.config.screenshots {
        state.outputs = state.output_state.outputs().collect();
        log::info!(
            "Capturing screenshots for {} outputs...",
            state.outputs.len()
        );
        state.screenshot_frames = vec![None; state.outputs.len()];
        state.captured_backgrounds = vec![None; state.outputs.len()];
        state.pending_screenshots = state.outputs.len();

        for (i, output) in state.outputs.iter().enumerate() {
            if let Some(mgr) = &state.screenshot_manager {
                let data = CaptureData::new(i);
                match mgr.capture_output(output, &qh, data) {
                    Ok(frame) => state.screenshot_frames[i] = Some(frame),
                    Err(e) => {
                        log::error!("Screencopy failed for output {}: {:?}", i, e);
                        state.pending_screenshots -= 1;
                    }
                }
            } else {
                state.pending_screenshots -= 1;
            }
        }

        let start = std::time::Instant::now();
        while state.pending_screenshots > 0 && start.elapsed() < Duration::from_secs(2) {
            event_loop.dispatch(Duration::from_millis(50), &mut state)?;
        }
        log::info!("Screenshot capture phase complete");
    }

    log::info!("Attempting to lock Wayland session...");
    let session_lock = state.session_lock_state.lock(&qh).map_err(|e| {
        log::error!("Lock failed: {}", e);
        e
    })?;

    let outputs_to_lock: Vec<WlOutput> = state.output_state.outputs().collect();
    for output in outputs_to_lock {
        let surface = state.compositor_state.create_surface(&qh);
        let (width, height) = state.get_output_dimensions(&output);
        let lock_surface = session_lock.create_lock_surface(surface.clone(), &output, &qh);
        state.lock_surfaces.push(lock_surface);
        if !state.outputs.contains(&output) {
            state.outputs.push(output.clone());
        }
        if let Ok(mut lm) = state.lock_manager.lock() {
            lm.add_surface(width, height, output.clone());
            let count = lm.surface_count();
            if let Some(ls) = lm.get_surface_mut(count - 1) {
                ls.set_wayland_surface(surface);
            }
        }
    }
    state.session_lock = Some(session_lock);

    while !state.exit && !state.ctrlc_exit.load(std::sync::atomic::Ordering::SeqCst) {
        event_loop.dispatch(Duration::from_millis(16), &mut state)?;
    }

    log::info!("Exiting rustlock");
    Ok(())
}
