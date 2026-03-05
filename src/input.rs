use zeroize::Zeroizing;

/// Handles keyboard input for password entry
pub struct InputHandler {
    password_buffer: Zeroizing<String>,
    cursor_position: usize,
    config: crate::config::Config,
    wrong_password_timer: Option<std::time::Instant>,
    key_highlight_timer: Option<std::time::Instant>,
    temp_screenshot_timer: Option<std::time::Instant>,
    temp_screenshot_active: bool,
}

impl InputHandler {
    pub fn new(config: crate::config::Config) -> Self {
        Self {
            password_buffer: Zeroizing::new(String::new()),
            cursor_position: 0,
            config,
            wrong_password_timer: None,
            key_highlight_timer: None,
            temp_screenshot_timer: None,
            temp_screenshot_active: false,
        }
    }

    /// Handle a key event from Wayland
    pub fn handle_key_event(
        &mut self,
        keysym: smithay_client_toolkit::seat::keyboard::Keysym,
        state: wayland_client::protocol::wl_keyboard::KeyState,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
    ) -> InputAction {
        // Only process key press events
        if state != wayland_client::protocol::wl_keyboard::KeyState::Pressed {
            return InputAction::None;
        }

        // Convert keysym to character
        let ch = self.keysym_to_char(keysym, modifiers);

        match ch {
            Some('\x08') | Some('\x7f') => {
                // Backspace or Delete
                if !self.password_buffer.is_empty() && self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.password_buffer.remove(self.cursor_position);
                }
                InputAction::PasswordChanged
            }
            Some('\r') | Some('\n') => {
                // Enter key - submit password
                let password = self.password_buffer.clone();
                self.password_buffer.clear();
                self.cursor_position = 0;
                InputAction::SubmitPassword(password)
            }
            Some('\x1b') => {
                // Escape key - cancel
                InputAction::Cancel
            }
            Some('p') | Some('P') if self.config.temp_screenshot => {
                // 'p' key for temp screenshot peek
                self.activate_temp_screenshot();
                InputAction::TempScreenshot
            }
            Some(c) if c.is_ascii() && !c.is_control() => {
                // Printable ASCII character
                self.password_buffer.insert(self.cursor_position, c);
                self.cursor_position += 1;
                InputAction::PasswordChanged
            }
            _ => {
                // Other keys (function keys, arrows, etc.)
                InputAction::None
            }
        }
    }

    /// Convert a keysym to a character, considering modifiers
    fn keysym_to_char(
        &self,
        keysym: smithay_client_toolkit::seat::keyboard::Keysym,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
    ) -> Option<char> {
        use smithay_client_toolkit::seat::keyboard::Keysym;

        // Handle special keys first
        match keysym {
            Keysym::BackSpace => return Some('\x08'),
            Keysym::Delete => return Some('\x7f'),
            Keysym::Return => return Some('\r'),
            Keysym::KP_Enter => return Some('\n'),
            Keysym::Escape => return Some('\x1b'),
            _ => {}
        }

        // Convert keysym to character
        let keysym_value = keysym.raw();

        // Basic ASCII conversion (simplified - real implementation would use xkbcommon)
        // This is a simplified mapping for demonstration
        if keysym_value >= 0x20 && keysym_value <= 0x7e {
            let mut ch = keysym_value as u8 as char;

            // Apply shift modifier
            if modifiers.shift {
                ch = match ch {
                    '`' => '~',
                    '1' => '!',
                    '2' => '@',
                    '3' => '#',
                    '4' => '$',
                    '5' => '%',
                    '6' => '^',
                    '7' => '&',
                    '8' => '*',
                    '9' => '(',
                    '0' => ')',
                    '-' => '_',
                    '=' => '+',
                    '[' => '{',
                    ']' => '}',
                    '\\' => '|',
                    ';' => ':',
                    '\'' => '"',
                    ',' => '<',
                    '.' => '>',
                    '/' => '?',
                    c if c.is_ascii_lowercase() => c.to_ascii_uppercase(),
                    _ => ch,
                };
            }

            Some(ch)
        } else {
            None
        }
    }

    /// Get the current password (for display purposes only - returns masked version)
    pub fn get_display_password(&self) -> String {
        self.password_buffer.chars().map(|_| '•').collect()
    }

    /// Get the actual password (for authentication)
    pub fn get_password(&self) -> Zeroizing<String> {
        self.password_buffer.clone()
    }

    /// Clear the password buffer (e.g., after wrong password)
    pub fn clear_password(&mut self) {
        self.password_buffer.clear();
        self.cursor_position = 0;
    }

    /// Set wrong password feedback timer
    pub fn set_wrong_password_feedback(&mut self) {
        self.wrong_password_timer = Some(std::time::Instant::now());
    }

    /// Check if wrong password feedback should be shown
    pub fn should_show_wrong_password(&self) -> bool {
        if let Some(timer) = self.wrong_password_timer {
            timer.elapsed() < std::time::Duration::from_millis(1000)
        } else {
            false
        }
    }

    /// Set key highlight timer (for visual feedback)
    pub fn set_key_highlight(&mut self) {
        self.key_highlight_timer = Some(std::time::Instant::now());
    }

    /// Check if key highlight should be shown
    pub fn should_show_key_highlight(&self) -> bool {
        if let Some(timer) = self.key_highlight_timer {
            timer.elapsed() < std::time::Duration::from_millis(200)
        } else {
            false
        }
    }

    /// Update timers (should be called periodically)
    pub fn update(&mut self) {
        // Update temp screenshot state
        self.update_temp_screenshot();
    }

    /// Activate temporary screenshot display (peek feature)
    pub fn activate_temp_screenshot(&mut self) {
        self.temp_screenshot_timer = Some(std::time::Instant::now());
        self.temp_screenshot_active = true;
    }

    /// Check if temporary screenshot should be shown
    pub fn should_show_temp_screenshot(&self) -> bool {
        if let Some(timer) = self.temp_screenshot_timer {
            let elapsed = timer.elapsed();
            // Show for 2 seconds
            if elapsed < std::time::Duration::from_secs(2) {
                return true;
            }
        }
        false
    }

    /// Check if temp screenshot is currently active
    pub fn is_temp_screenshot_active(&self) -> bool {
        self.temp_screenshot_active
    }

    /// Update temp screenshot state (call periodically)
    pub fn update_temp_screenshot(&mut self) {
        if self.temp_screenshot_active && !self.should_show_temp_screenshot() {
            self.temp_screenshot_active = false;
            self.temp_screenshot_timer = None;
        }
    }
}

/// Actions that can result from keyboard input
#[derive(Debug)]
pub enum InputAction {
    None,
    PasswordChanged,
    SubmitPassword(Zeroizing<String>),
    Cancel,
    TempScreenshot,
}
