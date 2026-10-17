use crate::{
    core::Hachimi,
    il2cpp::{symbols::{get_class, get_method_addr}, types::*}
};

// ---- Cut-in FOV (runtime scale from main-menu slider) ----
pub static CUTIN_FOV_RT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x3F800000);
pub fn fov_rt() -> f32 { f32::from_bits(CUTIN_FOV_RT.load(std::sync::atomic::Ordering::Relaxed)) }
pub fn set_fov_rt(v: f32) { CUTIN_FOV_RT.store(v.to_bits(), std::sync::atomic::Ordering::Relaxed); }

// ---- Cut-in progress bar data source ----
pub static CUTIN_CTRL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub static CUTIN_TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static CUTIN_TOTAL_BITS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub static CUTIN_TOTAL_OWNER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn cutin_active() -> bool {
    let t = CUTIN_TICK.load(std::sync::atomic::Ordering::Relaxed);
    t > 0 && now_ms().saturating_sub(t) < 600
}

pub fn cutin_current_time() -> f32 {
    let p = CUTIN_CTRL.load(std::sync::atomic::Ordering::Relaxed);
    if p == 0 { return 0.0; }
    unsafe { *((p as *const u8).add(0x538) as *const f32) }
}

pub fn cutin_total_time() -> f32 {
    f32::from_bits(CUTIN_TOTAL_BITS.load(std::sync::atomic::Ordering::Relaxed))
}

pub fn cutin_seek(target: f32) {
    unsafe {
        let p = CUTIN_CTRL.load(std::sync::atomic::Ordering::Relaxed);
        if p == 0 { return; }
        let this = p as *mut Il2CppObject;
        let k = *(this as *const *mut Il2CppClass);
        if k.is_null() { return; }
        let mp = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"Pause".as_ptr(), 1);
        if !mp.is_null() {
            let mut v = true;
            let args: [*mut std::ffi::c_void; 1] = [&mut v as *mut bool as *mut std::ffi::c_void];
            crate::il2cpp::api::il2cpp_runtime_invoke(mp, this as *mut std::ffi::c_void, args.as_ptr() as *mut *mut std::ffi::c_void, std::ptr::null_mut());
        }
        let ms = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"SetCurrentTime".as_ptr(), 1);
        if !ms.is_null() {
            let mr = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"ResetCurrentTime".as_ptr(), 0);
            if !mr.is_null() {
                crate::il2cpp::api::il2cpp_runtime_invoke(mr, this as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
            }
            let mut t = target;
            let args: [*mut std::ffi::c_void; 1] = [&mut t as *mut f32 as *mut std::ffi::c_void];
            crate::il2cpp::api::il2cpp_runtime_invoke(ms, this as *mut std::ffi::c_void, args.as_ptr() as *mut *mut std::ffi::c_void, std::ptr::null_mut());
        }
        let me = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"EffectSimulateToCurrentFrame".as_ptr(), 0);
        if !me.is_null() {
            crate::il2cpp::api::il2cpp_runtime_invoke(me, this as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
        }
    }
}

pub fn cutin_resume() {
    unsafe {
        let p = CUTIN_CTRL.load(std::sync::atomic::Ordering::Relaxed);
        if p == 0 { return; }
        let this = p as *mut Il2CppObject;
        let k = *(this as *const *mut Il2CppClass);
        if k.is_null() { return; }
        let mp = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"Pause".as_ptr(), 1);
        if !mp.is_null() {
            let mut v = false;
            let args: [*mut std::ffi::c_void; 1] = [&mut v as *mut bool as *mut std::ffi::c_void];
            crate::il2cpp::api::il2cpp_runtime_invoke(mp, this as *mut std::ffi::c_void, args.as_ptr() as *mut *mut std::ffi::c_void, std::ptr::null_mut());
        }
    }
}

unsafe fn read_object_field(obj: *mut Il2CppObject, name: &str) -> *mut Il2CppObject {
    if obj.is_null() { return std::ptr::null_mut(); }
    let klass = *(obj as *const *mut Il2CppClass);
    if klass.is_null() { return std::ptr::null_mut(); }
    let Ok(cn) = std::ffi::CString::new(name) else { return std::ptr::null_mut(); };
    let field = crate::il2cpp::api::il2cpp_class_get_field_from_name(klass, cn.as_ptr());
    if field.is_null() { return std::ptr::null_mut(); }
    let off = crate::il2cpp::api::il2cpp_field_get_offset(field);
    *((obj as *const u8).add(off) as *const *mut Il2CppObject)
}

