# COSMIC Calenderdot

A comic-style calendar applet for the COSMIC desktop, inspired by Dot.

## Features

- **Day badge** — panel button shows the current day number
- **Greeting** — time-based greeting with your username
- **Daily summary** — event count and total focus time
- **Monthly calendar grid** — day-of-week headers, event dots, today highlight
- **Today's agenda** — event cards with time badges, location chips, comic styling
- **Tomorrow preview** — upcoming events for the next day
- **World clock** — current time in NYC, BHO (Kolkata), and LDN
- **ICS support** — reads `.ics` calendar files from standard directories; falls back to demo events

## Build

```sh
just build
```

## Install

System-wide (requires `sudo`):

```sh
sudo just install
```

User-local:

```sh
PREFIX=$HOME/.local just install
```

## Add to Panel

1. Right-click the COSMIC panel → **Panel Settings**
2. Click **Add Applet** in the panel you want to modify
3. Search for **Calenderdot** and click **Add**

The day-number badge will appear in the panel. Click it to open the popover.

If the applet doesn't appear in the list, try restarting the panel:

```sh
killall cosmic-panel
```

Or log out and back in. Verify the desktop file is installed:

```sh
ls -la /usr/share/applications/com.cosmic.calenderdot.desktop
ls -la /usr/bin/cosmic-calenderdot
```

## Uninstall

```sh
sudo just uninstall
```

### Dependencies

- [just](https://github.com/casey/just) — command runner
- Rust 2021 edition
- libcosmic (fetched from git during build)
- Standard COSMIC desktop libraries
