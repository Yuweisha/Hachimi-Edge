use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::hachimi::Hachimi;
use crate::il2cpp::ext::StringExt;
use crate::il2cpp::{
    symbols::{get_method_addr, get_assembly_image, get_class, invoke_object_method},
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GET_LOADER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_Loader, GET_LOADER_ADDR, *mut Il2CppObject,);

pub fn LoadAssetHandle(this: *mut Il2CppObject, path: *mut Il2CppString, flag: bool) -> *mut Il2CppObject {
    let mut params: [*mut std::ffi::c_void; 2] = [path as *mut _, &flag as *const _ as *mut _];
    invoke_object_method(this, c"LoadAssetHandle", 2, &mut params).unwrap_or(std::ptr::null_mut())
}

static mut GET_ASSETBUNDLE_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_assetBundle, GET_ASSETBUNDLE_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);


unsafe fn il2cpp_str(s: *mut Il2CppString) -> String {
    if s.is_null() {
        return String::new();
    }
    let p = s as usize;
    let len = *((p + 16) as *const i32);
    if len <= 0 || len > 400 {
        return String::new();
    }
    let chars = std::slice::from_raw_parts((p + 20) as *const u16, len as usize);
    String::from_utf16_lossy(chars)
}

static LOADER_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

fn remap_card_path(s: &str) -> Option<String> {
    if !s.contains("crd") {
        return None;
    }

    let hachimi = Hachimi::instance();
    let config = hachimi.config.load();
    let char_replace = &config.replace_global_char;
    if !char_replace.enable || char_replace.data.is_empty() {
        return None;
    }

    let mut out = s.to_owned();
    let mut changed = false;
    for entry in char_replace.data.iter() {
        let key = format!("crd{:04}", entry.new_char_id);
        let mut search_start = 0;
        loop {
            let Some(idx) = out[search_start..].find(key.as_str()) else {
                break;
            };
            let pos = search_start + idx;
            let after_idx = pos + key.len();
            if after_idx + 2 <= out.len() {
                let xx = &out[after_idx..after_idx + 2];
                if xx != "01" && xx.bytes().all(|b| b.is_ascii_digit()) {
                    out.replace_range(after_idx..after_idx + 2, "01");
                    changed = true;
                }
            }
            search_start = after_idx + 2;
        }
    }

    if changed {
        Some(out)
    } else {
        None
    }
}

fn install_load_asset_handle_hook(loader: *mut Il2CppObject) {
    if LOADER_HOOK_INSTALLED.load(Ordering::Acquire) || loader.is_null() {
        return;
    }
    unsafe {
        let loader_class = *((loader as usize) as *const usize) as *mut Il2CppClass;
        if loader_class.is_null() {
            return;
        }
        let class_name = CStr::from_ptr((*loader_class).name).to_str().unwrap_or("?");
        let addr = get_method_addr(loader_class, c"LoadAssetHandle", 2);
        if addr == 0 {
            warn!("[charres] LoadAssetHandle 不在 {} 上，稍后重试", class_name);
            return;
        }
        LOADER_HOOK_INSTALLED.store(true, Ordering::Release);
        warn!("[charres] LoadAssetHandle 已挂钩于 {}", class_name);
        new_hook!(addr, LoadAssetHandle_hook);
    }
}

type GetLoaderFn = extern "C" fn() -> *mut Il2CppObject;
extern "C" fn get_Loader_hook() -> *mut Il2CppObject {
    let loader = get_orig_fn!(get_Loader_hook, GetLoaderFn)();
    install_load_asset_handle_hook(loader);
    loader
}

extern "system" {
    fn RtlCaptureStackBackTrace(frames_to_skip: u32, frames_to_capture: u32, back_trace: *mut *mut std::ffi::c_void, back_trace_hash: *mut u32) -> u16;
}

