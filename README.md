# tatsu-audioapp

Windows audio routing tool that captures surround channels from the main audio output and routes them to a secondary audio device.

## Features

- WASAPI Loopback capture from the primary speakers (4ch+)
- Extracts Rear Left (RL) and Rear Right (RR) channels from multichannel audio
- Routes extracted channels to a secondary output device (e.g., 2nd speaker output)
- Automatic sample rate conversion (resampling)
- System tray application with full configuration menu
- Per-channel volume, mute, and source selection
- L/R swap and balance control
- Speaker test tones
- Configuration persistence (TOML)

## Use Case

This tool is designed for setups where:
- You have a 4+ channel audio output (e.g., Realtek with 4ch/5.1ch configuration)
- You want to route the rear surround channels to a separate physical output
- You don't want to add virtual audio devices

Example: Route rear speakers from main Speakers output to Realtek 2nd Output.

## Requirements

- Windows 10/11
- Audio device with 4+ channel support
- Secondary audio output device
- Rust toolchain (for building)

## Building

```powershell
cargo build --release
```

The executable will be at `target\release\tatsu-audioapp.exe`

## Usage

1. Run `tatsu-audioapp.exe`
2. The app starts in the system tray
3. Right-click the tray icon to access settings:
   - **Enable/Disable Routing** - Start/stop audio routing
   - **Swap L/R Channels** - Swap left and right output
   - **Source Device** - Select the device to capture from (loopback)
   - **Target Device** - Select the output device
   - **Master Volume** - Overall volume control
   - **Balance** - Left/Right balance adjustment
   - **Left/Right Speaker** - Per-channel settings (source, volume, mute)
   - **Speaker Test** - Test tone for each speaker

## Configuration

Settings are saved to `config.toml` in the same directory as the executable:

```toml
source_device = "Speakers (Realtek(R) Audio)"
target_device = "Realtek HD Audio 2nd output (Realtek(R) Audio)"
volume = 1.0
balance = 0.0
enabled = true
swap_channels = false

[left_channel]
source = "RL"
volume = 1.0
muted = false

[right_channel]
source = "RR"
volume = 1.0
muted = false
```

## Technical Details

- Uses WASAPI Loopback for low-latency audio capture
- Channel mapping: FL(0), FR(1), RL(2), RR(3)
- Automatic resampling when source and target sample rates differ (e.g., 192kHz to 48kHz)
- Ring buffer for thread-safe audio transfer between capture and playback

## 5.1ch Supported Applications

Applications that output true surround sound on Windows:

**Streaming:**
- Netflix (Windows Store app)
- Disney+ (Windows Store app)
- Plex, Jellyfin, Kodi

**Games:**
- Most PC games support 5.1ch natively

**Media Players:**
- VLC
- MPC-BE
- mpv

**Not Supported:**
- Web browsers (Chrome, Edge, Firefox) - HTML5 audio is stereo only
- Spotify, Amazon Music (Windows versions)

## License

MIT
