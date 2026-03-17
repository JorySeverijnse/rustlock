use log::{debug, error};
use mpris::PlayerFinder;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use zbus::Connection;

#[derive(Clone, Default)]
pub struct SystemStatus {
    pub battery_percent: Option<f64>,
    pub is_charging: bool,
    pub media_title: Option<String>,
    pub media_artist: Option<String>,
    pub media_playing: bool,
    pub media_art_url: Option<String>,
    pub media_art_data: Option<Arc<Vec<u8>>>,
    pub wifi_ssid: Option<String>,
    pub wifi_strength: Option<u8>,
    pub bluetooth_connected: bool,
    pub bluetooth_devices: Vec<String>,
    pub keyboard_layout: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum SystemCommand {
    PowerOff,
    Reboot,
    Suspend,
}

pub struct SystemManager {
    status: Arc<Mutex<SystemStatus>>,
    cmd_tx: mpsc::UnboundedSender<SystemCommand>,
}

impl SystemManager {
    pub fn new() -> Self {
        let status = Arc::new(Mutex::new(SystemStatus::default()));
        let s_clone = status.clone();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SystemCommand>();

        // Spawn a thread to update status periodically and handle commands
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    error!("Failed to create tokio runtime for SystemManager: {}", e);
                    return;
                }
            };

