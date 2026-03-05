use cairo::ImageSurface;
use std::error::Error;
use std::time::Instant;
use wayland_client::protocol::{wl_shm, wl_surface};

use crate::config::Config;
use crate::input::{InputAction, InputHandler};
use crate::render::Renderer;
use crate::screenshot::Screenshot;
use smithay_client_toolkit::shm::slot::SlotPool;

/// Manages a locked surface for a single output
pub struct LockedSurface {
    width: i32,
    height: i32,
    config: Config,
    pub renderer: Renderer,
    input_handler: InputHandler,
    background: Option<ImageSurface>,
    fade_alpha: f64,
    wrong_password_shown: bool,
    key_highlight_shown: bool,
    temp_screenshot_shown: bool,
    last_update: Instant,
    wayland_surface: Option<wl_surface::WlSurface>,
}

impl LockedSurface {
    /// Create a new locked surface for an output
    pub fn new(width: i32, height: i32, config: &Config) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }

        let renderer = Renderer::new(width, height, config.clone());
        let input_handler = InputHandler::new(config.clone());

        // Create background if screenshots are enabled
        let background = if config.screenshots {
            // For now, create a dummy screenshot with the output dimensions
            // In a real implementation, this would capture actual screenshots via Wayland
            let mut screenshot = Screenshot {
                width: width as u32,
                height: height as u32,
                data: vec![0u8; (width * height * 4) as usize],
            };

            // Fill with a dark gray color (similar to swaylock default)
            for i in 0..(screenshot.width * screenshot.height) as usize {
                let offset = i * 4;
                screenshot.data[offset] = 40; // R
                screenshot.data[offset + 1] = 44; // G
                screenshot.data[offset + 2] = 52; // B
                screenshot.data[offset + 3] = 255; // A
            }

            // Apply effects if configured
            if let Some((blur_radius, blur_times)) = config.effect_blur {
                screenshot.apply_blur(blur_radius, blur_times);
            }

            if let Some((vignette_base, vignette_factor)) = config.effect_vignette {
                screenshot.apply_vignette(vignette_base, vignette_factor);
            }

            Some(screenshot.as_image_surface())
        } else {
            None
        };

        Some(Self {
            width,
            height,
            config: config.clone(),
            renderer,
            input_handler,
            background,
            fade_alpha: 0.0,
            wrong_password_shown: false,
            key_highlight_shown: false,
            temp_screenshot_shown: false,
            last_update: Instant::now(),
            wayland_surface: None,
        })
    }

    /// Check if this surface matches the given Wayland surface
    pub fn matches_surface(&self, surface: &wl_surface::WlSurface) -> bool {
        use wayland_client::Proxy;
        self.wayland_surface
            .as_ref()
            .map_or(false, |ws| ws.id() == surface.id())
    }

    /// Update the surface state (called on each frame)
    pub fn update(&mut self) {
        // Update timers
        self.input_handler.update();

        // Update fade animation
        if self.fade_alpha < 1.0 {
            let elapsed = self.last_update.elapsed();
            let fade_duration = std::time::Duration::from_secs_f32(self.config.fade_in);
            self.fade_alpha = (elapsed.as_secs_f64() / fade_duration.as_secs_f64()).min(1.0);
            self.renderer.set_fade_alpha(self.fade_alpha);
        }

        // Update visual feedback
        if self.input_handler.should_show_wrong_password() && !self.wrong_password_shown {
            self.renderer.show_wrong_password();
            self.wrong_password_shown = true;
        } else if !self.input_handler.should_show_wrong_password() && self.wrong_password_shown {
            self.wrong_password_shown = false;
        }

        if self.input_handler.should_show_key_highlight() && !self.key_highlight_shown {
            self.renderer.show_key_highlight();
            self.key_highlight_shown = true;
        } else if !self.input_handler.should_show_key_highlight() && self.key_highlight_shown {
            self.key_highlight_shown = false;
        }

        // Handle temp screenshot (peek feature)
        if self.input_handler.should_show_temp_screenshot() && !self.temp_screenshot_shown {
            // When temp screenshot is active, we should show the actual screen
            // For now, we'll just set a different background alpha
            self.renderer.set_fade_alpha(0.3); // Semi-transparent
            self.temp_screenshot_shown = true;
        } else if !self.input_handler.should_show_temp_screenshot() && self.temp_screenshot_shown {
            // Restore normal fade alpha
            self.renderer.set_fade_alpha(self.fade_alpha);
            self.temp_screenshot_shown = false;
        }

        // Set background if available
        if let Some(ref background) = self.background {
            self.renderer.set_background(background.clone());
        }

        self.renderer
            .set_password_display(self.input_handler.get_display_password());

        // Render the frame
        self.renderer.render();

        self.last_update = Instant::now();
    }

    /// Commit the rendered frame to the Wayland surface
    pub fn commit(&self, pool: &mut SlotPool) -> Result<(), Box<dyn Error>> {
        // Get pixel data from renderer
        let pixel_data = self.renderer.get_pixel_data()?;
        let (width, height, stride) = self.renderer.surface_info();

        // Create buffer from pool
        let (buffer, mut canvas) =
            pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888)?;

        // Copy pixel data to buffer
        let copy_len = pixel_data.len().min(canvas.len());
        canvas[..copy_len].copy_from_slice(&pixel_data[..copy_len]);

        // Attach buffer to Wayland surface and commit
        if let Some(wl_surface) = &self.wayland_surface {
            buffer.attach_to(wl_surface)?;
            wl_surface.damage_buffer(0, 0, width, height);
            wl_surface.commit();
        }

        Ok(())
    }

    /// Handle resize event from Wayland
    pub fn resize(&mut self, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }

        self.width = width;
        self.height = height;
        self.renderer.resize(width, height);

        // TODO: Re-capture screenshot if screenshots are enabled
    }

    /// Set fade alpha for animation
    pub fn set_fade_alpha(&mut self, alpha: f64) {
        self.fade_alpha = alpha.clamp(0.0, 1.0);
        self.renderer.set_fade_alpha(self.fade_alpha);
    }

    /// Show wrong password feedback
    pub fn show_wrong_password(&mut self) {
        self.input_handler.set_wrong_password_feedback();
    }

    /// Show key highlight feedback
    pub fn show_key_highlight(&mut self) {
        self.input_handler.set_key_highlight();
    }

    /// Handle a key event from Wayland
    pub fn handle_key_event(
        &mut self,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) -> Option<InputAction> {
        // Convert to our input handler format
        // Note: KeyEvent has fields: time, raw_code, keysym, utf8
        // We need to determine state and modifiers from context (not available in this demo)
        // For demonstration, we'll assume key press with no modifiers
        let keysym = event.keysym;
        let state = wayland_client::protocol::wl_keyboard::KeyState::Pressed;
        let modifiers = smithay_client_toolkit::seat::keyboard::Modifiers::default();

        let action = self
            .input_handler
            .handle_key_event(keysym, state, modifiers);

        match action {
            InputAction::SubmitPassword(password) => {
                // Show key highlight for visual feedback
                self.show_key_highlight();
                Some(InputAction::SubmitPassword(password))
            }
            InputAction::Cancel => Some(InputAction::Cancel),
            InputAction::TempScreenshot => Some(InputAction::TempScreenshot),
            InputAction::PasswordChanged => Some(InputAction::PasswordChanged),
            InputAction::None => None,
        }
    }

    /// Authenticate a password using PAM
    pub fn authenticate_password(&self, password: zeroize::Zeroizing<String>) -> bool {
        // Create a simple PAM conversation that provides the password
        struct SimpleConversation {
            password: Option<zeroize::Zeroizing<String>>,
        }

        impl pam_client::ConversationHandler for SimpleConversation {
            fn init(&mut self, _default_user: Option<impl AsRef<str>>) {}

            fn prompt_echo_on(
                &mut self,
                _msg: &std::ffi::CStr,
            ) -> Result<std::ffi::CString, pam_client::ErrorCode> {
                Err(pam_client::ErrorCode::ABORT)
            }

            fn prompt_echo_off(
                &mut self,
                _msg: &std::ffi::CStr,
            ) -> Result<std::ffi::CString, pam_client::ErrorCode> {
                if let Some(pwd) = self.password.take() {
                    std::ffi::CString::new(pwd.as_str()).map_err(|_| pam_client::ErrorCode::ABORT)
                } else {
                    Err(pam_client::ErrorCode::ABORT)
                }
            }

            fn text_info(&mut self, _msg: &std::ffi::CStr) {}
            fn error_msg(&mut self, _msg: &std::ffi::CStr) {}
            fn radio_prompt(
                &mut self,
                _msg: &std::ffi::CStr,
            ) -> Result<bool, pam_client::ErrorCode> {
                Ok(false)
            }
        }

        // Get username
        let username = match users::get_current_username() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => {
                log::error!("Failed to get current username");
                return false;
            }
        };

        // Create PAM context
        let service_name = &self.config.pam_service;
        let conversation = SimpleConversation {
            password: Some(password),
        };

        let mut context =
            match pam_client::Context::new(service_name, Some(username.as_str()), conversation) {
                Ok(ctx) => ctx,
                Err(e) => {
                    log::error!("Failed to initialize PAM context: {:?}", e);
                    return false;
                }
            };

        // Authenticate
        match context.authenticate(pam_client::Flag::NONE) {
            Ok(()) => {
                log::info!("PAM authentication successful for user {}", username);
                true
            }
            Err(e) => {
                log::warn!("PAM authentication failed: {:?}", e);
                false
            }
        }
    }

    /// Get the input handler for this locked surface
    pub fn input_handler(&self) -> &InputHandler {
        &self.input_handler
    }

    /// Get the rendered image surface for this locked surface
    pub fn as_image_surface(&self) -> &ImageSurface {
        self.renderer.as_image_surface()
    }

    /// Get the current display password (masked)
    pub fn get_display_password(&self) -> String {
        self.input_handler.get_display_password()
    }

    /// Get the output dimensions
    pub fn dimensions(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Set the Wayland surface for this locked surface
    pub fn set_wayland_surface(&mut self, surface: wl_surface::WlSurface) {
        self.wayland_surface = Some(surface);
    }

    /// Get the Wayland surface for this locked surface
    pub fn wayland_surface(&self) -> Option<&wl_surface::WlSurface> {
        self.wayland_surface.as_ref()
    }

    /// Check if this surface has a Wayland surface attached
    pub fn has_wayland_surface(&self) -> bool {
        self.wayland_surface.is_some()
    }
}

