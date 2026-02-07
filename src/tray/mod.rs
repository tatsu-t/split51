use anyhow::Result;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem, MenuId};
use tray_icon::{TrayIcon, TrayIconBuilder, Icon};
use std::collections::HashMap;
use crate::config::ChannelSource;

pub enum TrayCommand {
    ToggleEnabled,
    ToggleSwapChannels,
    SetVolume(f32),
    SetBalance(f32),
    TestMainLeft,     // Test FL on main speakers
    TestMainRight,    // Test FR on main speakers
    TestSubLeft,      // Test L on 2nd output (routed)
    TestSubRight,     // Test R on 2nd output (routed)
    SetLeftSource(ChannelSource),
    SetRightSource(ChannelSource),
    ToggleLeftMute,
    ToggleRightMute,
    SetLeftVolume(f32),
    SetRightVolume(f32),
    SelectSourceDevice(String),
    SelectTargetDevice(String),
    Quit,
}

pub struct TrayManager {
    _tray_icon: TrayIcon,
    toggle_item: MenuItem,
    swap_item: CheckMenuItem,
    left_mute_item: CheckMenuItem,
    right_mute_item: CheckMenuItem,
    volume_items: HashMap<MenuId, f32>,
    balance_items: HashMap<MenuId, f32>,
    left_volume_items: HashMap<MenuId, f32>,
    right_volume_items: HashMap<MenuId, f32>,
    source_device_items: HashMap<MenuId, String>,
    target_device_items: HashMap<MenuId, String>,
    toggle_id: MenuId,
    swap_id: MenuId,
    quit_id: MenuId,
    test_main_left_id: MenuId,
    test_main_right_id: MenuId,
    test_sub_left_id: MenuId,
    test_sub_right_id: MenuId,
    left_rl_id: MenuId,
    left_rr_id: MenuId,
    right_rl_id: MenuId,
    right_rr_id: MenuId,
    left_mute_id: MenuId,
    right_mute_id: MenuId,
}

