use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppObjectExt, Il2CppStringExt, StringExt},
        hook::umamusume::{CharacterBuildInfo, CharacterBuildInfo::UmaControllerType},
        symbols::{get_method_addr, get_assembly_image, get_class, invoke_object_method},
        types::*
    }
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

static LOADER_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

fn install_load_asset_handle_hook(loader: *mut Il2CppObject) {
    if LOADER_HOOK_INSTALLED.load(Ordering::Acquire) || loader.is_null() {
        return;
    }

    unsafe {
        let loader_class = (*loader).klass();
        if loader_class.is_null() {
            return;
        }
        let class_name = std::ffi::CStr::from_ptr((*loader_class).name)
            .to_str()
            .unwrap_or("?");

        let addr = get_method_addr(loader_class, c"LoadAssetHandle", 2);
        if addr == 0 {
            warn!("[charres] LoadAssetHandle 不在 {} 上，稍后重试", class_name);
            return;
        }

        LOADER_HOOK_INSTALLED.store(true, Ordering::Release);
        debug!("[charres] LoadAssetHandle 已挂钩于 {}", class_name);
        new_hook!(addr, LoadAssetHandle_hook);
    }
}

type GetLoaderFn = extern "C" fn() -> *mut Il2CppObject;
extern "C" fn get_Loader_hook() -> *mut Il2CppObject {
    let loader = get_orig_fn!(get_Loader_hook, GetLoaderFn)();
    install_load_asset_handle_hook(loader);
    loader
}

type LoadAssetHandleFn = extern "C" fn(this: *mut Il2CppObject, path: *mut Il2CppString, flag: bool) -> *mut Il2CppObject;

/// 过场里的角色模型重映射。
///
/// 过场（EventTimeline / CutIn）**不能改角色 ID** —— Timeline 的轨道绑定和演出数据都是按
/// 角色预制好的，ID 对不上会让游戏卡死或模型整个消失。所以这里只把加载路径里的角色 ID
/// 换成目标角色的：模型、服装、贴图整套跟着换，游戏看到的数据层仍然是原角色。
fn remap_cutscene_char_asset(path: &str) -> Option<String> {
    let ctrl = CharacterBuildInfo::CURRENT_CONTROLLER_TYPE.load(Ordering::Acquire);
    if ctrl != UmaControllerType::EventTimeline as i32 && ctrl != UmaControllerType::CutIn as i32 {
        return None;
    }

    let config = Hachimi::instance().config.load();
    let repl = &config.replace_global_char;
    if !repl.enable || !repl.replace_model_in_cutscene {
        return None;
    }

    for entry in repl.data.iter() {
        let (old, new) = (entry.orig_char_id, entry.new_char_id);
        if old <= 0 || new <= 0 || new == old {
            continue;
        }

        // bdy1002_ / chr1002_ 这两种前缀覆盖了模型、头发、贴图（mchr1002_ 会被 chr 的子串带上）
        let (old_body, new_body) = (format!("bdy{old}_"), format!("bdy{new}_"));
        let (old_head, new_head) = (format!("chr{old}_"), format!("chr{new}_"));

        if path.contains(&old_body) || path.contains(&old_head) {
            let remapped = path.replace(&old_body, &new_body).replace(&old_head, &new_head);
            warn!("[charres] 过场外观替换 {} -> {}", path, remapped);
            return Some(remapped);
        }
    }

    None
}

extern "C" fn LoadAssetHandle_hook(this: *mut Il2CppObject, path: *mut Il2CppString, flag: bool) -> *mut Il2CppObject {
    if !path.is_null() {
        let p = unsafe { (*path).as_utf16str() }.to_string();
        let low = p.to_lowercase();
        if low.contains("3d/chara") || low.contains("bdy") || low.contains("chr") {
            let ctrl = CharacterBuildInfo::CURRENT_CONTROLLER_TYPE.load(Ordering::Acquire);
            warn!("[charres] ctrl={:<9} {}", ctrl, p);
        }

        if let Some(remapped) = remap_cutscene_char_asset(&p) {
            return get_orig_fn!(LoadAssetHandle_hook, LoadAssetHandleFn)(
                this, remapped.to_il2cpp_string(), flag
            );
        }
    }

    get_orig_fn!(LoadAssetHandle_hook, LoadAssetHandleFn)(this, path, flag)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, AssetManager);

    unsafe {
        CLASS = AssetManager;
        GET_LOADER_ADDR = get_method_addr(AssetManager, c"get_Loader", 0);
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

    unsafe {
        new_hook!(GET_LOADER_ADDR, get_Loader_hook);
    }
}
