# ImageCompare

A native Rust desktop app for comparing one or more images. Windows is the primary platform; the viewer and image loader also build on macOS and Linux. Windows Explorer integration is isolated in `src/shell.rs`.

## Use on this computer

Run `target\release\imagecompare.exe`, or select images in Explorer and choose **Compare with ImageCompare** from the right-click menu. On Windows 11 this entry may be under **Show more options**.

- Images open side by side. Larger selections scroll horizontally to keep each panel readable.
- Click **One by one**, then use **Left/Right** or the Previous/Next buttons. Navigation wraps at the ends.
- **Tab** switches between views. Entering one-by-one view always starts at image #1. Side-by-side panels have no selection or click action.
- **Add images**, **Ctrl+O** (Cmd+O on macOS), or dropping files adds images to the current comparison.
- **Clear** starts a fresh comparison. Duplicate paths are ignored.
- Opening the app without arguments shows a file-selection screen.

Supported formats: PNG, JPEG/JFIF, WebP, GIF, BMP, TIFF, ICO. EXIF orientation is applied. Animated/multi-page formats show the first frame/page. Images retain their aspect ratio, with a checkerboard behind transparency. Files that cannot be decoded show an error within their panel.

This is a visual comparison tool, not a pixel-difference or color-proofing tool. It does not edit source files. Images larger than the GPU texture limit (at most 8192 px per side) display a labeled scaled preview. Decoder allocation is limited to 512 MiB per image; very large images may be rejected. All loaded previews stay in GPU memory until Clear or close.

## Build

Install stable Rust and the native platform build tools. On Windows use the MSVC toolchain and Visual Studio's **Desktop development with C++** workload.

```powershell
cargo build --locked --release
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
```

This checkout also has an untracked local Rust toolchain in `.tools`. Use the wrapper if Rust is not on PATH:

```powershell
.\scripts\cargo.ps1 build --locked --release
.\scripts\cargo.ps1 test --locked
```

Pass paths as separate arguments; quote paths containing spaces:

```powershell
& .\target\release\imagecompare.exe 'C:\Pictures\first shot.png' 'C:\Pictures\second shot.jpg'
```

On Linux, install native development dependencies first (Ubuntu example):

```sh
sudo apt-get install build-essential pkg-config libxkbcommon-dev libwayland-dev libegl1-mesa-dev
cargo build --locked --release
./target/release/imagecompare ./first.png ./second.jpg
```

Linux file dialogs use the desktop's XDG portal. On macOS install Xcode command-line tools, then use the same Cargo commands. The CI workflow builds and tests all three platforms; only Windows has been exercised locally.

## Windows Explorer integration

After building, run in PowerShell:

```powershell
.\scripts\install-explorer.ps1
```

The script registers a per-user context-menu entry for supported image extensions and a COM local server pointing to `target\release\imagecompare.exe`. No administrator rights are needed. Re-run it after moving the executable, or pass `-ExePath 'C:\your location\imagecompare.exe'`.

To remove the menu:

```powershell
.\scripts\uninstall-explorer.ps1
```

The integration uses `IDropTarget` and `MultiSelectModel=Player`. Explorer delivers the entire selection as one `IDataObject` to one comparison window, preserving Unicode and avoiding per-file launches and command-line length limits. Each invocation opens a fresh comparison. It only registers ImageCompare's own verb and CLSID; existing default image applications are preserved.

Microsoft reference: [Choosing a shortcut menu implementation](https://learn.microsoft.com/en-us/windows/win32/shell/shortcut-choose-method) and [verb selection models](https://learn.microsoft.com/en-us/windows/win32/shell/how-to-employ-the-verb-selection-model).

## Code layout

- `src/model.rs`: view mode, ordered paths, deduplication and wraparound navigation; unit tested independently of the GUI.
- `src/loader.rs`: bounded decoding, orientation and preview sizing; tested with temporary images.
- `src/app.rs`: egui/eframe desktop UI, queued background decoding, file picker and drag-and-drop.
- `src/shell.rs`: Windows COM selection receiver.
- `scripts/`: local build helper and reversible per-user Explorer registration.
- `.github/workflows/build.yml`: build/test matrix for Windows, Linux and macOS.

`Cargo.lock` is included for reproducible application builds. `.tools`, `target`, and local test artifacts are ignored.

## Validation

Local Windows validation includes unit tests for navigation, deduplication, image decoding/errors and a real Windows Shell multi-selection data object, plus strict Clippy checks and an optimized release build.

Optional native rendering smoke check (opens and automatically closes a test window):

```powershell
.\scripts\cargo.ps1 run --locked --example render_check
```

It writes screenshots of side-by-side, one-by-one, next-image and wraparound states to `artifacts/`. To test COM activation after registering Explorer integration and running the rendering check:

```powershell
.\scripts\cargo.ps1 run --locked --example shell_check
```

This opens one comparison window with the two generated fixtures via the installed COM server. For an actual Explorer menu check, select those two files in `artifacts`, right-click, and choose **Compare with ImageCompare**.