impl TrayManager {
    pub fn new(
        source_devices: &[String],
        target_devices: &[String],
        enabled: bool,
        swap_channels: bool,
    ) -> Result<Self> {
        // Create menu items
        let toggle_text = if enabled { "Disable Routing" } else { "Enable Routing" };
        let toggle_item = MenuItem::new(toggle_text, true, None);

        // Swap channels checkbox
        let swap_item = CheckMenuItem::new("Swap L/R Channels", true, swap_channels, None);

        // Source device submenu
        let source_submenu = Submenu::new("Source Device (Loopback)", true);
        let mut source_device_items = HashMap::new();
        for device in source_devices {
            let item = MenuItem::new(device, true, None);
            source_device_items.insert(item.id().clone(), device.clone());
            source_submenu.append(&item)?;
        }

        // Target device submenu
        let target_submenu = Submenu::new("Target Device (Output)", true);
        let mut target_device_items = HashMap::new();
        for device in target_devices {
            let item = MenuItem::new(device, true, None);
            target_device_items.insert(item.id().clone(), device.clone());
            target_submenu.append(&item)?;
        }

        // Master Volume submenu
        let volume_submenu = Submenu::new("Master Volume", true);
        let mut volume_items = HashMap::new();
        for v in [25, 50, 75, 100, 125, 150] {
            let item = MenuItem::new(&format!("{}%", v), true, None);
            volume_items.insert(item.id().clone(), v as f32 / 100.0);
            volume_submenu.append(&item)?;
        }

        // Balance submenu
        let balance_submenu = Submenu::new("Balance", true);
        let mut balance_items = HashMap::new();
        let balance_values = [
            ("Full Left", -1.0),
            ("50% Left", -0.5),
            ("Center", 0.0),
            ("50% Right", 0.5),
            ("Full Right", 1.0),
        ];
        for (label, value) in balance_values {
            let item = MenuItem::new(label, true, None);
            balance_items.insert(item.id().clone(), value);
            balance_submenu.append(&item)?;
        }

        // Left Speaker submenu
        let left_submenu = Submenu::new("Left Speaker", true);
        let left_rl = MenuItem::new("Source: RL (Rear Left)", true, None);
        let left_rr = MenuItem::new("Source: RR (Rear Right)", true, None);
        let left_mute = CheckMenuItem::new("Mute", true, false, None);
        left_submenu.append(&left_rl)?;
        left_submenu.append(&left_rr)?;
        left_submenu.append(&PredefinedMenuItem::separator())?;
        left_submenu.append(&left_mute)?;
        
        // Left volume
        let left_vol_submenu = Submenu::new("Volume", true);
        let mut left_volume_items = HashMap::new();
        for v in [25, 50, 75, 100, 125, 150] {
            let item = MenuItem::new(&format!("{}%", v), true, None);
            left_volume_items.insert(item.id().clone(), v as f32 / 100.0);
            left_vol_submenu.append(&item)?;
        }
        left_submenu.append(&left_vol_submenu)?;

        // Right Speaker submenu
        let right_submenu = Submenu::new("Right Speaker", true);
        let right_rl = MenuItem::new("Source: RL (Rear Left)", true, None);
        let right_rr = MenuItem::new("Source: RR (Rear Right)", true, None);
        let right_mute = CheckMenuItem::new("Mute", true, false, None);
        right_submenu.append(&right_rl)?;
        right_submenu.append(&right_rr)?;
        right_submenu.append(&PredefinedMenuItem::separator())?;
        right_submenu.append(&right_mute)?;

        // Right volume
        let right_vol_submenu = Submenu::new("Volume", true);
        let mut right_volume_items = HashMap::new();
        for v in [25, 50, 75, 100, 125, 150] {
            let item = MenuItem::new(&format!("{}%", v), true, None);
            right_volume_items.insert(item.id().clone(), v as f32 / 100.0);
            right_vol_submenu.append(&item)?;
        }
        right_submenu.append(&right_vol_submenu)?;

        // Speaker test submenu
        let test_submenu = Submenu::new("Speaker Test", true);
        let test_main_left = MenuItem::new("Main Left (FL)", true, None);
        let test_main_right = MenuItem::new("Main Right (FR)", true, None);
        let test_sub_left = MenuItem::new("Sub Left (L)", true, None);
        let test_sub_right = MenuItem::new("Sub Right (R)", true, None);
        test_submenu.append(&test_main_left)?;
        test_submenu.append(&test_main_right)?;
        test_submenu.append(&PredefinedMenuItem::separator())?;
        test_submenu.append(&test_sub_left)?;
        test_submenu.append(&test_sub_right)?;

        let quit_item = MenuItem::new("Quit", true, None);

        // Store IDs for event handling
        let toggle_id = toggle_item.id().clone();
        let swap_id = swap_item.id().clone();
        let quit_id = quit_item.id().clone();
        let test_main_left_id = test_main_left.id().clone();
        let test_main_right_id = test_main_right.id().clone();
        let test_sub_left_id = test_sub_left.id().clone();
        let test_sub_right_id = test_sub_right.id().clone();
        let left_rl_id = left_rl.id().clone();
        let left_rr_id = left_rr.id().clone();
        let right_rl_id = right_rl.id().clone();
        let right_rr_id = right_rr.id().clone();
        let left_mute_id = left_mute.id().clone();
        let right_mute_id = right_mute.id().clone();

        // Build menu
        let menu = Menu::new();
        menu.append(&toggle_item)?;
        menu.append(&swap_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&source_submenu)?;
        menu.append(&target_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&volume_submenu)?;
        menu.append(&balance_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&left_submenu)?;
        menu.append(&right_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&test_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit_item)?;

        // Create tray icon
        let icon = create_default_icon()?;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("tatsu-audioapp - Audio Router")
            .with_icon(icon)
            .with_menu(Box::new(menu.clone()))
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            toggle_item,
            swap_item,
            left_mute_item: left_mute,
            right_mute_item: right_mute,
            volume_items,
            balance_items,
            left_volume_items,
            right_volume_items,
            source_device_items,
            target_device_items,
            toggle_id,
            swap_id,
            quit_id,
            test_main_left_id,
            test_main_right_id,
            test_sub_left_id,
            test_sub_right_id,
            left_rl_id,
            left_rr_id,
            right_rl_id,
            right_rr_id,
            left_mute_id,
            right_mute_id,
        })
    }

    pub fn handle_menu_event(&self, event: &MenuEvent) -> Option<TrayCommand> {
        if event.id == self.toggle_id {
            Some(TrayCommand::ToggleEnabled)
        } else if event.id == self.swap_id {
            Some(TrayCommand::ToggleSwapChannels)
        } else if event.id == self.quit_id {
            Some(TrayCommand::Quit)
        } else if event.id == self.test_main_left_id {
            Some(TrayCommand::TestMainLeft)
        } else if event.id == self.test_main_right_id {
            Some(TrayCommand::TestMainRight)
        } else if event.id == self.test_sub_left_id {
            Some(TrayCommand::TestSubLeft)
        } else if event.id == self.test_sub_right_id {
            Some(TrayCommand::TestSubRight)
        } else if event.id == self.left_rl_id {
            Some(TrayCommand::SetLeftSource(ChannelSource::RL))
        } else if event.id == self.left_rr_id {
            Some(TrayCommand::SetLeftSource(ChannelSource::RR))
        } else if event.id == self.right_rl_id {
            Some(TrayCommand::SetRightSource(ChannelSource::RL))
        } else if event.id == self.right_rr_id {
            Some(TrayCommand::SetRightSource(ChannelSource::RR))
        } else if event.id == self.left_mute_id {
            Some(TrayCommand::ToggleLeftMute)
        } else if event.id == self.right_mute_id {
            Some(TrayCommand::ToggleRightMute)
        } else if let Some(&vol) = self.volume_items.get(&event.id) {
            Some(TrayCommand::SetVolume(vol))
        } else if let Some(&bal) = self.balance_items.get(&event.id) {
            Some(TrayCommand::SetBalance(bal))
        } else if let Some(&vol) = self.left_volume_items.get(&event.id) {
            Some(TrayCommand::SetLeftVolume(vol))
        } else if let Some(&vol) = self.right_volume_items.get(&event.id) {
            Some(TrayCommand::SetRightVolume(vol))
        } else if let Some(device) = self.source_device_items.get(&event.id) {
            Some(TrayCommand::SelectSourceDevice(device.clone()))
        } else if let Some(device) = self.target_device_items.get(&event.id) {
            Some(TrayCommand::SelectTargetDevice(device.clone()))
        } else {
            None
        }
    }

    pub fn update_toggle_text(&self, enabled: bool) {
        let text = if enabled { "⏸ Disable Routing" } else { "▶ Enable Routing" };
        self.toggle_item.set_text(text);
    }

    pub fn update_swap_checked(&self, swap: bool) {
        self.swap_item.set_checked(swap);
    }

    pub fn update_left_mute(&self, muted: bool) {
        self.left_mute_item.set_checked(muted);
    }

    pub fn update_right_mute(&self, muted: bool) {
        self.right_mute_item.set_checked(muted);
    }
}

fn create_default_icon() -> Result<Icon> {
    // Create a simple 16x16 RGBA icon (green/blue audio symbol)
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            // Create a simple speaker-like pattern
            let in_speaker = (x >= 2 && x <= 6 && y >= 4 && y <= 11) ||
                            (x >= 6 && x <= 10 && y >= 2 && y <= 13) ||
                            (x >= 10 && x <= 13 && (y == 4 || y == 7 || y == 10));
            if in_speaker {
                rgba[idx] = 50;      // R
                rgba[idx + 1] = 180; // G
                rgba[idx + 2] = 80;  // B
                rgba[idx + 3] = 255; // A
            } else {
                rgba[idx + 3] = 0; // Transparent
            }
        }
    }
    Icon::from_rgba(rgba, size as u32, size as u32).map_err(|e| anyhow::anyhow!("Icon error: {}", e))
}