            rt.block_on(async {
                let mut conn: Option<Connection> = None;
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
                let mut last_art_url: Option<String> = None;
                let mut last_art_data: Option<Arc<Vec<u8>>> = None;

                loop {
                    // Try to connect to system DBus if not connected
                    if conn.is_none() {
                        match Connection::system().await {
                            Ok(c) => conn = Some(c),
                            Err(e) => {
                                error!("Failed to connect to system DBus: {}", e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            }
                        }
                    }

                    tokio::select! {
                        _ = interval.tick() => {
                            let mut new_status = SystemStatus::default();

                            if let Some(ref c) = conn {
                                // 1. Battery status
                                if let Ok(proxy) = upower_dbus::UPowerProxy::new(c).await {
                                    if let Ok(display_device) = proxy.get_display_device().await {
                                        new_status.battery_percent = display_device.percentage().await.ok();
                                        if let Ok(state) = display_device.state().await {
                                            new_status.is_charging = format!("{:?}", state).contains("Charging");
                                        }
                                    }
                                }
                            }

                            // 2. MPRIS status
                            if let Ok(finder) = PlayerFinder::new() {
                                if let Ok(player) = finder.find_active() {
                                    if let Ok(metadata) = player.get_metadata() {
                                        new_status.media_title = metadata.title().map(|s| s.to_string());
                                        new_status.media_artist = metadata.artists().map(|a| a.join(", "));
                                        new_status.media_art_url = metadata.art_url().map(|u| u.to_string());

                                        if new_status.media_art_url != last_art_url {
                                            last_art_url = new_status.media_art_url.clone();
                                            last_art_data = None;
                                            if let Some(ref url) = last_art_url {
                                                if url.starts_with("file://") {
                                                    let path = url.trim_start_matches("file://");
                                                    if let Ok(data) = std::fs::read(path) {
                                                        last_art_data = Some(Arc::new(data));
                                                    }
                                                } else if url.starts_with("http") {
                                                    #[cfg(feature = "networking")]
                                                    if let Ok(resp) = reqwest::get(url).await {
                                                        if let Ok(bytes) = resp.bytes().await {
                                                            last_art_data = Some(Arc::new(bytes.to_vec()));
                                                        }
                                                    }
                                                    #[cfg(not(feature = "networking"))]
                                                    {
                                                        log::debug!("Networking disabled, skipping remote album art: {}", url);
                                                    }
                                                }
                                            }
                                        }
                                        new_status.media_art_data = last_art_data.clone();
                                    }
                                    new_status.media_playing = player.get_playback_status().map(|s| matches!(s, mpris::PlaybackStatus::Playing)).unwrap_or(false);
                                }
                            }

                            // 3. WiFi status (NetworkManager)
                            if let Some(ref c) = conn {
                                if let Ok(reply) = c.call_method(
                                    Some("org.freedesktop.NetworkManager"),
                                    "/org/freedesktop/NetworkManager",
                                    Some("org.freedesktop.NetworkManager"),
                                    "GetDevices",
                                    &(),
                                ).await {
                                    let devices: Vec<zbus::zvariant::OwnedObjectPath> = reply.body().unwrap();
                                    for dev_path in devices {
                                        if let Ok(dev_type_reply) = c.call_method(
                                            Some("org.freedesktop.NetworkManager"),
                                            &dev_path,
                                            Some("org.freedesktop.DBus.Properties"),
                                            "Get",
                                            &("org.freedesktop.NetworkManager.Device", "DeviceType"),
                                        ).await {
                                            let dev_type: u32 = dev_type_reply.body::<zbus::zvariant::Value>().unwrap().downcast().unwrap();
                                            if dev_type == 2 { // WiFi
                                                if let Ok(active_ap_reply) = c.call_method(
                                                    Some("org.freedesktop.NetworkManager"),
                                                    &dev_path,
                                                    Some("org.freedesktop.DBus.Properties"),
                                                    "Get",
                                                    &("org.freedesktop.NetworkManager.Device.Wireless", "ActiveAccessPoint"),
                                                ).await {
                                                    let ap_path: zbus::zvariant::OwnedObjectPath = active_ap_reply.body::<zbus::zvariant::Value>().unwrap().downcast().unwrap();
                                                    if ap_path.as_str() != "/" {
                                                        if let Ok(ssid_reply) = c.call_method(
                                                            Some("org.freedesktop.NetworkManager"),
                                                            &ap_path,
                                                            Some("org.freedesktop.DBus.Properties"),
                                                            "Get",
                                                            &("org.freedesktop.NetworkManager.AccessPoint", "Ssid"),
                                                        ).await {
                                                            let ssid_bytes: Vec<u8> = ssid_reply.body::<zbus::zvariant::Value>().unwrap().downcast().unwrap();
                                                            new_status.wifi_ssid = Some(String::from_utf8_lossy(&ssid_bytes).to_string());
                                                        }
                                                        if let Ok(strength_reply) = c.call_method(
                                                            Some("org.freedesktop.NetworkManager"),
                                                            &ap_path,
                                                            Some("org.freedesktop.DBus.Properties"),
                                                            "Get",
                                                            &("org.freedesktop.NetworkManager.AccessPoint", "Strength"),
                                                        ).await {
                                                            new_status.wifi_strength = Some(strength_reply.body::<zbus::zvariant::Value>().unwrap().downcast().unwrap());
                                                        }
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }

                                // 4. Bluetooth status (BlueZ)
                                if let Ok(objects_reply) = c.call_method(
                                    Some("org.bluez"),
                                    "/",
                                    Some("org.freedesktop.DBus.ObjectManager"),
                                    "GetManagedObjects",
                                    &(),
                                ).await {
                                    use std::collections::HashMap;
                                    type ManagedObjects = HashMap<zbus::zvariant::OwnedObjectPath, HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>>;
                                    if let Ok(objects) = objects_reply.body::<ManagedObjects>() {
                                        for (_path, interfaces) in objects {
                                            if let Some(device) = interfaces.get("org.bluez.Device1") {
                                                if let Some(connected) = device.get("Connected") {
                                                    if connected.downcast_ref::<bool>().copied().unwrap_or(false) {
                                                        new_status.bluetooth_connected = true;
                                                        if let Some(name) = device.get("Name") {
                                                            let name_str: String = name.downcast_ref::<str>().map(|s| s.to_string()).unwrap_or_default();
                                                            new_status.bluetooth_devices.push(name_str);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            {
                                if let Ok(mut s) = s_clone.lock() {
                                    *s = new_status;
                                }
                            }
                        }
                        Some(command) = cmd_rx.recv() => {
                            if let Some(ref c) = conn {
                                let method = match command {
                                    SystemCommand::PowerOff => "PowerOff",
                                    SystemCommand::Reboot => "Reboot",
                                    SystemCommand::Suspend => "Suspend",
                                };

                                debug!("Executing system command: {}", method);
                                // Set a timeout for the DBus call to prevent hanging the background thread
                                let result = tokio::time::timeout(
                                    tokio::time::Duration::from_secs(5),
                                    c.call_method(
                                        Some("org.freedesktop.login1"),
                                        "/org/freedesktop/login1",
                                        Some("org.freedesktop.login1.Manager"),
                                        method,
                                        &(true),
                                    )
                                ).await;

                                if let Err(_) = result {
                                    error!("System command {} timed out", method);
                                }
                            }
                        }
                    }
                }
            });
        });

        Self { status, cmd_tx }
    }

    pub fn get_status(&self) -> SystemStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn send_command(&self, cmd: SystemCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn media_play_pause(&self) {
        if let Ok(finder) = PlayerFinder::new() {
            if let Ok(player) = finder.find_active() {
                let _ = player.play_pause();
            }
        }
    }

    pub fn media_next(&self) {
        if let Ok(finder) = PlayerFinder::new() {
            if let Ok(player) = finder.find_active() {
                let _ = player.next();
            }
        }
    }

    pub fn media_prev(&self) {
        if let Ok(finder) = PlayerFinder::new() {
            if let Ok(player) = finder.find_active() {
                let _ = player.previous();
            }
        }
    }
}
