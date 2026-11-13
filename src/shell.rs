// Windows-only COM drop target: Explorer sends one IDataObject containing the
// entire selection. No per-file processes, timing heuristics, or command-line limits.
use std::{path::PathBuf, sync::mpsc, time::Duration};
use windows::{
    core::{implement, IUnknown, Interface, Ref, Result, GUID},
    Win32::{
        Foundation::{CLASS_E_NOAGGREGATION, E_POINTER, POINTL},
        System::{Com::*, Ole::*, SystemServices::MODIFIERKEYS_FLAGS},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

pub const CLSID: GUID = GUID::from_u128(0x9a29afc8_8445_49ee_b5d4_916e15cd4792);

#[implement(IDropTarget)]
struct SelectionTarget {
    sender: mpsc::Sender<Vec<PathBuf>>,
    quit: bool,
}

impl IDropTarget_Impl for SelectionTarget_Impl {
    fn DragEnter(
        &self,
        _: Ref<'_, IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        _: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        if effect.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *effect = DROPEFFECT_COPY;
        }
        Ok(())
    }
    fn DragOver(&self, _: MODIFIERKEYS_FLAGS, _: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        if effect.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *effect = DROPEFFECT_COPY;
        }
        Ok(())
    }
    fn DragLeave(&self) -> Result<()> {
        Ok(())
    }
    fn Drop(
        &self,
        data: Ref<'_, IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        _: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        if effect.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *effect = DROPEFFECT_NONE;
        }
        let data = data.ok()?;
        let items: IShellItemArray = unsafe { SHCreateShellItemArrayFromDataObject(data)? };
        let mut paths = Vec::new();
        for index in 0..unsafe { items.GetCount()? } {
            let item = unsafe { items.GetItemAt(index)? };
            let name = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH)? };
            // Keep UTF-16 paths lossless, including names not representable as UTF-8.
            use std::os::windows::ffi::OsStringExt;
            let path = unsafe { std::ffi::OsString::from_wide(name.as_wide()) };
            unsafe {
                CoTaskMemFree(Some(name.0.cast()));
            }
            paths.push(PathBuf::from(path));
        }
        self.sender
            .send(paths)
            .map_err(|_| windows::core::Error::from(E_POINTER))?;
        unsafe {
            *effect = DROPEFFECT_COPY;
        }
        if self.quit {
            unsafe {
                PostQuitMessage(0);
            }
        }
        Ok(())
    }
}

#[implement(IClassFactory)]
struct SelectionFactory {
    sender: mpsc::Sender<Vec<PathBuf>>,
}

impl IClassFactory_Impl for SelectionFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<'_, IUnknown>,
        iid: *const GUID,
        object: *mut *mut std::ffi::c_void,
    ) -> Result<()> {
        if object.is_null() || iid.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *object = std::ptr::null_mut();
        }
        if outer.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let target: IDropTarget = SelectionTarget {
            sender: self.sender.clone(),
            quit: true,
        }
        .into();
        unsafe { target.query(iid, object).ok() }
    }
    fn LockServer(&self, _: windows::core::BOOL) -> Result<()> {
        Ok(())
    }
}

pub fn receive_selection() -> std::result::Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let (sender, receiver) = mpsc::channel();
    // Keep COM apartment and message pumping off the UI's main thread.
    std::thread::spawn(move || unsafe {
        let result = (|| -> Result<()> {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
            let factory: IClassFactory = SelectionFactory { sender }.into();
            let cookie =
                CoRegisterClassObject(&CLSID, &factory, CLSCTX_LOCAL_SERVER, REGCLS_SINGLEUSE)?;
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            CoRevokeClassObject(cookie)?;
            CoUninitialize();
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("Explorer integration: {error}");
        }
    });
    Ok(receiver.recv_timeout(Duration::from_secs(30))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::HSTRING;

    #[test]
    fn drop_delivers_whole_selection_in_order_with_unicode_and_spaces() {
        let directory = tempfile::tempdir().unwrap();
        let paths: Vec<_> = ["first image.png", "second 日本.jpg", "third.webp"]
            .iter()
            .map(|name| directory.path().join(name))
            .collect();
        for path in &paths {
            std::fs::write(path, b"shell path fixture").unwrap();
        }
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
            {
                let mut pidls = Vec::new();
                for path in &paths {
                    let mut pidl = std::ptr::null_mut();
                    SHParseDisplayName(&HSTRING::from(path.as_os_str()), None, &mut pidl, 0, None)
                        .unwrap();
                    pidls.push(pidl);
                }
                let pointers: Vec<_> = pidls
                    .iter()
                    .map(|p| *p as *const Common::ITEMIDLIST)
                    .collect();
                let items = SHCreateShellItemArrayFromIDLists(&pointers).unwrap();
                for pidl in pidls {
                    CoTaskMemFree(Some(pidl.cast()));
                }
                let data: IDataObject = items.BindToHandler(None, &BHID_DataObject).unwrap();
                let (sender, receiver) = mpsc::channel();
                let target: IDropTarget = SelectionTarget {
                    sender,
                    quit: false,
                }
                .into();
                let mut effect = DROPEFFECT_COPY;
                target
                    .DragEnter(&data, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut effect)
                    .unwrap();
                target
                    .Drop(&data, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut effect)
                    .unwrap();
                let received = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
                let actual: Vec<_> = received.iter().map(|p| p.canonicalize().unwrap()).collect();
                let expected: Vec<_> = paths.iter().map(|p| p.canonicalize().unwrap()).collect();
                assert_eq!(actual, expected);
                assert_eq!(effect, DROPEFFECT_COPY);
                assert!(receiver.try_recv().is_err(), "one batch per selection");
            }
            CoUninitialize();
        }
    }
}