/// Manager for all locked surfaces (multiple outputs)
pub struct LockManager {
    pub surfaces: Vec<LockedSurface>,
    config: Config,
    locked: bool,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new(config: Config) -> Self {
        Self {
            surfaces: Vec::new(),
            config,
            locked: false,
        }
    }

    /// Add a locked surface for an output
    pub fn add_surface(&mut self, width: i32, height: i32) -> bool {
        match LockedSurface::new(width, height, &self.config) {
            Some(surface) => {
                self.surfaces.push(surface);
                true
            }
            None => false,
        }
    }

    /// Update all locked surfaces
    pub fn update(&mut self) {
        for surface in &mut self.surfaces {
            surface.update();
        }
    }

    /// Handle a key event and return any action that needs processing
    /// Returns the first non-None action from any surface
    pub fn handle_key_event(
        &mut self,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) -> Option<InputAction> {
        // Distribute key event to all surfaces and collect first action
        let mut action = None;
        for surface in &mut self.surfaces {
            if let Some(a) = surface.handle_key_event(event.clone()) {
                action = Some(a);
            }
        }
        action
    }

    /// Check if session is locked
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Lock the session
    pub fn lock(&mut self) {
        self.locked = true;
        // TODO: Implement actual Wayland session locking
    }

    /// Unlock the session
    pub fn unlock(&mut self) {
        self.locked = false;
        // TODO: Implement actual Wayland session unlocking
    }

