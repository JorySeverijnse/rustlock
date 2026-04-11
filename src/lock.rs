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
        })
    }

    /// Set the configured state
    pub fn set_configured(&mut self) {
        log::debug!("LockedSurface: Configured, starting animation");
        self.configured = true;
        self.start_time = Instant::now();
    }

    /// Check if this surface matches the given Wayland surface
    pub fn matches_surface(&self, surface: &wl_surface::WlSurface) -> bool {
        use wayland_client::Proxy;
        self.wayland_surface
            .as_ref()
            .is_some_and(|ws| ws.id() == surface.id())
    }

    /// Update the surface state (called on each frame)
    pub fn update(&mut self) {
        self.input_handler.update();

        if !self.configured {
            return;
        }

        // Update fade animation
        if self.fade_alpha < 1.0 {
            let elapsed = self.start_time.elapsed();
            let fade_duration = std::time::Duration::from_secs_f32(self.config.fade_in);
            // Ease-in-out cubic function
            let t = (elapsed.as_secs_f64() / fade_duration.as_secs_f64()).clamp(0.0, 1.0);
            let eased_t = if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            };
            let new_alpha = eased_t.min(1.0);
            if (new_alpha - self.fade_alpha).abs() > 0.001 {
                self.fade_alpha = new_alpha;
                self.renderer.set_fade_alpha(self.fade_alpha);
            }
        }

        // Check if we should show/hide wrong password feedback
        if self.input_handler.should_show_wrong_password() && !self.wrong_password_shown {
            self.renderer.show_wrong_password();
            self.wrong_password_shown = true;
        } else if !self.input_handler.should_show_wrong_password() && self.wrong_password_shown {
            self.wrong_password_shown = false;
        }

        // Check if we should show/hide key highlight feedback
        if self.input_handler.should_show_key_highlight() && !self.key_highlight_shown {
            self.renderer.show_key_highlight();
            self.key_highlight_shown = true;
        } else if !self.input_handler.should_show_key_highlight() && self.key_highlight_shown {
            self.key_highlight_shown = false;
        }

        // Update caps lock state in renderer
        self.renderer.caps_lock = self.input_handler.caps_lock();

        // Set background if available and not already applied
        if !self.background_applied {
            if let Some(ref background) = self.background {
                log::info!("Applying background image to renderer");
                self.renderer.set_background(background.clone());
                self.background_applied = true;
            }
        }

        self.renderer
            .set_password_display(self.input_handler.password_length());
        self.renderer
            .set_cursor_position(self.input_handler.cursor_position());
        self.renderer.render();
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
    }

    pub fn show_wrong_password(&mut self) {
        self.input_handler.set_wrong_password_feedback();
    }

    pub fn handle_key_event(
        &mut self,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
    ) -> Option<InputAction> {
        let action = self
            .input_handler
            .handle_key_event(event.keysym, event.utf8, modifiers);

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
    }

    pub fn set_system_status(&mut self, status: SystemStatus) {
        self.renderer.system_status = status;
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

    pub fn update(&mut self) {
        for surface in &mut self.surfaces {
            surface.update();
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
                action = Some(a);
            }
        }
        action
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
