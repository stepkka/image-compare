//! Exercises the installed out-of-process COM server with a real Shell selection.
//! Run after install-explorer.ps1. Opens one ImageCompare window containing fixtures.
#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    use windows::{
        core::HSTRING,
        Win32::{
            Foundation::POINTL,
            System::{Com::*, Ole::*, SystemServices::MODIFIERKEYS_FLAGS},
            UI::Shell::*,
        },
    };
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        {
            let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("artifacts");
            let paths = [root.join("first image.png"), root.join("second image.png")];
            let mut pidls = Vec::new();
            for path in &paths {
                assert!(path.exists(), "Run the render_check example first");
                let mut pidl = std::ptr::null_mut();
                SHParseDisplayName(&HSTRING::from(path.as_os_str()), None, &mut pidl, 0, None)?;
                pidls.push(pidl);
            }
            let pointers: Vec<_> = pidls
                .iter()
                .map(|p| *p as *const Common::ITEMIDLIST)
                .collect();
            let items = SHCreateShellItemArrayFromIDLists(&pointers)?;
            for pidl in pidls {
                CoTaskMemFree(Some(pidl.cast()));
            }
            let data: IDataObject = items.BindToHandler(None, &BHID_DataObject)?;
            let target: IDropTarget =
                CoCreateInstance(&imagecompare::shell::CLSID, None, CLSCTX_LOCAL_SERVER)?;
            let mut effect = DROPEFFECT_COPY;
            target.DragEnter(&data, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut effect)?;
            target.Drop(&data, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut effect)?;
            assert_eq!(effect, DROPEFFECT_COPY);
            println!("Installed COM server accepted the complete two-image selection.");
        }
        CoUninitialize();
    }
    Ok(())
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Explorer integration is Windows-only.");
}