    /// Get the number of locked surfaces
    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }

    /// Get a reference to a locked surface by index
    pub fn get_surface(&self, index: usize) -> Option<&LockedSurface> {
        self.surfaces.get(index)
    }

    /// Get a mutable reference to a locked surface by index
    pub fn get_surface_mut(&mut self, index: usize) -> Option<&mut LockedSurface> {
        self.surfaces.get_mut(index)
    }

    /// Initialize lock surfaces for all outputs (called after session is locked)
    pub fn initialize_lock_surfaces(&mut self) {
        // In a real implementation, this would create Wayland surfaces for each output
        // For now, we'll create dummy surfaces with default dimensions
        if self.surfaces.is_empty() {
            // Add a default surface (single monitor)
            self.add_surface(1920, 1080);
        }
    }

    /// Toggle temp screenshot peek mode
    pub fn toggle_peek(&mut self) {
        for surface in &mut self.surfaces {
            surface.input_handler.update_temp_screenshot();
        }
    }

    /// Find a locked surface by Wayland surface
    pub fn find_surface_by_wayland_surface(
        &mut self,
        wayland_surface: &wl_surface::WlSurface,
    ) -> Option<&mut LockedSurface> {
        self.surfaces
            .iter_mut()
            .find(|surface| surface.matches_surface(wayland_surface))
    }
}
