use std::ffi::{CStr, CString};
use std::thread;

use log::{debug, error};
use pam_client::{Context, ErrorCode, Flag};
use smithay_client_toolkit::reexports::{calloop::channel, calloop::EventLoop};
use users::get_current_username;
use zeroize::Zeroizing;

const SERVICE_NAME: &str = "wayrustlock";

pub struct LockConversation {
    pub password: Option<Zeroizing<String>>,
}

impl pam_client::ConversationHandler for LockConversation {
    fn init(&mut self, _default_user: Option<impl AsRef<str>>) {}

    fn prompt_echo_on(&mut self, _msg: &CStr) -> Result<CString, ErrorCode> {
        Err(ErrorCode::ABORT)
    }

    fn prompt_echo_off(&mut self, _msg: &CStr) -> Result<CString, ErrorCode> {
        if let Some(password) = self.password.take() {
            CString::new(password.as_str()).map_err(|_| ErrorCode::ABORT)
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

pub fn create_and_run_auth_loop() -> (channel::Sender<Zeroizing<String>>, channel::Channel<bool>) {
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

    let (auth_req_send, auth_req_recv) = channel::channel::<Zeroizing<String>>();
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
