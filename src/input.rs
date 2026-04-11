use zeroize::Zeroizing;

/// Handles keyboard input for password entry
pub struct InputHandler {
    password_buffer: Zeroizing<String>,
    cursor_position: usize,
    wrong_password_timer: Option<std::time::Instant>,
    key_highlight_timer: Option<std::time::Instant>,
    caps_lock: bool,
}

impl InputHandler {
    pub fn new(_config: crate::config::Config) -> Self {
        Self {
            password_buffer: Zeroizing::new(String::new()),
            cursor_position: 0,
            wrong_password_timer: None,
            key_highlight_timer: None,
            caps_lock: false,
        }
    }

    /// Handle a key event from Wayland
    pub fn handle_key_event(
        &mut self,
        keysym: smithay_client_toolkit::seat::keyboard::Keysym,
        utf8: Option<String>,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
    ) -> InputAction {
        // Update Caps Lock state
        self.caps_lock = modifiers.caps_lock;

        if modifiers.ctrl && keysym == Keysym::u {
            if !self.password_buffer.is_empty() {
                self.password_buffer.clear();
                self.cursor_position = 0;
                return InputAction::PasswordCleared;
            }
            return InputAction::None;
        }

        // Handle special keys first using keysym
        use smithay_client_toolkit::seat::keyboard::Keysym;
        match keysym {
            Keysym::BackSpace => {
                if !self.password_buffer.is_empty() && self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.password_buffer.remove(self.cursor_position);
                    if self.password_buffer.is_empty() {
                        return InputAction::PasswordCleared;
                    }
                }
                return InputAction::PasswordChanged;
            }
            Keysym::Return | Keysym::KP_Enter => {
                let password = self.password_buffer.clone();
                self.password_buffer.clear();
                self.cursor_position = 0;
                return InputAction::SubmitPassword(password);
            }
            Keysym::Escape => {
                return InputAction::Cancel;
            }
            Keysym::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    return InputAction::CursorMoved;
                }
                return InputAction::None;
            }
            Keysym::Right => {
                if self.cursor_position < self.password_buffer.len() {
                    self.cursor_position += 1;
                    return InputAction::CursorMoved;
                }
                return InputAction::None;
            }
            Keysym::Home => {
                if self.cursor_position > 0 {
                    self.cursor_position = 0;
                    return InputAction::CursorMoved;
                }
                return InputAction::None;
            }
            Keysym::End => {
                if self.cursor_position < self.password_buffer.len() {
                    self.cursor_position = self.password_buffer.len();
                    return InputAction::CursorMoved;
                }
                return InputAction::None;
            }
            Keysym::Delete => {
                if self.cursor_position < self.password_buffer.len() {
                    self.password_buffer.remove(self.cursor_position);
                    if self.password_buffer.is_empty() {
                        return InputAction::PasswordCleared;
                    }
                    return InputAction::PasswordChanged;
                }
                return InputAction::None;
            }
            _ => {}
        }

        // Use the UTF-8 string provided by SCTK for character input
        if let Some(txt) = utf8 {
            for c in txt.chars() {
                if c.is_ascii() && !c.is_control() {
                    self.password_buffer.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                }
            }
            return InputAction::PasswordChanged;
        }

        InputAction::None
    }

    pub fn password_length(&self) -> usize {
        self.password_buffer.len()
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor_position
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
    pub fn update(&mut self) {}

    /// Get the current Caps Lock state
    pub fn caps_lock(&self) -> bool {
        self.caps_lock
    }
}

/// Actions that can result from keyboard input
#[derive(Debug)]
pub enum InputAction {
    None,
    PasswordChanged,
    PasswordCleared,
    CursorMoved,
    SubmitPassword(Zeroizing<String>),
    Cancel,
}
