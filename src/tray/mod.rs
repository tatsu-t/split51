use anyhow::Result;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem, MenuId};
use tray_icon::{TrayIcon, TrayIconBuilder, Icon};
use std::collections::HashMap;
use crate::config::ChannelSource;

pub enum TrayCommand {
    ToggleEnabled,
    ToggleSwapChannels,
    ToggleCloneStereo,
    ToggleStartup,
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
    // DSP commands
    SetDelayMs(f32),
    ToggleEq,
    SetEqLow(f32),
    SetEqMid(f32),
    SetEqHigh(f32),
    ToggleUpmix,
    SetUpmixStrength(f32),
    ToggleSyncMasterVolume,
    Quit,
}

pub struct TrayManager {
    tray_icon: TrayIcon,
    toggle_item: MenuItem,
    swap_item: CheckMenuItem,
    clone_stereo_item: CheckMenuItem,
    startup_item: CheckMenuItem,
    left_mute_item: CheckMenuItem,
    right_mute_item: CheckMenuItem,
    eq_item: CheckMenuItem,
    sync_master_item: CheckMenuItem,
    upmix_item: CheckMenuItem,
    volume_items: HashMap<MenuId, f32>,
    balance_items: HashMap<MenuId, f32>,
    left_volume_items: HashMap<MenuId, f32>,
    right_volume_items: HashMap<MenuId, f32>,
    delay_items: HashMap<MenuId, f32>,
    eq_low_items: HashMap<MenuId, f32>,
    eq_mid_items: HashMap<MenuId, f32>,
    eq_high_items: HashMap<MenuId, f32>,
    source_device_items: HashMap<MenuId, String>,
    target_device_items: HashMap<MenuId, String>,
    source_menu_items: Vec<(MenuId, MenuItem, String)>,
    target_menu_items: Vec<(MenuId, MenuItem, String)>,
    // For updating checkmarks
    delay_menu_items: Vec<(MenuId, MenuItem, i32)>,
    eq_low_menu_items: Vec<(MenuId, MenuItem, i32)>,
    eq_mid_menu_items: Vec<(MenuId, MenuItem, i32)>,
    eq_high_menu_items: Vec<(MenuId, MenuItem, i32)>,
    upmix_strength_items: HashMap<MenuId, f32>,
    upmix_strength_menu_items: Vec<(MenuId, MenuItem, i32)>,
    toggle_id: MenuId,
    swap_id: MenuId,
    clone_stereo_id: MenuId,
    startup_id: MenuId,
    quit_id: MenuId,
    test_main_left_id: MenuId,
    test_main_right_id: MenuId,
    test_sub_left_id: MenuId,
    test_sub_right_id: MenuId,
    left_fl_id: MenuId,
    left_fr_id: MenuId,
    left_rl_id: MenuId,
    left_rr_id: MenuId,
    right_fl_id: MenuId,
    right_fr_id: MenuId,
    right_rl_id: MenuId,
    right_rr_id: MenuId,
    left_mute_id: MenuId,
    right_mute_id: MenuId,
    eq_id: MenuId,
    upmix_id: MenuId,
    sync_master_id: MenuId,
}

