use cairo::ImageSurface;
use std::error::Error;
use std::time::Instant;
use wayland_client::protocol::{wl_output, wl_shm, wl_surface};

use crate::config::Config;
use crate::input::{InputAction, InputHandler};
use crate::render::Renderer;
use crate::system::SystemStatus;
use smithay_client_toolkit::seat::keyboard::KeyEvent;
use smithay_client_toolkit::shm::slot::SlotPool;

/// Manages a locked surface for a single output
pub struct LockedSurface {
    config: Config,
    pub renderer: Renderer,
    input_handler: InputHandler,
    background: Option<ImageSurface>,
    background_applied: bool,
    fade_alpha: f64,
    wrong_password_shown: bool,
    key_highlight_shown: bool,
    start_time: Instant,
    wayland_surface: Option<wl_surface::WlSurface>,
    output: wl_output::WlOutput,
    configured: bool,
    /// Set whenever rendered state changes (keystroke, status, clock minute,
    /// animation step). update() renders only when this is set or an animation
    /// is in flight, so an idle lock screen does no per-frame cairo work.
    dirty: bool,
    /// Last clock minute (unix-minute) we rendered, to detect %H:%M rollover.
    last_minute: i64,
    ctrl_held: bool,
    /// True while the pointer button is held on the indicator ring (hold-to-peek).
    /// Cleared on button release, matching Ctrl-hold transient behavior.
    peek_toggled: bool,
}