unsafe fn read_il2cpp_string(s: *mut Il2CppString) -> String {
    if s.is_null() { return String::new(); }
    let p = s as usize;
    let len = *((p + 16) as *const i32);
    if len <= 0 || len > 400 { return String::new(); }
    let chars = std::slice::from_raw_parts((p + 20) as *const u16, len as usize);
    String::from_utf16_lossy(chars)
}

unsafe fn set_go_active(go: *mut Il2CppObject, active: bool) {
    if go.is_null() { return; }
    let klass = *(go as *const *mut Il2CppClass);
    if klass.is_null() { return; }
    let m = crate::il2cpp::api::il2cpp_class_get_method_from_name(klass, c"SetActive".as_ptr(), 1);
    if m.is_null() { return; }
    let mut v = active;
    let args: [*mut std::ffi::c_void; 1] = [&mut v as *mut bool as *mut std::ffi::c_void];
    crate::il2cpp::api::il2cpp_runtime_invoke(m, go as *mut std::ffi::c_void, args.as_ptr() as *mut *mut std::ffi::c_void, std::ptr::null_mut());
}

static TAG_OBJECTS: once_cell::sync::Lazy<std::sync::Mutex<Vec<usize>>> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(Vec::new()));

fn suppress_tag_objects() {
    let Ok(mut v) = TAG_OBJECTS.lock() else { return; };
    if v.is_empty() { return; }
    v.retain(|&p| {
        unsafe {
            let alive = crate::il2cpp::hook::UnityEngine_CoreModule::Object::IsNativeObjectAlive(p as *mut Il2CppObject);
            if alive {
                set_go_active(p as *mut Il2CppObject, false);
            }
            alive
        }
    });
}

fn is_tag_prefab(s: &str) -> bool {
    s.contains("gacha_icon_new") || s.contains("gacha_chara_name") || s.contains("gacha_chrname")
}

unsafe fn hide_cloned_tag(prefab: *mut Il2CppObject, cloned: *mut Il2CppObject) {
    if prefab.is_null() || cloned.is_null() { return; }
    let k = *(prefab as *const *mut Il2CppClass);
    if k.is_null() { return; }
    let mn = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"get_name".as_ptr(), 0);
    if mn.is_null() { return; }
    let nr = crate::il2cpp::api::il2cpp_runtime_invoke(mn, prefab as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
    if nr.is_null() { return; }
    let s = read_il2cpp_string(nr as *mut Il2CppString);
    if !is_tag_prefab(&s) { return; }
    set_go_active(cloned, false);
    if let Ok(mut v) = TAG_OBJECTS.lock() {
        if v.len() < 64 { v.push(cloned as usize); }
    }
}

type CloneSingleFn = extern "C" fn(prefab: *mut Il2CppObject, parent: *mut Il2CppObject, world: bool) -> *mut Il2CppObject;
extern "C" fn CloneSingle_hook(prefab: *mut Il2CppObject, parent: *mut Il2CppObject, world: bool) -> *mut Il2CppObject {
    let r = get_orig_fn!(CloneSingle_hook, CloneSingleFn)(prefab, parent, world);
    unsafe { hide_cloned_tag(prefab, r); }
    r
}

type CloneSingle1Fn = extern "C" fn(prefab: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn CloneSingle1_hook(prefab: *mut Il2CppObject) -> *mut Il2CppObject {
    let r = get_orig_fn!(CloneSingle1_hook, CloneSingle1Fn)(prefab);
    unsafe { hide_cloned_tag(prefab, r); }
    r
}

const BLOCKED_IDS: [i32; 8] = [61, 83, 126, 145, 173, 264, 281, 293];
static SHADOW_TRACK: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashMap<i32, (u64, u32)>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
static AUTO_BLOCK: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashSet<i32>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

fn shadow_file_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("hachimi").join("cutin_shadow_frames.txt"))
}

pub fn load_shadow_frames() {
    if let Some(p) = shadow_file_path() {
        if let Ok(txt) = std::fs::read_to_string(&p) {
            let Ok(mut set) = AUTO_BLOCK.lock() else { return; };
            for line in txt.lines() {
                if let Ok(v) = line.trim().parse::<i32>() { set.insert(v); }
            }
            info!("[cutin] loaded {} shadow frames", set.len());
        }
    }
}

fn append_shadow_frame(id: i32) {
    if let Some(p) = shadow_file_path() {
        if let Some(dir) = p.parent() { let _ = std::fs::create_dir_all(dir); }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
            use std::io::Write;
            let _ = writeln!(f, "{}", id);
        }
    }
}

