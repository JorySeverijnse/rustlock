use std::ffi::{CStr, CString};
use std::thread;

use log::{debug, error};
use pam_client::{Context, ErrorCode, Flag};
use secstr::SecVec;
use smithay_client_toolkit::reexports::{calloop::channel, calloop::EventLoop};
use users::get_current_username;

const SERVICE_NAME: &str = "wayrustlock";

pub struct PasswordBuffer(SecVec<u8>);

impl PasswordBuffer {
    pub fn new() -> Self {
        Self(SecVec::new(Vec::new()))
    }

    fn zeroize_string(mut data: String) {
        use std::sync::atomic;

        let default = u8::default();

        for c in unsafe { data.as_bytes_mut() } {
            unsafe { std::ptr::write_volatile(c, default) };
        }

        atomic::fence(atomic::Ordering::SeqCst);
        atomic::compiler_fence(atomic::Ordering::SeqCst);
    }

    pub fn append(&mut self, data: String) {
        let bytes = data.as_bytes();
        let mut og_len = self.0.unsecure().len();
        self.0.resize(og_len + bytes.len(), 0);
        for b in bytes {
            self.0.unsecure_mut()[og_len] = *b;
            og_len += 1;
        }
        Self::zeroize_string(data);
    }

    pub fn backspace(&mut self) {
        let og_len = self.0.unsecure().len();
        if og_len != 0 {
            self.0.resize(og_len - 1, 0);
        }
    }

    pub fn unsecure(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.0.unsecure()) }
    }

    pub fn take(&mut self) -> Self {
        let mut new_buffer = SecVec::new(Vec::new());
        std::mem::swap(&mut self.0, &mut new_buffer);
        Self(new_buffer)
    }
}

pub struct LockConversation {
    pub password: Option<PasswordBuffer>,
}

impl pam_client::ConversationHandler for LockConversation {
    fn init(&mut self, _default_user: Option<impl AsRef<str>>) {}

    fn prompt_echo_on(&mut self, _msg: &CStr) -> Result<CString, ErrorCode> {
        Err(ErrorCode::ABORT)
    }

    fn prompt_echo_off(&mut self, _msg: &CStr) -> Result<CString, ErrorCode> {
        if let Some(password) = self.password.take() {
            CString::new(password.unsecure()).map_err(|_| ErrorCode::ABORT)
        } else {
            Err(ErrorCode::ABORT)
        }
    }

    fn text_info(&mut self, _msg: &CStr) {}
    fn error_msg(&mut self, _msg: &CStr) {}
    fn radio_prompt(&mut self, _msg: &CStr) -> Result<bool, ErrorCode> {
        Ok(false)
    }
}

pub fn create_and_run_auth_loop() -> (channel::Sender<PasswordBuffer>, channel::Channel<bool>) {
    struct AuthLoopState {
        auth_res_send: channel::Sender<bool>,
        main_closed: bool,
        context: pam_client::Context<LockConversation>,
    }

    let username = get_current_username()
        .expect("Failed to get username")
        .to_str()
        .expect("Failed to get non-unicode username")
        .to_string();

    let conversation = LockConversation { password: None };
    let context = Context::new(SERVICE_NAME, Some(username.as_str()), conversation)
        .expect("Failed to initialize PAM context");
    debug!("Prepared to authenticate user '{}'", username);

    let (auth_req_send, auth_req_recv) = channel::channel::<PasswordBuffer>();
    let (auth_res_send, auth_res_recv) = channel::channel::<bool>();

    thread::spawn(move || {
        let mut event_loop: EventLoop<AuthLoopState> = EventLoop::try_new().unwrap();
        event_loop
            .handle()
            .insert_source(auth_req_recv, |evt, _metadata, state| match evt {
                channel::Event::Msg(password) => {
                    state.context.conversation_mut().password = Some(password);
                    let status = match state.context.authenticate(Flag::NONE) {
                        Ok(()) => true,
                        Err(err) => {
                            error!("Pam authenticate failed with {:?}", err);
                            false
                        }
                    };
                    state.auth_res_send.send(status).unwrap();
                }
                channel::Event::Closed => state.main_closed = true,
            })
            .unwrap();

        let mut state = AuthLoopState {
            auth_res_send,
            main_closed: false,
            context,
        };

        while !state.main_closed {
            event_loop
                .dispatch(None, &mut state)
                .expect("Failed to run");
        }
    });

    (auth_req_send, auth_res_recv)
}
