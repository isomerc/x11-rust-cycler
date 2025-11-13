use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use std::sync::{Arc, Mutex};

pub struct X11Manager {
    conn: Arc<RustConnection>,
    screen_num: usize,
}

#[derive(Debug, Clone)]
pub struct EveWindow {
    pub id: u32,
    pub title: String,
}

impl X11Manager {
    pub fn new() -> Result<Self> {
        let (conn, screen_num) = RustConnection::connect(None)
            .context("Failed to connect to X11 server")?;

        Ok(Self {
            conn: Arc::new(conn),
            screen_num,
        })
    }

    pub fn get_eve_windows(&self) -> Result<Vec<EveWindow>> {
        let screen = &self.conn.setup().roots[self.screen_num];
        let root = screen.root;

        // Get _NET_CLIENT_LIST atom
        let net_client_list = self.conn
            .intern_atom(false, b"_NET_CLIENT_LIST")?
            .reply()?
            .atom;

        // Get list of all windows
        let client_list_reply = self.conn
            .get_property(
                false,
                root,
                net_client_list,
                AtomEnum::WINDOW,
                0,
                u32::MAX,
            )?
            .reply()?;

        let windows: Vec<u32> = client_list_reply
            .value32()
            .ok_or_else(|| anyhow::anyhow!("Failed to get window list"))?
            .collect();

        let mut eve_windows = Vec::new();

        for &window in &windows {
            if let Ok(title) = self.get_window_title(window) {
                // Filter for EVE windows (steam_app_8500) and exclude launcher
                if title.starts_with("EVE - ") && !title.contains("Launcher") {
                    eve_windows.push(EveWindow {
                        id: window,
                        title: title.trim_start_matches("EVE - ").to_string(),
                    });
                }
            }
        }

        Ok(eve_windows)
    }

    pub fn get_active_window(&self) -> Result<u32> {
        let screen = &self.conn.setup().roots[self.screen_num];
        let root = screen.root;

        let net_active_window = self.conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
            .reply()?
            .atom;

        let reply = self.conn
            .get_property(
                false,
                root,
                net_active_window,
                AtomEnum::WINDOW,
                0,
                1,
            )?
            .reply()?;

        let active: Vec<u32> = reply.value32()
            .ok_or_else(|| anyhow::anyhow!("Failed to get active window"))?
            .collect();

        Ok(*active.first().unwrap_or(&0))
    }

    pub fn activate_window(&self, window_id: u32) -> Result<()> {
        let screen = &self.conn.setup().roots[self.screen_num];
        let root = screen.root;

        let net_active_window = self.conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
            .reply()?
            .atom;

        // Send _NET_ACTIVE_WINDOW message
        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: window_id,
            type_: net_active_window,
            data: ClientMessageData::from([2, 0, 0, 0, 0]),
        };

        self.conn.send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT,
            event,
        )?;

        self.conn.flush()?;
        Ok(())
    }

    pub fn stack_windows(&self, windows: &[EveWindow], x: i32, y: i32, width: u32, height: u32) -> Result<()> {
        for window in windows {
            // Move and resize window
            let values = ConfigureWindowAux::new()
                .x(x)
                .y(y)
                .width(width)
                .height(height);

            self.conn.configure_window(window.id, &values)?;
        }

        self.conn.flush()?;
        Ok(())
    }

    fn get_window_title(&self, window: u32) -> Result<String> {
        // Try _NET_WM_NAME first (UTF-8)
        let net_wm_name = self.conn
            .intern_atom(false, b"_NET_WM_NAME")?
            .reply()?
            .atom;

        let utf8_string = self.conn
            .intern_atom(false, b"UTF8_STRING")?
            .reply()?
            .atom;

        if let Ok(reply) = self.conn
            .get_property(false, window, net_wm_name, utf8_string, 0, 1024)?
            .reply()
        {
            if !reply.value.is_empty() {
                if let Ok(title) = String::from_utf8(reply.value.clone()) {
                    return Ok(title);
                }
            }
        }

        // Fall back to WM_NAME
        if let Ok(reply) = self.conn
            .get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)?
            .reply()
        {
            if !reply.value.is_empty() {
                return Ok(String::from_utf8_lossy(&reply.value).to_string());
            }
        }

        Ok(String::new())
    }

    pub fn get_connection(&self) -> Arc<RustConnection> {
        Arc::clone(&self.conn)
    }

    pub fn get_screen_num(&self) -> usize {
        self.screen_num
    }
}
