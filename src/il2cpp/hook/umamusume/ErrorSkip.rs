#![cfg(target_os = "windows")]

use crate::il2cpp::{symbols::{get_class, get_method_addr}, types::*};

type GotoTitleOnErrorFn = extern "C" fn(text: *mut Il2CppString) -> ();
static mut ORIG: usize = 0;

extern "C" fn GotoTitleOnError_hook(text: *mut Il2CppString) {
    let msg = if text.is_null() {
        String::new()
    } else {
        unsafe {
            let p = text as usize;
            let len = *((p + 16) as *const i32);
            if len > 0 && len <= 400 {
                let chars = std::slice::from_raw_parts((p + 20) as *const u16, len as usize);
                String::from_utf16_lossy(chars)
            } else { String::new() }
        }
    };
    warn!("[errskip] GotoTitleOnError suppressed: {}", msg);
    // TLG 风格：替换开启时直接吞掉，不回标题、不弹窗
    if crate::core::Hachimi::instance().config.load().replace_global_char.enable {
        return;
    }
    unsafe {
        let orig = ORIG;
        if orig != 0 {
            let f: GotoTitleOnErrorFn = std::mem::transmute(orig);
            f(text);
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(k) = get_class(umamusume, c"Gallop", c"GallopUtil") {
        let addr = get_method_addr(k, c"GotoTitleOnError", 1);
        if addr == 0 { return; }
        unsafe {
            if let Ok(t) = minhook::MinHook::create_hook(addr as *mut std::ffi::c_void, GotoTitleOnError_hook as *const () as *mut std::ffi::c_void) {
                ORIG = t as usize;
                let _ = minhook::MinHook::enable_hook(addr as *mut std::ffi::c_void);
                info!("[errskip] hooked {:#x}", addr);
            }
        }
    }
}