impl TrayManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_devices: &[String],
        target_devices: &[String],
        current_source: Option<&str>,
        current_target: Option<&str>,
        current_volume: f32,
        current_balance: f32,
        current_left_source: ChannelSource,
        current_right_source: ChannelSource,
        current_left_volume: f32,
        current_right_volume: f32,
        left_muted: bool,
        right_muted: bool,
        enabled: bool,
        swap_channels: bool,
        clone_stereo: bool,
        startup_enabled: bool,
        // DSP settings
        delay_ms: f32,
        eq_enabled: bool,
        eq_low: f32,
        eq_mid: f32,
        eq_high: f32,
        upmix_enabled: bool,
        upmix_strength: f32,
        sync_master_volume: bool,
    ) -> Result<Self> {
        // Create menu items
        let toggle_text = if enabled { "Disable Routing" } else { "Enable Routing" };
        let toggle_item = MenuItem::new(toggle_text, true, None);

        // Swap channels checkbox
        let swap_item = CheckMenuItem::new("Swap L/R Channels", true, swap_channels, None);
        
        // Clone stereo checkbox (FL/FR instead of RL/RR)
        let clone_stereo_item = CheckMenuItem::new("Clone Stereo (FL/FR)", true, clone_stereo, None);
        
        // Startup checkbox
        let startup_item = CheckMenuItem::new("Start with Windows", true, startup_enabled, None);

        // Source device submenu with checkmarks
        let source_submenu = Submenu::new("Source Device (Loopback)", true);
        let mut source_device_items = HashMap::new();
        let mut source_menu_items = Vec::new();
        for device in source_devices {
            let is_current = current_source.map(|s| s == device).unwrap_or(false);
            let label = if is_current { format!("[*] {}", device) } else { device.clone() };
            let item = MenuItem::new(&label, true, None);
            source_device_items.insert(item.id().clone(), device.clone());
            source_menu_items.push((item.id().clone(), item.clone(), device.clone()));
            source_submenu.append(&item)?;
        }

        // Target device submenu with checkmarks
        let target_submenu = Submenu::new("Target Device (Output)", true);
        let mut target_device_items = HashMap::new();
        let mut target_menu_items = Vec::new();
        for device in target_devices {
            let is_current = current_target.map(|t| t == device).unwrap_or(false);
            let label = if is_current { format!("[*] {}", device) } else { device.clone() };
            let item = MenuItem::new(&label, true, None);
            target_device_items.insert(item.id().clone(), device.clone());
            target_menu_items.push((item.id().clone(), item.clone(), device.clone()));
            target_submenu.append(&item)?;
        }

        // Master Volume submenu
        let volume_submenu = Submenu::new("Master Volume", true);
        let mut volume_items = HashMap::new();
        let current_vol_pct = (current_volume * 100.0).round() as i32;
        for v in [25, 50, 75, 100, 125, 150] {
            let is_current = v == current_vol_pct;
            let label = if is_current { format!("[*] {}%", v) } else { format!("{}%", v) };
            let item = MenuItem::new(&label, true, None);
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
            let is_current = (current_balance - value).abs() < 0.1;
            let text = if is_current { format!("[*] {}", label) } else { label.to_string() };
            let item = MenuItem::new(&text, true, None);
            balance_items.insert(item.id().clone(), value);
            balance_submenu.append(&item)?;
        }

        // Left Speaker submenu
        let left_submenu = Submenu::new("Left Speaker", true);
        let left_fl_label = if matches!(current_left_source, ChannelSource::FL) { "[*] Source: FL (Front Left)" } else { "Source: FL (Front Left)" };
        let left_fr_label = if matches!(current_left_source, ChannelSource::FR) { "[*] Source: FR (Front Right)" } else { "Source: FR (Front Right)" };
        let left_rl_label = if matches!(current_left_source, ChannelSource::RL) { "[*] Source: RL (Rear Left)" } else { "Source: RL (Rear Left)" };
        let left_rr_label = if matches!(current_left_source, ChannelSource::RR) { "[*] Source: RR (Rear Right)" } else { "Source: RR (Rear Right)" };
        let left_fl = MenuItem::new(left_fl_label, true, None);
        let left_fr = MenuItem::new(left_fr_label, true, None);
        let left_rl = MenuItem::new(left_rl_label, true, None);
        let left_rr = MenuItem::new(left_rr_label, true, None);
        let left_mute = CheckMenuItem::new("Mute", true, left_muted, None);
        left_submenu.append(&left_fl)?;
        left_submenu.append(&left_fr)?;
        left_submenu.append(&left_rl)?;
        left_submenu.append(&left_rr)?;
        left_submenu.append(&PredefinedMenuItem::separator())?;
        left_submenu.append(&left_mute)?;
        
        // Left volume
        let left_vol_submenu = Submenu::new("Volume", true);
        let mut left_volume_items = HashMap::new();
        let current_left_vol_pct = (current_left_volume * 100.0).round() as i32;
        for v in [25, 50, 75, 100, 125, 150] {
            let is_current = v == current_left_vol_pct;
            let label = if is_current { format!("[*] {}%", v) } else { format!("{}%", v) };
            let item = MenuItem::new(&label, true, None);
            left_volume_items.insert(item.id().clone(), v as f32 / 100.0);
            left_vol_submenu.append(&item)?;
        }
        left_submenu.append(&left_vol_submenu)?;

        // Right Speaker submenu
        let right_submenu = Submenu::new("Right Speaker", true);
        let right_fl_label = if matches!(current_right_source, ChannelSource::FL) { "[*] Source: FL (Front Left)" } else { "Source: FL (Front Left)" };
        let right_fr_label = if matches!(current_right_source, ChannelSource::FR) { "[*] Source: FR (Front Right)" } else { "Source: FR (Front Right)" };
        let right_rl_label = if matches!(current_right_source, ChannelSource::RL) { "[*] Source: RL (Rear Left)" } else { "Source: RL (Rear Left)" };
        let right_rr_label = if matches!(current_right_source, ChannelSource::RR) { "[*] Source: RR (Rear Right)" } else { "Source: RR (Rear Right)" };
        let right_fl = MenuItem::new(right_fl_label, true, None);
        let right_fr = MenuItem::new(right_fr_label, true, None);
        let right_rl = MenuItem::new(right_rl_label, true, None);
        let right_rr = MenuItem::new(right_rr_label, true, None);
        let right_mute = CheckMenuItem::new("Mute", true, right_muted, None);
        right_submenu.append(&right_fl)?;
        right_submenu.append(&right_fr)?;
        right_submenu.append(&right_rl)?;
        right_submenu.append(&right_rr)?;
        right_submenu.append(&PredefinedMenuItem::separator())?;
        right_submenu.append(&right_mute)?;

        // Right volume
        let right_vol_submenu = Submenu::new("Volume", true);
        let mut right_volume_items = HashMap::new();
        let current_right_vol_pct = (current_right_volume * 100.0).round() as i32;
        for v in [25, 50, 75, 100, 125, 150] {
            let is_current = v == current_right_vol_pct;
            let label = if is_current { format!("[*] {}%", v) } else { format!("{}%", v) };
            let item = MenuItem::new(&label, true, None);
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

        // DSP submenu
        let dsp_submenu = Submenu::new("DSP Effects", true);
        
        // Delay submenu
        let delay_submenu = Submenu::new("Delay", true);
        let mut delay_items = HashMap::new();
        let mut delay_menu_items = Vec::new();
        let current_delay_ms = delay_ms.round() as i32;
        for ms in [0, 10, 20, 50, 100, 200] {
            let is_current = ms == current_delay_ms;
            let label = if is_current { format!("[*] {} ms", ms) } else { format!("{} ms", ms) };
            let item = MenuItem::new(&label, true, None);
            delay_items.insert(item.id().clone(), ms as f32);
            delay_menu_items.push((item.id().clone(), item.clone(), ms));
            delay_submenu.append(&item)?;
        }
        dsp_submenu.append(&delay_submenu)?;
        
        // EQ checkbox
        let eq_item = CheckMenuItem::new("Equalizer", true, eq_enabled, None);
        dsp_submenu.append(&eq_item)?;
        
        // EQ Low submenu
        let eq_low_submenu = Submenu::new("EQ Low (200Hz)", true);
        let mut eq_low_items = HashMap::new();
        let mut eq_low_menu_items = Vec::new();
        let current_low = eq_low.round() as i32;
        for db in [-12, -6, -3, 0, 3, 6, 12] {
            let is_current = db == current_low;
            let label = if is_current { format!("[*] {:+} dB", db) } else { format!("{:+} dB", db) };
            let item = MenuItem::new(&label, true, None);
            eq_low_items.insert(item.id().clone(), db as f32);
            eq_low_menu_items.push((item.id().clone(), item.clone(), db));
            eq_low_submenu.append(&item)?;
        }
        dsp_submenu.append(&eq_low_submenu)?;
        
        // EQ Mid submenu
        let eq_mid_submenu = Submenu::new("EQ Mid (1kHz)", true);
        let mut eq_mid_items = HashMap::new();
        let mut eq_mid_menu_items = Vec::new();
        let current_mid = eq_mid.round() as i32;
        for db in [-12, -6, -3, 0, 3, 6, 12] {
            let is_current = db == current_mid;
            let label = if is_current { format!("[*] {:+} dB", db) } else { format!("{:+} dB", db) };
            let item = MenuItem::new(&label, true, None);
            eq_mid_items.insert(item.id().clone(), db as f32);
            eq_mid_menu_items.push((item.id().clone(), item.clone(), db));
            eq_mid_submenu.append(&item)?;
        }
        dsp_submenu.append(&eq_mid_submenu)?;
        
        // EQ High submenu
        let eq_high_submenu = Submenu::new("EQ High (4kHz)", true);
        let mut eq_high_items = HashMap::new();
        let mut eq_high_menu_items = Vec::new();
        let current_high = eq_high.round() as i32;
        for db in [-12, -6, -3, 0, 3, 6, 12] {
            let is_current = db == current_high;
            let label = if is_current { format!("[*] {:+} dB", db) } else { format!("{:+} dB", db) };
            let item = MenuItem::new(&label, true, None);
            eq_high_items.insert(item.id().clone(), db as f32);
            eq_high_menu_items.push((item.id().clone(), item.clone(), db));
            eq_high_submenu.append(&item)?;
        }
        dsp_submenu.append(&eq_high_submenu)?;
        
        dsp_submenu.append(&PredefinedMenuItem::separator())?;
        
        // Upmix checkbox
        let upmix_item = CheckMenuItem::new("Pseudo Surround (Upmix)", true, upmix_enabled, None);
        dsp_submenu.append(&upmix_item)?;
        
        // Upmix strength submenu
        let upmix_strength_submenu = Submenu::new("Upmix Volume", true);
        let mut upmix_strength_items = HashMap::new();
        let mut upmix_strength_menu_items = Vec::new();
        let current_strength = (upmix_strength * 10.0).round() as i32;  // Store as x10 int
        for strength in [10, 20, 40, 60, 80, 100] {  // 1x, 2x, 4x, 6x, 8x, 10x
            let is_current = strength == current_strength;
            let label = if is_current { format!("[*] {}x", strength / 10) } else { format!("{}x", strength / 10) };
            let item = MenuItem::new(&label, true, None);
            upmix_strength_items.insert(item.id().clone(), strength as f32 / 10.0);
            upmix_strength_menu_items.push((item.id().clone(), item.clone(), strength));
            upmix_strength_submenu.append(&item)?;
        }
        dsp_submenu.append(&upmix_strength_submenu)?;
        
        dsp_submenu.append(&PredefinedMenuItem::separator())?;
        
        // Sync master volume checkbox
        let sync_master_item = CheckMenuItem::new("Sync Master Volume", true, sync_master_volume, None);
        dsp_submenu.append(&sync_master_item)?;

        let quit_item = MenuItem::new("Quit", true, None);

        // Store IDs for event handling
        let toggle_id = toggle_item.id().clone();
        let swap_id = swap_item.id().clone();
        let clone_stereo_id = clone_stereo_item.id().clone();
        let startup_id = startup_item.id().clone();
        let quit_id = quit_item.id().clone();
        let test_main_left_id = test_main_left.id().clone();
        let test_main_right_id = test_main_right.id().clone();
        let test_sub_left_id = test_sub_left.id().clone();
        let test_sub_right_id = test_sub_right.id().clone();
        let left_fl_id = left_fl.id().clone();
        let left_fr_id = left_fr.id().clone();
        let left_rl_id = left_rl.id().clone();
        let left_rr_id = left_rr.id().clone();
        let right_fl_id = right_fl.id().clone();
        let right_fr_id = right_fr.id().clone();
        let right_rl_id = right_rl.id().clone();
        let right_rr_id = right_rr.id().clone();
        let left_mute_id = left_mute.id().clone();
        let right_mute_id = right_mute.id().clone();
        let eq_id = eq_item.id().clone();
        let upmix_id = upmix_item.id().clone();
        let sync_master_id = sync_master_item.id().clone();

        // Build menu
        let menu = Menu::new();
        menu.append(&toggle_item)?;
        menu.append(&swap_item)?;
        menu.append(&clone_stereo_item)?;
        menu.append(&startup_item)?;
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
        menu.append(&dsp_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&test_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit_item)?;

        // Create tray icon
        let icon = create_default_icon()?;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("split51 - 5.1ch Audio Splitter")
            .with_icon(icon)
            .with_menu(Box::new(menu.clone()))
            .build()?;

        Ok(Self {
            tray_icon,
            toggle_item,
            swap_item,
            clone_stereo_item,
            startup_item,
            left_mute_item: left_mute,
            right_mute_item: right_mute,
            volume_items,
            balance_items,
            left_volume_items,
            right_volume_items,
            source_device_items,
            target_device_items,
            source_menu_items,
            target_menu_items,
            toggle_id,
            swap_id,
            clone_stereo_id,
            startup_id,
            quit_id,
            test_main_left_id,
            test_main_right_id,
            test_sub_left_id,
            test_sub_right_id,
            left_fl_id,
            left_fr_id,
            left_rl_id,
            left_rr_id,
            right_fl_id,
            right_fr_id,
            right_rl_id,
            right_rr_id,
            left_mute_id,
            right_mute_id,
            eq_item,
            upmix_item,
            delay_items,
            eq_low_items,
            eq_mid_items,
            eq_high_items,
            delay_menu_items,
            eq_low_menu_items,
            eq_mid_menu_items,
            eq_high_menu_items,
            upmix_strength_items,
            upmix_strength_menu_items,
            eq_id,
            upmix_id,
            sync_master_item,
            sync_master_id,
        })
    }

    /// Update delay menu checkmarks
    pub fn set_delay_ms(&mut self, ms: f32) {
        let current = ms.round() as i32;
        for (_, item, value) in &self.delay_menu_items {
            let is_current = *value == current;
            let label = if is_current { format!("[*] {} ms", value) } else { format!("{} ms", value) };
            item.set_text(&label);
        }
    }

    /// Update Upmix strength checkmarks
    pub fn set_upmix_strength(&mut self, strength: f32) {
        let current = (strength * 10.0).round() as i32;
        for (_, item, value) in &self.upmix_strength_menu_items {
            let is_current = *value == current;
            let label = if is_current { format!("[*] {}x", value / 10) } else { format!("{}x", value / 10) };
            item.set_text(&label);
        }
    }

    pub fn set_sync_master_volume(&mut self, enabled: bool) {
        self.sync_master_item.set_checked(enabled);
    }

    /// Update EQ Low checkmarks
    pub fn set_eq_low(&mut self, db: f32) {
        let current = db.round() as i32;
        for (_, item, value) in &self.eq_low_menu_items {
            let is_current = *value == current;
            let label = if is_current { format!("[*] {:+} dB", value) } else { format!("{:+} dB", value) };
            item.set_text(&label);
        }
    }

    /// Update EQ Mid checkmarks
    pub fn set_eq_mid(&mut self, db: f32) {
        let current = db.round() as i32;
        for (_, item, value) in &self.eq_mid_menu_items {
            let is_current = *value == current;
            let label = if is_current { format!("[*] {:+} dB", value) } else { format!("{:+} dB", value) };
            item.set_text(&label);
        }
    }

    /// Update EQ High checkmarks
    pub fn set_eq_high(&mut self, db: f32) {
        let current = db.round() as i32;
        for (_, item, value) in &self.eq_high_menu_items {
            let is_current = *value == current;
            let label = if is_current { format!("[*] {:+} dB", value) } else { format!("{:+} dB", value) };
            item.set_text(&label);
        }
    }

    /// Update tray icon and tooltip based on enabled state
    pub fn set_enabled(&mut self, enabled: bool) {
        let text = if enabled { "Disable Routing" } else { "Enable Routing" };
        self.toggle_item.set_text(text);
        
        let tooltip = if enabled {
            "split51 - Routing Active"
        } else {
            "split51 - Routing Disabled"
        };
        self.tray_icon.set_tooltip(Some(tooltip)).ok();
        
        // Change icon color based on state
        if let Ok(icon) = if enabled { create_enabled_icon() } else { create_disabled_icon() } {
            self.tray_icon.set_icon(Some(icon)).ok();
        }
    }

    /// Update startup checkbox
    pub fn set_startup(&mut self, enabled: bool) {
        self.startup_item.set_checked(enabled);
    }

    /// Update swap checkbox
    pub fn set_swap(&mut self, swap: bool) {
        self.swap_item.set_checked(swap);
    }

    /// Update clone stereo checkbox
    pub fn set_clone_stereo(&mut self, enabled: bool) {
        self.clone_stereo_item.set_checked(enabled);
    }

    /// Update EQ checkbox
    pub fn set_eq_enabled(&mut self, enabled: bool) {
        self.eq_item.set_checked(enabled);
    }

    /// Update upmix checkbox
    pub fn set_upmix_enabled(&mut self, enabled: bool) {
        self.upmix_item.set_checked(enabled);
    }

    /// Update mute checkboxes
    pub fn set_left_mute(&mut self, muted: bool) {
        self.left_mute_item.set_checked(muted);
    }

    pub fn set_right_mute(&mut self, muted: bool) {
        self.right_mute_item.set_checked(muted);
    }

    /// Update source device menu checkmarks
    pub fn set_current_source(&mut self, device: Option<&str>) {
        for (_, item, name) in &self.source_menu_items {
            let is_current = device.map(|d| d == name).unwrap_or(false);
            let label = if is_current { format!("[*] {}", name) } else { name.clone() };
            item.set_text(&label);
        }
    }

    /// Update target device menu checkmarks
    pub fn set_current_target(&mut self, device: Option<&str>) {
        for (_, item, name) in &self.target_menu_items {
            let is_current = device.map(|d| d == name).unwrap_or(false);
            let label = if is_current { format!("[*] {}", name) } else { name.clone() };
            item.set_text(&label);
        }
    }

    pub fn handle_menu_event(&self, event: &MenuEvent) -> Option<TrayCommand> {
        if event.id == self.toggle_id {
            Some(TrayCommand::ToggleEnabled)
        } else if event.id == self.swap_id {
            Some(TrayCommand::ToggleSwapChannels)
        } else if event.id == self.clone_stereo_id {
            Some(TrayCommand::ToggleCloneStereo)
        } else if event.id == self.startup_id {
            Some(TrayCommand::ToggleStartup)
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
        } else if event.id == self.left_fl_id {
            Some(TrayCommand::SetLeftSource(ChannelSource::FL))
        } else if event.id == self.left_fr_id {
            Some(TrayCommand::SetLeftSource(ChannelSource::FR))
        } else if event.id == self.left_rl_id {
            Some(TrayCommand::SetLeftSource(ChannelSource::RL))
        } else if event.id == self.left_rr_id {
            Some(TrayCommand::SetLeftSource(ChannelSource::RR))
        } else if event.id == self.right_fl_id {
            Some(TrayCommand::SetRightSource(ChannelSource::FL))
        } else if event.id == self.right_fr_id {
            Some(TrayCommand::SetRightSource(ChannelSource::FR))
        } else if event.id == self.right_rl_id {
            Some(TrayCommand::SetRightSource(ChannelSource::RL))
        } else if event.id == self.right_rr_id {
            Some(TrayCommand::SetRightSource(ChannelSource::RR))
        } else if event.id == self.left_mute_id {
            Some(TrayCommand::ToggleLeftMute)
        } else if event.id == self.right_mute_id {
            Some(TrayCommand::ToggleRightMute)
        } else if event.id == self.eq_id {
            Some(TrayCommand::ToggleEq)
        } else if event.id == self.upmix_id {
            Some(TrayCommand::ToggleUpmix)
        } else if event.id == self.sync_master_id {
            Some(TrayCommand::ToggleSyncMasterVolume)
        } else if let Some(&vol) = self.volume_items.get(&event.id) {
            Some(TrayCommand::SetVolume(vol))
        } else if let Some(&bal) = self.balance_items.get(&event.id) {
            Some(TrayCommand::SetBalance(bal))
        } else if let Some(&vol) = self.left_volume_items.get(&event.id) {
            Some(TrayCommand::SetLeftVolume(vol))
        } else if let Some(&vol) = self.right_volume_items.get(&event.id) {
            Some(TrayCommand::SetRightVolume(vol))
        } else if let Some(&delay) = self.delay_items.get(&event.id) {
            Some(TrayCommand::SetDelayMs(delay))
        } else if let Some(&db) = self.eq_low_items.get(&event.id) {
            Some(TrayCommand::SetEqLow(db))
        } else if let Some(&db) = self.eq_mid_items.get(&event.id) {
            Some(TrayCommand::SetEqMid(db))
        } else if let Some(&db) = self.eq_high_items.get(&event.id) {
            Some(TrayCommand::SetEqHigh(db))
        } else if let Some(&strength) = self.upmix_strength_items.get(&event.id) {
            Some(TrayCommand::SetUpmixStrength(strength))
        } else if let Some(device) = self.source_device_items.get(&event.id) {
            Some(TrayCommand::SelectSourceDevice(device.clone()))
        } else if let Some(device) = self.target_device_items.get(&event.id) {
            Some(TrayCommand::SelectTargetDevice(device.clone()))
        } else {
            None
        }
    }
}

fn create_default_icon() -> Result<Icon> {
    create_enabled_icon()
}

fn create_enabled_icon() -> Result<Icon> {
    // Create a simple 16x16 RGBA icon (green - active)
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
                rgba[idx + 1] = 200; // G (brighter green for enabled)
                rgba[idx + 2] = 80;  // B
                rgba[idx + 3] = 255; // A
            } else {
                rgba[idx + 3] = 0; // Transparent
            }
        }
    }
    Icon::from_rgba(rgba, size as u32, size as u32).map_err(|e| anyhow::anyhow!("Icon error: {}", e))
}

fn create_disabled_icon() -> Result<Icon> {
    // Create a simple 16x16 RGBA icon (gray - disabled)
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
                rgba[idx] = 120;     // R (gray)
                rgba[idx + 1] = 120; // G
                rgba[idx + 2] = 120; // B
                rgba[idx + 3] = 255; // A
            } else {
                rgba[idx + 3] = 0; // Transparent
            }
        }
    }
    Icon::from_rgba(rgba, size as u32, size as u32).map_err(|e| anyhow::anyhow!("Icon error: {}", e))
}
