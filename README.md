# nudge

A small macOS focus pill: it sits on top of everything, shows what you are working on,
and nudges you back when you drift. Built with Tauri 2 + Rust + React.

## Install (curl)

```bash
curl -fsSL https://raw.githubusercontent.com/peter711/nudge/main/install.sh | bash
```

System-wide install (`/Applications`):

```bash
curl -fsSL https://raw.githubusercontent.com/peter711/nudge/main/install.sh | bash -s -- --system
```

Uninstall (add `--purge` to delete the saved task too):

```bash
curl -fsSL https://raw.githubusercontent.com/peter711/nudge/main/install.sh | bash -s -- --uninstall
```

The installer downloads `nudge-universal.zip` from the latest GitHub release, installs to
`~/Applications` (or `/Applications` with `--system`), clears the quarantine flag and
launches the app when a GUI session is available. Requires macOS 14 or newer; releases
are universal (Apple Silicon + Intel).

The repository must be **public** for the unauthenticated curl flow. For a private repo,
install with `gh release download` instead and pass the archive to
`NUDGE_ZIP=/path/to/nudge-universal.zip ./install.sh`.

## Development

```bash
npm install
npm run tauri dev
```

Build an app bundle:

```bash
npm run tauri build          # src-tauri/target/release/bundle/macos/nudge.app + .dmg
```

Install a local build or a downloaded DMG (same mechanics, no GitHub involved):

```bash
scripts/install.sh
scripts/install.sh ~/Downloads/nudge_0.1.0_universal.dmg
scripts/install.sh path/to/nudge.app --no-launch
```

Universal local build (needs the Rust targets; `rustup` is not present when Rust comes
from Homebrew):

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run tauri build -- --target universal-apple-darwin
```

Requirements and the manual QA checklist live in `.docs/`. The visual prototype (throwaway, plain HTML) is still around:

```bash
npm run proto                # opens prototype/pill.html
```

## Releases

`.github/workflows/release.yml` builds a universal `.app` + `.dmg` on every `v*` tag and
attaches `nudge-universal.zip`, its SHA-256, `install.sh` and the DMG to the GitHub
release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## What it does

- Floating pill, click to edit, `Enter` commits, `Esc` reverts, click outside commits.
- Drag anywhere (4 px threshold separates a click from a drag); position is remembered
  per monitor and clamped to the visible area.
- Lives in the menu bar only — `ActivationPolicy::Accessory`, no Dock icon.
- Menu bar: current task, *Change task…*, *Show/Hide pill*, reminder interval, *Quit*.
- `Cmd+Shift+Space` opens the editor from any app; focus returns to the previous app
  after you commit.
- Reminders every 15/30/45/60 minutes (or off): native notification + a short pulse of
  the dot. The clock survives restarts and does not reset on launch.

State lives in
`~/Library/Application Support/peter711.nudge/state.json` (atomic write, Rust-owned).

## Implementation notes

- The window is an `NSPanel` (`tauri-nspanel`) with `canJoinAllSpaces` +
  `fullScreenAuxiliary`, which is what makes the pill float above fullscreen apps.
  A plain `alwaysOnTop` window cannot do that.
- Background blur is the native `NSVisualEffectView` (`window-vibrancy`, HUD material).
  CSS `backdrop-filter` does not blur the desktop behind a transparent WKWebView.
- The pill window is resized to exactly the pill's content, so the transparent area
  never swallows clicks.
- Reminder timing runs in Rust (`tokio`), not JS: WKWebView throttles timers when the
  window is hidden or occluded.
- Notifications are passive. Action buttons exist in the plugin, but macOS buries them
  in an "Options" submenu, so the pulse is the real signal.
- `macOSPrivateApi` is not set — it is a no-op since Tauri 2.12.1.
- Global shortcut uses `global-hotkey` 0.10 directly (not the plugin) so a conflict is
  reported instead of silently swallowing the hotkey.

## Known gaps

- Lock/sleep is handled heuristically (a long timer gap skips one nudge); explicit
  `NSWorkspace` sleep/lock observers are not wired yet.
- The pill is always dark (HUD material). Adaptive light/dark was not needed for the
  chosen design, but is a one-line material change if wanted.
- Tray icon is a generated 32×32 template PNG; retina sharpness still to be eyeballed.
- Releases are unsigned/notarised: curl installs run fine (no quarantine), DMGs
  downloaded through a browser need the installer or a manual
  `xattr -dr com.apple.quarantine`.

Regenerate the icons with `npm run icons` (tray template + app icon/icns).
