//! Fingerprint unlock through fprintd's D-Bus API, running alongside the
//! password prompt: whichever succeeds first unlocks.
//!
//! PAM cannot do this for a locker (pam_fprintd blocks the conversation until
//! a finger or a timeout), so the lock screen talks to fprintd directly, like
//! GDM and hyprlock do. The PAM service stays password only.
//!
//! After `fingerprint_max_attempts` fingers that do not match, the sensor is
//! released and only the password unlocks (as on macOS). The claim on the
//! sensor is dropped across suspend, so it is taken again on resume.

use std::future::poll_fn;
use std::pin::Pin;
use std::time::Duration;

use log::{debug, info, warn};
use smithay_client_toolkit::reexports::calloop::channel;
use zbus::export::futures_core::Stream;
use zbus::message::Type;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, MatchRule, Message, MessageStream};

const FPRINT: &str = "net.reactivated.Fprint";
const MANAGER_PATH: &str = "/net/reactivated/Fprint/Manager";
const MANAGER_IFACE: &str = "net.reactivated.Fprint.Manager";
const DEVICE_IFACE: &str = "net.reactivated.Fprint.Device";

/// Delay before trying again after a transient failure (sensor busy, resume).
const RETRY_DELAY: Duration = Duration::from_secs(2);
/// Delay before looking for a sensor again when none is present.
const NO_DEVICE_DELAY: Duration = Duration::from_secs(10);

pub enum FingerprintEvent {
    /// An enrolled finger matched: unlock.
    Match,
    /// A finger was read but did not match.
    NoMatch,
    /// Too many fingers did not match: only the password unlocks now.
    Disabled,
}

enum Outcome {
    /// Stop for good (matched, disabled, or no enrolled fingers).
    Done,
    /// Try again after `RETRY_DELAY`.
    Retry,
    /// No sensor right now; look again after `NO_DEVICE_DELAY`.
    NoDevice,
}

/// Starts the fingerprint thread; events arrive on the returned channel.
pub fn spawn(max_attempts: u32) -> channel::Channel<FingerprintEvent> {
    let (tx, rx) = channel::channel();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                warn!("Fingerprint: failed to create tokio runtime: {}", e);
                return;
            }
        };
        rt.block_on(run(tx, max_attempts.max(1)));
    });
    rx
}

async fn run(tx: channel::Sender<FingerprintEvent>, max_attempts: u32) {
    let mut failures = 0;
    loop {
        let outcome = match session(&tx, max_attempts, &mut failures).await {
            Ok(outcome) => outcome,
            Err(e) => {
                debug!("Fingerprint: {}", e);
                Outcome::Retry
            }
        };
        match outcome {
            Outcome::Done => return,
            Outcome::Retry => tokio::time::sleep(RETRY_DELAY).await,
            Outcome::NoDevice => tokio::time::sleep(NO_DEVICE_DELAY).await,
        }
    }
}