unsafe fn scan_owner_for(target: usize) {
    let Ok(image) = get_assembly_image(c"umamusume.dll") else { return; };
    let count = crate::il2cpp::api::il2cpp_image_get_class_count(image);
    for i in 0..count {
        let klass = crate::il2cpp::api::il2cpp_image_get_class(image, i) as *mut Il2CppClass;
        if klass.is_null() { continue; }
        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut min_a = usize::MAX;
        let mut max_a = 0usize;
        loop {
            let method = crate::il2cpp::api::il2cpp_class_get_methods(klass, &mut iter);
            if method.is_null() { break; }
            let ptr = (*method).methodPointer as usize;
            if ptr != 0 {
                if ptr < min_a { min_a = ptr; }
                if ptr > max_a { max_a = ptr; }
            }
        }
        if min_a != usize::MAX && min_a <= target && target <= max_a + 0x3000 {
            let name = CStr::from_ptr((*klass).name).to_string_lossy().to_string();
            let inside = min_a <= target && target <= max_a;
            info!("[mvres] frame owner: {} [{:#x} ~ {:#x}] {}", name, min_a, max_a, if inside { "INSIDE" } else { "near" });
        }
    }
}

unsafe fn log_cutin_stack() {
    let mut buf: [*mut std::ffi::c_void; 40] = [std::ptr::null_mut(); 40];
    let cnt = RtlCaptureStackBackTrace(1, 40, buf.as_mut_ptr(), std::ptr::null_mut());
    let addrs: Vec<String> = (0..cnt as usize).map(|i| format!("{:#x}", buf[i] as usize)).collect();
    info!("[mvres] cutin stack: {}", addrs.join(" "));
}

type LoadAssetHandleFn = extern "C" fn(this: *mut Il2CppObject, path: *mut Il2CppString, flag: bool) -> *mut Il2CppObject;
extern "C" fn LoadAssetHandle_hook(this: *mut Il2CppObject, path: *mut Il2CppString, flag: bool) -> *mut Il2CppObject {
    let s = unsafe { il2cpp_str(path) };
    if !s.is_empty() {
        let sl = s.to_lowercase();
        if sl.contains("live/") || sl.contains("movie") || sl.contains("_act") || sl.contains("act_") || sl.contains("cutin") || sl.contains("gacha") || sl.contains("banner") || sl.contains("_new") || sl.contains("new_") || sl.contains("star") || sl.contains("title") {
            info!("[mvres] {}", s);
        }
    }
    if s.starts_with("cutt/cutin/") {
        unsafe { log_cutin_stack(); }
    }
    if s.contains("pf_fl_gacha_icon_new00") || s.contains("fa_gacha_chara_name") || s.contains("gacha_chrname") || s.contains("gacha_comment") {
        unsafe { log_cutin_stack(); }
    }
    let remapped = remap_card_path(&s);
    let r = if let Some(ref fixed) = remapped {
        warn!("[crdremap] {} -> {}", s, fixed);
        let new_path = fixed.to_il2cpp_string();
        get_orig_fn!(LoadAssetHandle_hook, LoadAssetHandleFn)(this, new_path, flag)
    } else {
        get_orig_fn!(LoadAssetHandle_hook, LoadAssetHandleFn)(this, path, flag)
    };
    if r.is_null() && !s.is_empty() {
        warn!("[charres] 失败: {}", s);
    }
    r
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, AssetManager);

    unsafe {
        CLASS = AssetManager;
        GET_LOADER_ADDR = get_method_addr(AssetManager, c"get_Loader", 0);
        new_hook!(GET_LOADER_ADDR, get_Loader_hook);
    }

    if let Ok(cyan_image) = get_assembly_image(c"_Cyan.dll") {
        if let Ok(asset_handle_class) = get_class(cyan_image, c"Cyan.Loader", c"AssetHandle") {
            unsafe {
                GET_ASSETBUNDLE_ADDR = get_method_addr(asset_handle_class, c"get_assetBundle", 0);
            }
        } else {
            error!("Failed to find AssetHandle class in _Cyan.dll");
        }
    } else {
        error!("Failed to load _Cyan.dll assembly");
    }
}