impl LockedSurface {
    /// Create a new locked surface for an output
    pub fn new(
        width: i32,
        height: i32,
        config: &Config,
        output: wl_output::WlOutput,
    ) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }

        let renderer = Renderer::new(width, height, config.clone());
        let input_handler = InputHandler::new(config.clone());

        Some(Self {
            config: config.clone(),
            renderer,
            input_handler,
            background: None,
            background_applied: false,
            fade_alpha: 0.0,
            wrong_password_shown: false,
            key_highlight_shown: false,
            start_time: Instant::now(),
            wayland_surface: None,
            output,
            configured: false,
            dirty: true,
            last_minute: i64::MIN,
            ctrl_held: false,
            peek_toggled: false,
        })
    }

    /// Set the configured state
    pub fn set_configured(&mut self) {
        log::debug!("LockedSurface: Configured, starting animation");
        self.configured = true;
        self.start_time = Instant::now();
        self.dirty = true;
    }

    /// Check if this surface matches the given Wayland surface
    pub fn matches_surface(&self, surface: &wl_surface::WlSurface) -> bool {
        use wayland_client::Proxy;
        self.wayland_surface
            .as_ref()
            .is_some_and(|ws| ws.id() == surface.id())
    }

    /// Update the surface state (called on each frame). Returns `true` if the
    /// surface was re-rendered and therefore needs to be committed. An idle
    /// surface (no input, no animation, same clock minute) returns `false` and
    /// does no cairo work, which keeps a locked session near-zero CPU.
    pub fn update(&mut self) -> bool {
        if !self.configured {
            return false;
        }

        // Update fade animation
        if self.fade_alpha < 1.0 {
            let elapsed = self.start_time.elapsed();
            let fade_duration = std::time::Duration::from_secs_f32(self.config.fade_in);

            if fade_duration.is_zero() {
                self.fade_alpha = 1.0;
                self.renderer.set_fade_alpha(1.0);
                self.dirty = true;
            } else {
                // Ease-in-out cubic function
                let t = (elapsed.as_secs_f64() / fade_duration.as_secs_f64()).clamp(0.0, 1.0);
                let eased_t = if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                };
                let new_alpha = eased_t.min(1.0);
                if t >= 1.0 {
                    // The eased curve only approaches 1.0 asymptotically, and the
                    // 0.001 throttle below suppresses the tiny final steps — which
                    // would leave fade_alpha stuck just under 1.0 forever. Since
                    // `fade_alpha < 1.0` is our "still animating" signal, that would
                    // force a full render every frame. Snap to exactly 1.0 once the
                    // fade duration has elapsed so the animation cleanly completes.
                    if self.fade_alpha != 1.0 {
                        self.fade_alpha = 1.0;
                        self.renderer.set_fade_alpha(1.0);
                        self.dirty = true;
                    }
                } else if (new_alpha - self.fade_alpha).abs() > 0.001 {
                    self.fade_alpha = new_alpha;
                    self.renderer.set_fade_alpha(self.fade_alpha);
                    self.dirty = true;
                }
            }
        }

        // Check if we should show/hide wrong password feedback
        if self.input_handler.should_show_wrong_password() && !self.wrong_password_shown {
            self.renderer.show_wrong_password();
            self.wrong_password_shown = true;
            self.dirty = true;
        } else if !self.input_handler.should_show_wrong_password() && self.wrong_password_shown {
            self.wrong_password_shown = false;
        }

        // Check if we should show/hide key highlight feedback
        if self.input_handler.should_show_key_highlight() && !self.key_highlight_shown {
            self.renderer.show_key_highlight();
            self.key_highlight_shown = true;
            self.dirty = true;
        } else if !self.input_handler.should_show_key_highlight() && self.key_highlight_shown {
            self.key_highlight_shown = false;
        }

        // Update caps lock state in renderer
        if self.renderer.caps_lock != self.input_handler.caps_lock() {
            self.renderer.caps_lock = self.input_handler.caps_lock();
            self.dirty = true;
        }

        // Set background if available and not already applied
        if !self.background_applied {
            if let Some(ref background) = self.background {
                log::debug!("Applying background image to renderer");
                self.renderer.set_background(background.clone());
                self.background_applied = true;
                self.dirty = true;
            }
        }

        // The clock displays %H:%M, so it only needs a redraw once per minute.
        if self.config.clock {
            let minute = chrono::Local::now().timestamp().div_euclid(60);
            if self.last_minute != minute {
                self.last_minute = minute;
                self.dirty = true;
            }
        }

        // Keep emitting frames while an animation is in flight so it can run to
        // completion even though no new event arrives.
        let animating = self.fade_alpha < 1.0 || self.renderer.is_animating();

        if !self.dirty && !animating {
            return false;
        }

        if !self.config.hide_password {
            let buf = self.input_handler.password_buffer();
            let length = self.input_handler.password_length();
            if self.ctrl_held || self.peek_toggled {
                self.renderer.peek_password(buf.as_str());
            } else {
                self.renderer.set_password_display(length);
            }
        }
        self.renderer
            .set_cursor_position(self.input_handler.cursor_position());
        self.renderer.render();
        self.dirty = false;
        true
    }

    /// Commit the rendered frame to the Wayland surface
    pub fn commit(&self, pool: &mut SlotPool) -> Result<(), Box<dyn Error>> {
        if !self.configured {
            return Ok(());
        }

        let pixel_data = self.renderer.get_pixel_data()?;
        let (width, height, stride) = self.renderer.surface_info();

        let (buffer, canvas) =
            pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888)?;

        let copy_len = pixel_data.len().min(canvas.len());
        canvas[..copy_len].copy_from_slice(&pixel_data[..copy_len]);

        if let Some(wl_surface) = &self.wayland_surface {
            buffer.attach_to(wl_surface)?;
            wl_surface.damage_buffer(0, 0, width, height);
            wl_surface.commit();
        }

        Ok(())
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        self.renderer.resize(width, height);
        self.background_applied = false;
        self.dirty = true;
    }

    pub fn show_wrong_password(&mut self) {
        self.renderer.clear_verifying();
        self.input_handler.set_wrong_password_feedback();
        self.wrong_password_shown = false;
        self.dirty = true;
    }

    pub fn show_verifying(&mut self) {
        self.renderer.show_verifying();
        self.dirty = true;
    }

    pub fn handle_key_event(
        &mut self,
        event: KeyEvent,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
    ) -> Option<InputAction> {
        self.ctrl_held = modifiers.ctrl;

        let action = self
            .input_handler
            .handle_key_event(event.keysym, event.utf8, modifiers);

        // Any key event may change the password display, cursor or caps state,
        // so request a redraw on the next update().
        self.dirty = true;

        match action {
            InputAction::PasswordChanged => {
                self.input_handler.set_key_highlight();
                self.key_highlight_shown = false;
            }
            InputAction::PasswordCleared => {
                self.renderer.show_cleared_feedback();
            }
            InputAction::SubmitPassword(_) => {
                self.input_handler.set_key_highlight();
                self.key_highlight_shown = false;
            }
            _ => {}
        }

        Some(action)
    }

    pub fn set_wayland_surface(&mut self, surface: wl_surface::WlSurface) {
        self.wayland_surface = Some(surface);
    }

    pub fn output(&self) -> &wl_output::WlOutput {
        &self.output
    }

    pub fn set_background(&mut self, surface: ImageSurface) {
        self.background = Some(surface);
        self.background_applied = false;
        self.dirty = true;
    }

    pub fn set_system_status(&mut self, status: SystemStatus) {
        if self.renderer.system_status != status {
            self.renderer.system_status = status;
            self.dirty = true;
        }
    }

    pub fn set_ctrl_held(&mut self, held: bool) {
        if self.ctrl_held != held {
            self.ctrl_held = held;
            self.dirty = true;
        }
    }

    /// Set peek mode on/off. Called when the user presses (held=true) or
    /// releases (held=false) the mouse button on the indicator ring. Works
    /// like Ctrl-hold — peek only lasts while the button is held.
    pub fn set_peek_held(&mut self, held: bool) {
        self.peek_toggled = held;
        log::debug!("set_peek_held: peek_toggled = {}", self.peek_toggled);
        self.dirty = true;
    }
}