fn observe_shadow_frame(id: i32) {
    if id <= 0 { return; }
    let now = now_ms();
    let mut detect = false;
    if let Ok(mut m) = SHADOW_TRACK.lock() {
        let e = m.entry(id).or_insert((now, 0));
        if now.saturating_sub(e.0) <= 10000 {
            e.1 += 1;
            if e.1 == 10 { detect = true; e.1 = 11; }
        } else {
            *e = (now, 1);
        }
    }
    if detect {
        let fresh = { AUTO_BLOCK.lock().map(|mut s| s.insert(id)).unwrap_or(false) };
        if fresh {
            info!("[cutin] auto-block shadow frame {}", id);
            append_shadow_frame(id);
        }
    }
}

fn is_blocked_frame(id: i32) -> bool {
    if BLOCKED_IDS.contains(&id) { return true; }
    AUTO_BLOCK.lock().map(|s| s.contains(&id)).unwrap_or(false)
}

type FireEvent2Fn = extern "C" fn(this: *mut Il2CppObject, key: *mut Il2CppObject, b: bool) -> ();
extern "C" fn FireEvent2(this: *mut Il2CppObject, key: *mut Il2CppObject, b: bool) {
    unsafe {
        if !key.is_null() {
            let id = *((key as *const u8).add(0x10) as *const i32);
            observe_shadow_frame(id);
            if Hachimi::instance().config.load().cutin_block_ui_events && is_blocked_frame(id) {
                return;
            }
        }
    }
    get_orig_fn!(FireEvent2, FireEvent2Fn)(this, key, b)
}