async fn next(stream: &mut MessageStream) -> Option<zbus::Result<Message>> {
    poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

fn error_name(e: &zbus::Error) -> Option<String> {
    match e {
        zbus::Error::MethodError(name, _, _) => Some(name.to_string()),
        _ => None,
    }
}

async fn device_call<B>(
    conn: &Connection,
    dev: &OwnedObjectPath,
    method: &str,
    body: &B,
) -> zbus::Result<Message>
where
    B: serde::ser::Serialize + zbus::zvariant::DynamicType,
{
    conn.call_method(Some(FPRINT), dev, Some(DEVICE_IFACE), method, body)
        .await
}

/// Stops verification and releases the sensor; errors are irrelevant here.
async fn release(conn: &Connection, dev: &OwnedObjectPath) {
    let _ = device_call(conn, dev, "VerifyStop", &()).await;
    let _ = device_call(conn, dev, "Release", &()).await;
}

async fn session(
    tx: &channel::Sender<FingerprintEvent>,
    max_attempts: u32,
    failures: &mut u32,
) -> zbus::Result<Outcome> {
    let conn = Connection::system().await?;

    let dev: OwnedObjectPath = match conn
        .call_method(
            Some(FPRINT),
            MANAGER_PATH,
            Some(MANAGER_IFACE),
            "GetDefaultDevice",
            &(),
        )
        .await
    {
        Ok(reply) => reply.body().deserialize()?,
        Err(e) => {
            debug!("Fingerprint: no sensor ({})", e);
            return Ok(Outcome::NoDevice);
        }
    };

    // "" = the calling user.
    if let Err(e) = device_call(&conn, &dev, "Claim", &("",)).await {
        return Ok(match error_name(&e).as_deref() {
            // Someone else (e.g. sudo) is using the sensor right now.
            Some("net.reactivated.Fprint.Error.AlreadyInUse") => Outcome::Retry,
            _ => {
                debug!("Fingerprint: claim failed ({})", e);
                Outcome::Retry
            }
        });
    }

    // Only accept signals from the fprintd instance we are talking to: the bus
    // already filters by the well-known name, and matching on its unique name
    // lets zbus check the sender too.
    let owner: String = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &(FPRINT,),
        )
        .await?
        .body()
        .deserialize()?;

    let status_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(owner.as_str())?
        .path(dev.as_str())?
        .interface(DEVICE_IFACE)?
        .member("VerifyStatus")?
        .build();
    let mut status = MessageStream::for_match_rule(status_rule, &conn, None).await?;

    let sleep_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.login1")?
        .path("/org/freedesktop/login1")?
        .interface("org.freedesktop.login1.Manager")?
        .member("PrepareForSleep")?
        .build();
    let mut sleep = MessageStream::for_match_rule(sleep_rule, &conn, None).await?;

    loop {
        // "any" = any enrolled finger.
        if let Err(e) = device_call(&conn, &dev, "VerifyStart", &("any",)).await {
            release(&conn, &dev).await;
            return Ok(match error_name(&e).as_deref() {
                Some("net.reactivated.Fprint.Error.NoEnrolledPrints") => {
                    info!("Fingerprint: no enrolled fingers, password only");
                    Outcome::Done
                }
                _ => {
                    debug!("Fingerprint: VerifyStart failed ({})", e);
                    Outcome::Retry
                }
            });
        }
        debug!("Fingerprint: waiting for a finger");

        // One verification: runs until fprintd reports it done.
        loop {
            tokio::select! {
                msg = next(&mut status) => {
                    let Some(Ok(msg)) = msg else {
                        release(&conn, &dev).await;
                        return Ok(Outcome::Retry);
                    };
                    let (result, done): (String, bool) = msg.body().deserialize()?;
                    debug!("Fingerprint: {} (done: {})", result, done);
                    match result.as_str() {
                        "verify-match" => {
                            let _ = tx.send(FingerprintEvent::Match);
                            release(&conn, &dev).await;
                            return Ok(Outcome::Done);
                        }
                        "verify-no-match" => {
                            *failures += 1;
                            let _ = tx.send(FingerprintEvent::NoMatch);
                            if *failures >= max_attempts {
                                info!("Fingerprint: {} failed attempts, password only", failures);
                                let _ = tx.send(FingerprintEvent::Disabled);
                                release(&conn, &dev).await;
                                return Ok(Outcome::Done);
                            }
                        }
                        "verify-disconnected" | "verify-unknown-error" => {
                            release(&conn, &dev).await;
                            return Ok(Outcome::Retry);
                        }
                        // verify-retry-scan, verify-swipe-too-short, ...:
                        // the same verification continues.
                        _ => {}
                    }
                    if done {
                        // A finished verification must be stopped before
                        // the next VerifyStart.
                        let _ = device_call(&conn, &dev, "VerifyStop", &()).await;
                        break;
                    }
                }
                msg = next(&mut sleep) => {
                    let Some(Ok(msg)) = msg else {
                        release(&conn, &dev).await;
                        return Ok(Outcome::Retry);
                    };
                    let going_to_sleep: bool = msg.body().deserialize()?;
                    if !going_to_sleep {
                        // Resumed: fprintd dropped the device across suspend.
                        debug!("Fingerprint: resumed, claiming the sensor again");
                        release(&conn, &dev).await;
                        return Ok(Outcome::Retry);
                    }
                }
            }
        }
    }
}