pub struct LockManager {
    pub surfaces: Vec<LockedSurface>,
    config: Config,
}

impl LockManager {
    pub fn new(config: Config) -> Self {
        Self {
            surfaces: Vec::new(),
            config,
        }
    }

    pub fn add_surface(&mut self, width: i32, height: i32, output: wl_output::WlOutput) -> bool {
        match LockedSurface::new(width, height, &self.config, output) {
            Some(surface) => {
                self.surfaces.push(surface);
                true
            }
            None => false,
        }
    }

    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }

    pub fn get_surface_mut(&mut self, index: usize) -> Option<&mut LockedSurface> {
        self.surfaces.get_mut(index)
    }

    pub fn find_surface_by_wayland_surface(
        &mut self,
        wayland_surface: &wl_surface::WlSurface,
    ) -> Option<&mut LockedSurface> {
        self.surfaces
            .iter_mut()
            .find(|surface| surface.matches_surface(wayland_surface))
    }

    pub fn handle_key_event(
        &mut self,
        event: KeyEvent,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
    ) -> Option<InputAction> {
        let mut action = None;
        for surface in &mut self.surfaces {
            if let Some(a) = surface.handle_key_event(event.clone(), modifiers) {
                if let crate::input::InputAction::SubmitPassword(p) = &a {
                    if !p.is_empty() {
                        return Some(a);
                    }
                } else {
                    action = Some(a);
                }
            }
        }
        action
    }

    pub fn set_ctrl_held(&mut self, held: bool) {
        for surface in &mut self.surfaces {
            surface.set_ctrl_held(held);
        }
    }

    pub fn remove_surface_by_output(&mut self, output: &wl_output::WlOutput) -> Option<usize> {
        use wayland_client::Proxy;
        let output_id = Proxy::id(output);
        let idx = self
            .surfaces
            .iter()
            .position(|s| Proxy::id(s.output()) == output_id)?;
        self.surfaces.remove(idx);
        Some(idx)
    }

    pub fn set_system_status(&mut self, status: SystemStatus) {
        for surface in &mut self.surfaces {
            surface.set_system_status(status.clone());
        }
    }
}