type AlterLateUpdateFn = extern "C" fn(this: *mut Il2CppObject, pos: *mut std::ffi::c_void, rot: *mut std::ffi::c_void, sc: f32) -> ();
extern "C" fn CutInAlterLateUpdate(this: *mut Il2CppObject, pos: *mut std::ffi::c_void, rot: *mut std::ffi::c_void, sc: f32) {
    get_orig_fn!(CutInAlterLateUpdate, AlterLateUpdateFn)(this, pos, rot, sc);
    unsafe {
        suppress_tag_objects();
        let scale_rt = fov_rt();
        let scale = if (scale_rt - 1.0).abs() > 0.001 { scale_rt } else { Hachimi::instance().config.load().cutin_fov_scale };
        if (scale - 1.0).abs() < 0.001 { return; }
        let cam = read_object_field(this, "_targetCamera");
        if !cam.is_null() {
            let klass = *(cam as *const *mut Il2CppClass);
            let get_m = crate::il2cpp::api::il2cpp_class_get_method_from_name(klass, c"get_fieldOfView".as_ptr(), 0);
            let set_m = crate::il2cpp::api::il2cpp_class_get_method_from_name(klass, c"set_fieldOfView".as_ptr(), 1);
            if !get_m.is_null() && !set_m.is_null() {
                let r = crate::il2cpp::api::il2cpp_runtime_invoke(get_m, cam as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
                if !r.is_null() {
                    let fov = *((r as *const u8).add(16) as *const f32);
                    let mut new_fov = fov * scale;
                    if new_fov > 160.0 { new_fov = 160.0; }
                    let args: [*mut std::ffi::c_void; 1] = [&mut new_fov as *mut f32 as *mut std::ffi::c_void];
                    crate::il2cpp::api::il2cpp_runtime_invoke(set_m, cam as *mut std::ffi::c_void, args.as_ptr() as *mut *mut std::ffi::c_void, std::ptr::null_mut());
                }
            }
        }
    }
}

unsafe fn ctl_touch(this: *mut Il2CppObject) {
    let p = this as usize;
    CUTIN_CTRL.store(p, std::sync::atomic::Ordering::Relaxed);
    CUTIN_TICK.store(now_ms(), std::sync::atomic::Ordering::Relaxed);
    if CUTIN_TOTAL_OWNER.load(std::sync::atomic::Ordering::Relaxed) != p {
        let k = *(this as *const *mut Il2CppClass);
        if !k.is_null() {
            let m = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"GetTotalTime".as_ptr(), 0);
            if !m.is_null() {
                CUTIN_TOTAL_OWNER.store(p, std::sync::atomic::Ordering::Relaxed);
                let r = crate::il2cpp::api::il2cpp_runtime_invoke(m, this as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
                if !r.is_null() {
                    let total = *((r as *const u8).add(16) as *const f32);
                    CUTIN_TOTAL_BITS.store(total.to_bits(), std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
}

type CTLAlterLateFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn CTL_AlterLateUpdate(this: *mut Il2CppObject) -> bool {
    unsafe { ctl_touch(this); }
    get_orig_fn!(CTL_AlterLateUpdate, CTLAlterLateFn)(this)
}

type CTLAlterUpdateFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn CTL_AlterUpdate(this: *mut Il2CppObject) -> bool {
    unsafe { ctl_touch(this); }
    get_orig_fn!(CTL_AlterUpdate, CTLAlterUpdateFn)(this)
}

type CTLAlterFixedFn = extern "C" fn(this: *mut Il2CppObject, a: f32, b: f32) -> ();
extern "C" fn CTL_AlterFixedUpdate(this: *mut Il2CppObject, a: f32, b: f32) {
    unsafe { ctl_touch(this); }
    get_orig_fn!(CTL_AlterFixedUpdate, CTLAlterFixedFn)(this, a, b)
}

fn find_class_any(umamusume: *const Il2CppImage, name: &str) -> *mut Il2CppClass {
    unsafe {
        let Ok(cn) = std::ffi::CString::new(name) else { return std::ptr::null_mut(); };
        if let Ok(k) = get_class(umamusume, c"Gallop", cn.as_c_str()) { return k; }
        if let Ok(k) = get_class(umamusume, c"", cn.as_c_str()) { return k; }
        let count = crate::il2cpp::api::il2cpp_image_get_class_count(umamusume);
        for i in 0..count {
            let kk = crate::il2cpp::api::il2cpp_image_get_class(umamusume, i) as *mut Il2CppClass;
            if kk.is_null() { continue; }
            let n = std::ffi::CStr::from_ptr((*kk).name).to_string_lossy();
            if n == name { return kk; }
        }
        std::ptr::null_mut()
    }
}

fn cls_opt(umamusume: *const Il2CppImage, name: &str) -> Option<*mut Il2CppClass> {
    let k = find_class_any(umamusume, name);
    if k.is_null() { None } else { Some(k) }
}

// GallopUtil.GotoTitleOnError(string)
type GotoTitleOnErrorFn = extern "C" fn(text: *mut Il2CppString) -> ();
static GOTO_ERR_LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
extern "C" fn GotoTitleOnError_hook(text: *mut Il2CppString) {
    if Hachimi::instance().config.load().intercept_error_dialog {
        if !GOTO_ERR_LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            let msg = unsafe { read_il2cpp_string(text) };
            warn!("[cutin] intercepted GotoTitleOnError: {}", msg);
        }
        return;
    }
    get_orig_fn!(GotoTitleOnError_hook, GotoTitleOnErrorFn)(text)
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(k) = get_class(umamusume, c"Gallop.CutIn.Cutt", c"CutInTimelineMotionCamera") {
        let a = get_method_addr(k, c"AlterLateUpdate", 3);
        if a != 0 { new_hook!(a, CutInAlterLateUpdate); }
    }
    if let Some(ctl) = cls_opt(umamusume, "CutInTimelineController") {
        let a = get_method_addr(ctl, c"AlterLateUpdate", 0);
        if a != 0 { new_hook!(a, CTL_AlterLateUpdate); }
        let a = get_method_addr(ctl, c"AlterUpdate", 0);
        if a != 0 { new_hook!(a, CTL_AlterUpdate); }
        let a = get_method_addr(ctl, c"AlterFixedUpdate", 2);
        if a != 0 { new_hook!(a, CTL_AlterFixedUpdate); }
    }
    if let Some(pubr) = cls_opt(umamusume, "CutInTimelineEventPublisher") {
        let a = get_method_addr(pubr, c"FireEvent", 2);
        if a != 0 { new_hook!(a, FireEvent2); }
    }
    if let Ok(ui_img) = crate::il2cpp::symbols::get_assembly_image(c"UnityEngine.CoreModule") {
        if let Ok(obj_cls) = crate::il2cpp::symbols::get_class(ui_img, c"UnityEngine", c"Object") {
            let a = get_method_addr(obj_cls, c"Internal_CloneSingleWithParent", 3);
            if a != 0 { new_hook!(a, CloneSingle_hook); }
            let a = get_method_addr(obj_cls, c"Internal_CloneSingle", 1);
            if a != 0 { new_hook!(a, CloneSingle1_hook); }
        }
    }
    if let Ok(k) = get_class(umamusume, c"Gallop", c"GallopUtil") {
        let a = get_method_addr(k, c"GotoTitleOnError", 1);
        if a != 0 { new_hook!(a, GotoTitleOnError_hook); }
    }
    load_shadow_frames();
}
