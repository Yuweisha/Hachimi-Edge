// 临时探针：确认「演出」卡片动画的播放器与方向请求链（为横屏化做准备）。
// 定位完成后删除本文件及 mod.rs 中的注册。

use crate::{
    core::Hachimi,
    il2cpp::{ext::Il2CppStringExt, symbols::{get_class, get_method_addr, IEnumerator, MoveNextFn}, types::*}
};

// StoryFullMoviePlayer.Init(DisplayMode, bool)
type SFMPInitFn = extern "C" fn(this: *mut Il2CppObject, mode: i32, b: bool);
extern "C" fn SFMP_Init(this: *mut Il2CppObject, mode: i32, b: bool) {
    info!("[mvprobe] StoryFullMoviePlayer.Init mode={} b={}", mode, b);
    get_orig_fn!(SFMP_Init, SFMPInitFn)(this, mode, b)
}

// StoryFullMoviePlayer.ChangeOrientationAsync(DisplayMode)
type SFMPChOrientFn = extern "C" fn(this: *mut Il2CppObject, mode: i32) -> *mut Il2CppObject;
extern "C" fn SFMP_ChangeOrientation(this: *mut Il2CppObject, mode: i32) -> *mut Il2CppObject {
    info!("[mvprobe] StoryFullMoviePlayer.ChangeOrientationAsync mode={}", mode);
    get_orig_fn!(SFMP_ChangeOrientation, SFMPChOrientFn)(this, mode)
}

// StoryFullMoviePlayer.RequestChangeOrientation()
type SFMPReqFn = extern "C" fn(this: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn SFMP_RequestChangeOrientation(this: *mut Il2CppObject) -> *mut Il2CppObject {
    info!("[mvprobe] StoryFullMoviePlayer.RequestChangeOrientation");
    get_orig_fn!(SFMP_RequestChangeOrientation, SFMPReqFn)(this)
}

// StoryFullMoviePlayer.AdjustMovieSize(DisplayMode)
type SFMPAdjFn = extern "C" fn(this: *mut Il2CppObject, mode: i32);
extern "C" fn SFMP_AdjustMovieSize(this: *mut Il2CppObject, mode: i32) {
    info!("[mvprobe] StoryFullMoviePlayer.AdjustMovieSize mode={}", mode);
    get_orig_fn!(SFMP_AdjustMovieSize, SFMPAdjFn)(this, mode)
}

// StoryMovieViewController.GetChangeViewOrientation(ScreenOrientationClassWrapper)
type SMVCGetOrientFn = extern "C" fn(this: *mut Il2CppObject, wrapper: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn SMVC_GetChangeViewOrientation(this: *mut Il2CppObject, wrapper: *mut Il2CppObject) -> *mut Il2CppObject {
    info!("[mvprobe] StoryMovieViewController.GetChangeViewOrientation");
    get_orig_fn!(SMVC_GetChangeViewOrientation, SMVCGetOrientFn)(this, wrapper)
}

// PartsGachaMovieListItem.OnClick()
type ItemOnClickFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Item_OnClick(this: *mut Il2CppObject) {
    get_orig_fn!(Item_OnClick, ItemOnClickFn)(this)
}

extern "system" {
    fn RtlCaptureStackBackTrace(frames_to_skip: u32, frames_to_capture: u32, back_trace: *mut *mut std::ffi::c_void, back_trace_hash: *mut u32) -> u16;
}

unsafe fn log_stack(tag: &str) {
    let mut buf: [*mut std::ffi::c_void; 40] = [std::ptr::null_mut(); 40];
    let cnt = RtlCaptureStackBackTrace(1, 40, buf.as_mut_ptr(), std::ptr::null_mut());
    let addrs: Vec<String> = (0..cnt as usize).map(|i| format!("{:#x}", buf[i] as usize)).collect();
    info!("[mvprobe] {} stack: {}", tag, addrs.join(" "));
}

fn dump_method_addrs(klass: *mut Il2CppClass, label: &str) {
    unsafe {
        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        loop {
            let method = crate::il2cpp::api::il2cpp_class_get_methods(klass, &mut iter);
            if method.is_null() { break; }
            let mname = std::ffi::CStr::from_ptr((*method).name).to_string_lossy().to_string();
            let count = crate::il2cpp::api::il2cpp_method_get_param_count(method);
            let ptr = (*method).methodPointer as usize;
            info!("[mvmap] {}::{} argc={} @ {:#x}", label, mname, count, ptr);
        }
    }
}

fn try_dump_method_addrs(umamusume: *const Il2CppImage, name: &str) {
    let Ok(c_name) = std::ffi::CString::new(name) else { return; };
    unsafe {
        match get_class(umamusume, c"Gallop", c_name.as_c_str()) {
            Ok(k) => dump_method_addrs(k, name),
            Err(e) => error!("[mvmap] class {}: {}", name, e),
        }
    }
}

// PartsOthersMovieListItem.OnClick()
type OthersOnClickFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Others_OnClick(this: *mut Il2CppObject) {
    get_orig_fn!(Others_OnClick, OthersOnClickFn)(this)
}

// GachaMovie.SetGachaIdAndLoad(int)
type SetGachaFn = extern "C" fn(this: *mut Il2CppObject, id: i32);
extern "C" fn GM_SetGachaIdAndLoad(this: *mut Il2CppObject, id: i32) {
    info!("[mvprobe] GachaMovie.SetGachaIdAndLoad id={}", id);
    get_orig_fn!(GM_SetGachaIdAndLoad, SetGachaFn)(this, id)
}

// FullScreenMoviePlayer.Init(bool)
type FMPInitFn = extern "C" fn(this: *mut Il2CppObject, b: bool);
extern "C" fn FMP_Init(this: *mut Il2CppObject, b: bool) {
    info!("[mvprobe] FullScreenMoviePlayer.Init b={}", b);
    get_orig_fn!(FMP_Init, FMPInitFn)(this, b)
}

// FullScreenMoviePlayer.LoadAndPlay(string, Action, Action)
type FMPLoadAndPlayFn = extern "C" fn(this: *mut Il2CppObject, path: *mut Il2CppString, a: *mut Il2CppObject, b: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn FMP_LoadAndPlay(this: *mut Il2CppObject, path: *mut Il2CppString, a: *mut Il2CppObject, b: *mut Il2CppObject) -> *mut Il2CppObject {
    unsafe {
        let p = if path.is_null() { "null".to_string() } else { (*path).as_utf16str().to_string() };
        info!("[mvprobe] FMP.LoadAndPlay path={:?}", p);
    }
    get_orig_fn!(FMP_LoadAndPlay, FMPLoadAndPlayFn)(this, path, a, b)
}

// FullScreenMoviePlayer.Play(Action, Action)
type FMPPlayFn = extern "C" fn(this: *mut Il2CppObject, a: *mut Il2CppObject, b: *mut Il2CppObject);
extern "C" fn FMP_Play(this: *mut Il2CppObject, a: *mut Il2CppObject, b: *mut Il2CppObject) {
    info!("[mvprobe] FMP.Play");
    get_orig_fn!(FMP_Play, FMPPlayFn)(this, a, b)
}

// FullScreenMoviePlayer.AdjustMovieSizeHD()
type FMPAdjFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn FMP_AdjustMovieSizeHD(this: *mut Il2CppObject) {
    info!("[mvprobe] FMP.AdjustMovieSizeHD");
    get_orig_fn!(FMP_AdjustMovieSizeHD, FMPAdjFn)(this)
}

// FullScreenMoviePlayerEtoE.CalcScreenSize() -> Vector2
type EtoECalcFn = extern "C" fn(this: *mut Il2CppObject) -> u64;
extern "C" fn EtoE_CalcScreenSize(this: *mut Il2CppObject) -> u64 {
    let ret = get_orig_fn!(EtoE_CalcScreenSize, EtoECalcFn)(this);
    info!("[mvprobe] EtoE.CalcScreenSize ret_bits={:#x}", ret);
    ret
}

fn dump_all_method_addrs(umamusume: *const Il2CppImage) {
    unsafe {
        let count = crate::il2cpp::api::il2cpp_image_get_class_count(umamusume);
        for i in 0..count {
            let klass = crate::il2cpp::api::il2cpp_image_get_class(umamusume, i) as *mut Il2CppClass;
            if klass.is_null() { continue; }
            let cname = std::ffi::CStr::from_ptr((*klass).name).to_string_lossy().to_string();
            let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
            loop {
                let method = crate::il2cpp::api::il2cpp_class_get_methods(klass, &mut iter);
                if method.is_null() { break; }
                let ptr = (*method).methodPointer as usize;
                if ptr != 0 {
                    let mname = std::ffi::CStr::from_ptr((*method).name).to_string_lossy().to_string();
                    info!("[mvall] {}::{} @ {:#x}", cname, mname, ptr);
                }
            }
        }
    }
}

// SceneManager.ChangeView(7 args)
type SMChangeViewFn = extern "C" fn(this: *mut Il2CppObject, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize, g: usize) -> *mut Il2CppObject;
static mut SM_CLASS: *mut Il2CppClass = std::ptr::null_mut();
extern "C" fn SM_ChangeView(this: *mut Il2CppObject, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize, g: usize) -> *mut Il2CppObject {
    info!("[mvprobe] ChangeView enter: arg0={:#x}", a);
    let ret = get_orig_fn!(SM_ChangeView, SMChangeViewFn)(this, a, b, c, d, e, f, g);
    unsafe {
        let cls = SM_CLASS;
        if !cls.is_null() {
            let get_id = crate::il2cpp::api::il2cpp_class_get_method_from_name(cls, c"GetCurrentViewId".as_ptr(), 0);
            let get_ls = crate::il2cpp::api::il2cpp_class_get_method_from_name(cls, c"get_IsLandscapeMode".as_ptr(), 0);
            let mut id = -1i32;
            if !get_id.is_null() {
                let r = crate::il2cpp::api::il2cpp_runtime_invoke(get_id, this as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
                if !r.is_null() { id = *((r as *const u8).add(16) as *const i32); }
            }
            let mut ls = false;
            if !get_ls.is_null() {
                let r = crate::il2cpp::api::il2cpp_runtime_invoke(get_ls, this as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
                if !r.is_null() { ls = *((r as *const u8).add(16)) != 0; }
            }
            info!("[mvprobe] ChangeView done: curViewId={} ({:#x}) isLandscape={}", id, id, ls);
        }
    }
    ret
}

// SceneManager.get_IsLandscapeMode()
type SMIsLandFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn SM_IsLandscape(this: *mut Il2CppObject) -> bool {
    let r = get_orig_fn!(SM_IsLandscape, SMIsLandFn)(this);
    info!("[mvprobe] get_IsLandscapeMode -> {}", r);
    unsafe { log_stack("IsLandscape"); }
    r
}

fn scan_by_keywords(umamusume: *const Il2CppImage, keywords: &[&str]) {
    unsafe {
        let count = crate::il2cpp::api::il2cpp_image_get_class_count(umamusume);
        for i in 0..count {
            let klass = crate::il2cpp::api::il2cpp_image_get_class(umamusume, i) as *mut Il2CppClass;
            if klass.is_null() { continue; }
            let name = std::ffi::CStr::from_ptr((*klass).name).to_string_lossy().to_string();
            if keywords.iter().any(|k| name.contains(k)) {
                let ns = std::ffi::CStr::from_ptr((*klass).namespaze).to_string_lossy().to_string();
                let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
                let mut n = 0;
                loop {
                    let method = crate::il2cpp::api::il2cpp_class_get_methods(klass, &mut iter);
                    if method.is_null() { break; }
                    n += 1;
                }
                info!("[mvcut] class {}.{} methods={}", ns, name, n);
            }
        }
    }
}

fn dump_class_sigs(umamusume: *const Il2CppImage, name: &str) {
    let Ok(c_name) = std::ffi::CString::new(name) else { return; };
    unsafe {
        let k = match get_class(umamusume, c"Gallop", c_name.as_c_str()) {
            Ok(k) => k,
            Err(_) => match get_class(umamusume, c"", c_name.as_c_str()) {
                Ok(k) => k,
                Err(_) => {
                    let mut found: *mut Il2CppClass = std::ptr::null_mut();
                    let count = crate::il2cpp::api::il2cpp_image_get_class_count(umamusume);
                    for i in 0..count {
                        let kk = crate::il2cpp::api::il2cpp_image_get_class(umamusume, i) as *mut Il2CppClass;
                        if kk.is_null() { continue; }
                        let n = std::ffi::CStr::from_ptr((*kk).name).to_string_lossy();
                        if n == name { found = kk; break; }
                    }
                    if found.is_null() { warn!("[mvcut] class {} not found", name); return; }
                    found
                }
            }
        };
        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut lines: Vec<String> = Vec::new();
        loop {
            let method = crate::il2cpp::api::il2cpp_class_get_methods(k, &mut iter);
            if method.is_null() { break; }
            if (*method).is_generic() != 0 { continue; }
            let mname = std::ffi::CStr::from_ptr((*method).name).to_string_lossy().to_string();
            let count = crate::il2cpp::api::il2cpp_method_get_param_count(method);
            let mut params = Vec::new();
            for i in 0..count {
                let ty = crate::il2cpp::api::il2cpp_method_get_param(method, i);
                if ty.is_null() { params.push("?".to_string()); continue; }
                let p = crate::il2cpp::api::il2cpp_type_get_name(ty);
                params.push(if p.is_null() { "?".to_string() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().to_string() });
            }
            let ret = {
                let ty = crate::il2cpp::api::il2cpp_method_get_return_type(method);
                if ty.is_null() { "?".to_string() } else {
                    let p = crate::il2cpp::api::il2cpp_type_get_name(ty);
                    if p.is_null() { "?".to_string() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().to_string() }
                }
            };
            lines.push(format!("{}({}) -> {}", mname, params.join(", "), ret));
        }
        lines.sort();
        info!("[mvcut] ==== {} ====", name);
        for l in &lines { info!("[mvcut]   {}", l); }
    }
}

fn dump_class_fields(umamusume: *const Il2CppImage, name: &str) {
    let Ok(c_name) = std::ffi::CString::new(name) else { return; };
    unsafe {
        let k = match get_class(umamusume, c"Gallop", c_name.as_c_str()) {
            Ok(k) => k,
            Err(_) => match get_class(umamusume, c"", c_name.as_c_str()) {
                Ok(k) => k,
                Err(_) => {
                    let mut found: *mut Il2CppClass = std::ptr::null_mut();
                    let count = crate::il2cpp::api::il2cpp_image_get_class_count(umamusume);
                    for i in 0..count {
                        let kk = crate::il2cpp::api::il2cpp_image_get_class(umamusume, i) as *mut Il2CppClass;
                        if kk.is_null() { continue; }
                        let n = std::ffi::CStr::from_ptr((*kk).name).to_string_lossy();
                        if n == name { found = kk; break; }
                    }
                    if found.is_null() { warn!("[mvcut] fields class {} not found", name); return; }
                    found
                }
            }
        };
        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut lines: Vec<String> = Vec::new();
        loop {
            let field = crate::il2cpp::api::il2cpp_class_get_fields(k, &mut iter);
            if field.is_null() { break; }
            let fname = std::ffi::CStr::from_ptr((*field).name).to_string_lossy().to_string();
            let ty = crate::il2cpp::api::il2cpp_field_get_type(field);
            let tname = if ty.is_null() { "?".to_string() } else {
                let p = crate::il2cpp::api::il2cpp_type_get_name(ty);
                if p.is_null() { "?".to_string() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().to_string() }
            };
            let off = crate::il2cpp::api::il2cpp_field_get_offset(field);
            lines.push(format!("{} {} (off=0x{:x})", tname, fname, off));
        }
        lines.sort();
        info!("[mvcut] ---- fields {} ({}) ----", name, lines.len());
        for l in &lines { info!("[mvcut]   f: {}", l); }
    }
}

fn dump_class_methods_named(umamusume: *const Il2CppImage, name: &str) {
    let Ok(c_name) = std::ffi::CString::new(name) else { return; };
    unsafe {
        match get_class(umamusume, c"", c_name.as_c_str()) {
            Ok(k) => dump_method_addrs(k, name),
            Err(_) => {
                match get_class(umamusume, c"Gallop", c_name.as_c_str()) {
                    Ok(k) => dump_method_addrs(k, name),
                    Err(e) => warn!("[mvcut] class {}: {}", name, e),
                }
            }
        }
    }
}

// CutInTimelineMotionCamera.SetTargetCamera(UnityEngine.Camera)
type SetTargetCameraFn = extern "C" fn(this: *mut Il2CppObject, cam: *mut Il2CppObject) -> ();
extern "C" fn CutInSetTargetCamera(this: *mut Il2CppObject, cam: *mut Il2CppObject) {
    unsafe {
        let df = read_f32_field(this, 0x108);
        let kf = read_f32_field(this, 0x10c);
        let use_mc = read_u8_field(this, 0x28);
        let cam_fov = if !cam.is_null() { read_f32_field(cam, 0x18) } else { 0.0 };
        info!("[mvfix] SetTargetCamera: cam={:p} defaultFov={} keepFov={} useMotionCam={} camFov@{:#x}={}", cam, df, kf, use_mc, 0x18usize, cam_fov);
    }
    get_orig_fn!(CutInSetTargetCamera, SetTargetCameraFn)(this, cam)
}

// CutInTimelineMotionCamera.AlterLateUpdate(Vector3, Quaternion, float)
type AlterLateUpdateFn = extern "C" fn(this: *mut Il2CppObject, pos: *mut std::ffi::c_void, rot: *mut std::ffi::c_void, sc: f32) -> ();
// FOV 运行时倍率（主菜单滑条直控，拖动即生效；不落盘，重启恢复配置值）
pub static CUTIN_FOV_RT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x3F800000); // 1.0f32
pub fn fov_rt() -> f32 { f32::from_bits(CUTIN_FOV_RT.load(std::sync::atomic::Ordering::Relaxed)) }
pub fn set_fov_rt(v: f32) { CUTIN_FOV_RT.store(v.to_bits(), std::sync::atomic::Ordering::Relaxed); }

extern "C" fn CutInAlterLateUpdate(this: *mut Il2CppObject, pos: *mut std::ffi::c_void, rot: *mut std::ffi::c_void, sc: f32) {
    get_orig_fn!(CutInAlterLateUpdate, AlterLateUpdateFn)(this, pos, rot, sc);
    unsafe {
        // 每帧压制被拦截的遮罩对象（游戏会反复激活）
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

unsafe fn read_object_field(obj: *mut Il2CppObject, name: &str) -> *mut Il2CppObject {
    if obj.is_null() { return std::ptr::null_mut(); }
    let klass = *(obj as *const *mut Il2CppClass);
    if klass.is_null() { return std::ptr::null_mut(); }
    let Ok(cn) = std::ffi::CString::new(name) else { return std::ptr::null_mut(); };
    let field = crate::il2cpp::api::il2cpp_class_get_field_from_name(klass, cn.as_ptr());
    if field.is_null() { return std::ptr::null_mut(); }
    let off = crate::il2cpp::api::il2cpp_field_get_offset(field);
    let addr = (obj as *const u8).add(off) as *const *mut Il2CppObject;
    *addr
}

unsafe fn read_f32_field(obj: *mut Il2CppObject, off: usize) -> f32 {
    if obj.is_null() { return 0.0; }
    *((obj as *const u8).add(off) as *const f32)
}

unsafe fn read_u8_field(obj: *mut Il2CppObject, off: usize) -> u8 {
    if obj.is_null() { return 0; }
    *((obj as *const u8).add(off))
}

// LowResolutionCamera hooks
type LRCInitVFn = extern "C" fn(this: *mut Il2CppObject) -> ();
extern "C" fn LRC_InitVertical(this: *mut Il2CppObject) {
    info!("[mvfix] LRC.InitializeVertical called");
    unsafe { log_stack_short("initV"); }
    get_orig_fn!(LRC_InitVertical, LRCInitVFn)(this)
}

type LRCInitHFn = extern "C" fn(this: *mut Il2CppObject, view: *mut Il2CppObject) -> ();
extern "C" fn LRC_InitHorizontal(this: *mut Il2CppObject, view: *mut Il2CppObject) {
    info!("[mvfix] LRC.InitializeHorizontal called view={:p}", view);
    unsafe { log_stack_short("initH"); }
    get_orig_fn!(LRC_InitHorizontal, LRCInitHFn)(this, view)
}

type LRCCreateTexFn = extern "C" fn(this: *mut Il2CppObject, res: *mut i32) -> ();
extern "C" fn LRC_CreateTex(this: *mut Il2CppObject, res: *mut i32) {
    unsafe {
        if !res.is_null() {
            let x = *res;
            let y = *res.add(1);
            info!("[mvfix] LRC.CreateRenderTexture {}x{}", x, y);
            // dump display-side geometry
            let daw = read_f32_field(this, 0x4c);
            let dah = read_f32_field(this, 0x50);
            let fsx = read_f32_field(this, 0x68);
            let fsy = read_f32_field(this, 0x6c);
            let ox = read_f32_field(this, 0x70);
            let oy = read_f32_field(this, 0x74);
            let r2x = read_i32_field(this, 0x58);
            let r2y = read_i32_field(this, 0x5c);
            let r3x = read_i32_field(this, 0x60);
            let r3y = read_i32_field(this, 0x64);
            let canvas = read_object_field(this, "_gameCanvasRect");
            let contents = read_object_field(this, "_contentsRootRect");
            let (cw, ch) = rect_size(canvas);
            let (qw, qh) = rect_size(contents);
            info!("[mvfix] LRC geo: disp={}x{} fix=({},{}) off=({},{}) r2d={}x{} r3d={}x{} canvasSZ=({},{}) contentsSZ=({},{})", daw, dah, fsx, fsy, ox, oy, r2x, r2y, r3x, r3y, cw, ch, qw, qh);
        }
    }
    get_orig_fn!(LRC_CreateTex, LRCCreateTexFn)(this, res)
}

unsafe fn read_i32_field(obj: *mut Il2CppObject, off: usize) -> i32 {
    if obj.is_null() { return 0; }
    *((obj as *const u8).add(off) as *const i32)
}

unsafe fn rect_size(rect: *mut Il2CppObject) -> (f32, f32) {
    if rect.is_null() { return (0.0, 0.0); }
    let klass = *(rect as *const *mut Il2CppClass);
    if klass.is_null() { return (0.0, 0.0); }
    let m = crate::il2cpp::api::il2cpp_class_get_method_from_name(klass, c"get_sizeDelta".as_ptr(), 0);
    if m.is_null() { return (0.0, 0.0); }
    let r = crate::il2cpp::api::il2cpp_runtime_invoke(m, rect as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
    if r.is_null() { return (0.0, 0.0); }
    let x = *((r as *const u8).add(16) as *const f32);
    let y = *((r as *const u8).add(20) as *const f32);
    (x, y)
}

unsafe fn log_stack_short(tag: &str) {
    let mut buf: [*mut std::ffi::c_void; 24] = [std::ptr::null_mut(); 24];
    let cnt = RtlCaptureStackBackTrace(2, 24, buf.as_mut_ptr(), std::ptr::null_mut());
    let addrs: Vec<String> = (0..cnt as usize).map(|i| format!("{:#x}", buf[i] as usize)).collect();
    info!("[mvfix] stack({}): {}", tag, addrs.join(" "));
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

// ViewControllerBase`1 / IViewController shared GetChangeViewOrientation
static mut GCVO_WRAPPER: *mut Il2CppObject = std::ptr::null_mut();
static mut GCVO_THIS: *mut Il2CppObject = std::ptr::null_mut();

type GCVOBaseFn = extern "C" fn(this: *mut Il2CppObject, retOrientation: *mut Il2CppObject) -> IEnumerator;
extern "C" fn GCVO_Base(this: *mut Il2CppObject, retOrientation: *mut Il2CppObject) -> IEnumerator {
    let enumerator = get_orig_fn!(GCVO_Base, GCVOBaseFn)(this, retOrientation);
    unsafe {
        GCVO_WRAPPER = retOrientation;
        GCVO_THIS = this;
    }
    if let Err(e) = enumerator.hook_move_next(GCVO_MoveNext) {
        error!("[mvfix] hook GCVO MoveNext: {}", e);
    }
    enumerator
}

extern "C" fn GCVO_MoveNext(enumerator: *mut Il2CppObject) -> bool {
    let moved = get_orig_fn!(GCVO_MoveNext, MoveNextFn)(enumerator);
    if !moved {
        unsafe {
            let wrapper = GCVO_WRAPPER;
            let this = GCVO_THIS;
            if !wrapper.is_null() && !this.is_null() {
                let k = *(this as *const *mut Il2CppClass);
                let cname = if k.is_null() { "?".to_string() } else {
                    std::ffi::CStr::from_ptr((*k).name).to_string_lossy().to_string()
                };
                let o = crate::il2cpp::hook::umamusume::Screen::ScreenOrientationClassWrapper::get_Orientation(wrapper);
                info!("[mvfix] GCVO done: class={} orientation={}", cname, o);
                GCVO_WRAPPER = std::ptr::null_mut();
                GCVO_THIS = std::ptr::null_mut();
            }
        }
    }
    moved
}

fn class_name_of(obj: *mut Il2CppObject) -> String {
    unsafe {
        if obj.is_null() { return "?".to_string(); }
        let k = *(obj as *const *mut Il2CppClass);
        if k.is_null() { return "?".to_string(); }
        std::ffi::CStr::from_ptr((*k).name).to_string_lossy().to_string()
    }
}

type CutInVoidFn = extern "C" fn(this: *mut Il2CppObject) -> ();

macro_rules! cutin_land_hook {
    ($name:ident) => {
        extern "C" fn $name(this: *mut Il2CppObject) {
            get_orig_fn!($name, CutInVoidFn)(this)
        }
    };
}

macro_rules! cutin_port_hook {
    ($name:ident) => {
        extern "C" fn $name(this: *mut Il2CppObject) {
            get_orig_fn!($name, CutInVoidFn)(this)
        }
    };
}

cutin_land_hook!(CI_Play_Base);
cutin_land_hook!(CI_Play_Cmn);
cutin_land_hook!(CI_Play_CharaBase);
cutin_port_hook!(CI_End_Cmn);
cutin_port_hook!(CI_EndSkip_Cmn);

// ==== Screen.get_IsVertical / get_IsSplitWindow 探针 ====
type IsVerticalFn = extern "C" fn() -> bool;
extern "C" fn IsVerticalHook() -> bool {
    let v = get_orig_fn!(IsVerticalHook, IsVerticalFn)();
    unsafe {
        static mut LAST_VERT: i32 = -1;
        if LAST_VERT != v as i32 {
            LAST_VERT = v as i32;
            info!("[mvfix] Screen.IsVertical -> {}", v);
        }
    }
    v
}

type IsSplitWindowFn = extern "C" fn() -> bool;
extern "C" fn IsSplitWindowHook() -> bool {
    let v = get_orig_fn!(IsSplitWindowHook, IsSplitWindowFn)();
    unsafe {
        static mut LAST_SPLIT: i32 = -1;
        if LAST_SPLIT != v as i32 {
            LAST_SPLIT = v as i32;
            info!("[mvfix] Screen.IsSplitWindow -> {}", v);
        }
    }
    v
}

// ==== 历史探针保留位 ====

type VoidPlayFn = extern "C" fn(this: *mut Il2CppObject) -> ();
extern "C" fn GachaCutCharaResult_Play(this: *mut Il2CppObject) {
    info!("[mvfix] GachaCutCharaResult.Play called");
    get_orig_fn!(GachaCutCharaResult_Play, VoidPlayFn)(this)
}
extern "C" fn GachaCutResultBase_Play(this: *mut Il2CppObject) {
    info!("[mvfix] GachaCutResultBase.Play called");
    get_orig_fn!(GachaCutResultBase_Play, VoidPlayFn)(this)
}

extern "C" fn GachaResultVC_PlayInView(this: *mut Il2CppObject) {
    info!("[mvfix] GachaResultViewController.PlayInView called");
    get_orig_fn!(GachaResultVC_PlayInView, VoidPlayFn)(this)
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

fn dump_filtered_sigs(umamusume: *const Il2CppImage, name: &str, kws: &[&str]) {
    let Ok(c_name) = std::ffi::CString::new(name) else { return; };
    unsafe {
        let k = find_class_any(umamusume, name);
        if k.is_null() { warn!("[mvcut] filtered class {} not found", name); return; }
        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut lines: Vec<String> = Vec::new();
        loop {
            let method = crate::il2cpp::api::il2cpp_class_get_methods(k, &mut iter);
            if method.is_null() { break; }
            if (*method).is_generic() != 0 { continue; }
            let mname = std::ffi::CStr::from_ptr((*method).name).to_string_lossy().to_string();
            if !kws.iter().any(|w| mname.contains(w)) { continue; }
            let count = crate::il2cpp::api::il2cpp_method_get_param_count(method);
            let mut params = Vec::new();
            for i in 0..count {
                let ty = crate::il2cpp::api::il2cpp_method_get_param(method, i);
                if ty.is_null() { params.push("?".to_string()); continue; }
                let p = crate::il2cpp::api::il2cpp_type_get_name(ty);
                params.push(if p.is_null() { "?".to_string() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().to_string() });
            }
            let ret = {
                let ty = crate::il2cpp::api::il2cpp_method_get_return_type(method);
                if ty.is_null() { "?".to_string() } else {
                    let p = crate::il2cpp::api::il2cpp_type_get_name(ty);
                    if p.is_null() { "?".to_string() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().to_string() }
                }
            };
            lines.push(format!("{}({}) -> {}", mname, params.join(", "), ret));
        }
        lines.sort();
        info!("[mvcut] ==== {} (filtered, {}) ====", name, lines.len());
        for l in &lines { info!("[mvcut]   {}", l); }
        let _ = c_name;
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

// ==== CutIn 进度条数据源 ====
pub static CUTIN_CTRL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub static CUTIN_TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static CUTIN_TOTAL_BITS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub static CUTIN_TOTAL_OWNER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub static CUTIN_DRAGGING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
            // 先重置时间轴状态（清掉"已激活但不可回退"的 UI 元素），再跳到目标时间
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

// CutInTimelineController 多路触发（不同演出可能走不同更新函数）
static CTL_TICK_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

unsafe fn ctl_touch(this: *mut Il2CppObject, tag: &str) {
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
                    info!("[mvfix] ctl({}) total={}", tag, total);
                } else {
                    info!("[mvfix] ctl({}) GetTotalTime null", tag);
                }
            }
        }
    }
    let n = CTL_TICK_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n < 6 {
        info!("[mvfix] ctl touch via {} this={:p}", tag, this);
    }
}

type CTLAlterLateFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn CTL_AlterLateUpdate(this: *mut Il2CppObject) -> bool {
    unsafe { ctl_touch(this, "AlterLateUpdate"); }
    get_orig_fn!(CTL_AlterLateUpdate, CTLAlterLateFn)(this)
}

type CTLAlterUpdateFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn CTL_AlterUpdate(this: *mut Il2CppObject) -> bool {
    unsafe { ctl_touch(this, "AlterUpdate"); }
    get_orig_fn!(CTL_AlterUpdate, CTLAlterUpdateFn)(this)
}

type CTLAlterFixedFn = extern "C" fn(this: *mut Il2CppObject, a: f32, b: f32) -> ();
extern "C" fn CTL_AlterFixedUpdate(this: *mut Il2CppObject, a: f32, b: f32) {
    unsafe { ctl_touch(this, "AlterFixedUpdate"); }
    get_orig_fn!(CTL_AlterFixedUpdate, CTLAlterFixedFn)(this, a, b)
}

// ==== 事件触发探针（定位"遮罩/文字"事件）====
static FIRE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

type FireEvent3Fn = extern "C" fn(this: *mut Il2CppObject, data: *mut Il2CppObject, a: i32, b: bool) -> ();
extern "C" fn FireEvent3(this: *mut Il2CppObject, data: *mut Il2CppObject, a: i32, b: bool) {
    let n = FIRE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n < 150 {
        info!("[mvfix] FireEvent3 #{} data={:p} a={} b={}", n, data, a, b);
    }
    get_orig_fn!(FireEvent3, FireEvent3Fn)(this, data, a, b)
}

type FireEvent2Fn = extern "C" fn(this: *mut Il2CppObject, key: *mut Il2CppObject, b: bool) -> ();
static BLOCK_LOG_ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static STRUCT_PROBE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
const BLOCKED_IDS: [i32; 8] = [61, 83, 126, 145, 173, 264, 281, 293];
extern "C" fn FireEvent2(this: *mut Il2CppObject, key: *mut Il2CppObject, b: bool) {
    unsafe {
        if !key.is_null() {
            let p = key as *const u8;
            let id = *((p).add(0x10) as *const i32);
            observe_shadow_frame(id);
            if Hachimi::instance().config.load().cutin_block_ui_events && is_blocked_frame(id) {
                if !BLOCK_LOG_ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    info!("[mvfix] blocked cutin UI event id={}", id);
                }
                return;
            }
            let n = STRUCT_PROBE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if n < 40 {
                let p18 = *(p.add(0x18) as *const usize);
                let mut cname = "null".to_string();
                if p18 > 0x10000 && is_readable(p18 as *const std::ffi::c_void, 16) {
                    let k = *(p18 as *const *mut Il2CppClass);
                    if !k.is_null() && is_readable(k as *const std::ffi::c_void, 8) {
                        cname = std::ffi::CStr::from_ptr((*k).name).to_string_lossy().to_string();
                    }
                }
                info!("[mvfix] evt #{} f10={} param={:#x} class={}", n, id, p18, cname);
            }
        }
    }
    get_orig_fn!(FireEvent2, FireEvent2Fn)(this, key, b)
}

fn dump_enum_names(umamusume: *const Il2CppImage, name: &str) {
    unsafe {
        let k = find_class_any(umamusume, name);
        if k.is_null() { warn!("[mvcut] enum {} not found", name); return; }
        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut idx: i32 = 0;
        loop {
            let f = crate::il2cpp::api::il2cpp_class_get_fields(k, &mut iter);
            if f.is_null() { break; }
            let fname = std::ffi::CStr::from_ptr((*f).name).to_string_lossy().to_string();
            if fname == "value__" { continue; }
            info!("[mvcut] enum {} [{}] = {}", name, idx, fname);
            idx += 1;
        }
        info!("[mvcut] enum {} total {} members", name, idx);
    }
}

fn is_readable(ptr: *const std::ffi::c_void, size: usize) -> bool {
    extern "system" { fn IsBadReadPtr(lp: *const std::ffi::c_void, cb: usize) -> i32; }
    unsafe { IsBadReadPtr(ptr, size) == 0 }
}

// 读取 Il2CppString（MovieProbe 内部用）
unsafe fn read_il2cpp_string(s: *mut Il2CppString) -> String {
    if s.is_null() { return String::new(); }
    let p = s as usize;
    let len = *((p + 16) as *const i32);
    if len <= 0 || len > 400 { return String::new(); }
    let chars = std::slice::from_raw_parts((p + 20) as *const u16, len as usize);
    String::from_utf16_lossy(chars)
}

// ==== 遮罩 UI 克隆拦截（按 prefab 名识别 + 每帧压制）====
static TAG_OBJECTS: once_cell::sync::Lazy<std::sync::Mutex<Vec<usize>>> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(Vec::new()));

type CloneSingleFn = extern "C" fn(prefab: *mut Il2CppObject, parent: *mut Il2CppObject, world: bool) -> *mut Il2CppObject;
extern "C" fn CloneSingle_hook(prefab: *mut Il2CppObject, parent: *mut Il2CppObject, world: bool) -> *mut Il2CppObject {
    let r = get_orig_fn!(CloneSingle_hook, CloneSingleFn)(prefab, parent, world);
    unsafe {
        if !prefab.is_null() {
            let k = *(prefab as *const *mut Il2CppClass);
            if !k.is_null() {
                let mn = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"get_name".as_ptr(), 0);
                if !mn.is_null() {
                    let nr = crate::il2cpp::api::il2cpp_runtime_invoke(mn, prefab as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
                    if !nr.is_null() {
                        let s = read_il2cpp_string(nr as *mut Il2CppString);
                        if s.contains("gacha_icon_new") || s.contains("gacha_chara_name") || s.contains("gacha_chrname") {
                            info!("[mvfix] tag prefab cloned: {}", s);
                            if !r.is_null() {
                                set_go_active(r, false);
                                if let Ok(mut v) = TAG_OBJECTS.lock() {
                                    if v.len() < 64 { v.push(r as usize); }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    r
}

// 1 参克隆（Instantiate(Object) 路径）：打印名字并匹配
static CLONE_LOG_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
type CloneSingle1Fn = extern "C" fn(prefab: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn CloneSingle1_hook(prefab: *mut Il2CppObject) -> *mut Il2CppObject {
    let r = get_orig_fn!(CloneSingle1_hook, CloneSingle1Fn)(prefab);
    unsafe {
        if !prefab.is_null() {
            let k = *(prefab as *const *mut Il2CppClass);
            if !k.is_null() {
                let mn = crate::il2cpp::api::il2cpp_class_get_method_from_name(k, c"get_name".as_ptr(), 0);
                if !mn.is_null() {
                    let nr = crate::il2cpp::api::il2cpp_runtime_invoke(mn, prefab as *mut std::ffi::c_void, std::ptr::null_mut(), std::ptr::null_mut());
                    if !nr.is_null() {
                        let s = read_il2cpp_string(nr as *mut Il2CppString);
                        let n = CLONE_LOG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if n < 120 {
                            info!("[mvfix] clone: {}", s);
                        }
                        if s.contains("gacha_icon_new") || s.contains("gacha_chara_name") || s.contains("gacha_chrname") {
                            info!("[mvfix] tag prefab cloned(1): {}", s);
                            if !r.is_null() {
                                set_go_active(r, false);
                                if let Ok(mut v) = TAG_OBJECTS.lock() {
                                    if v.len() < 64 { v.push(r as usize); }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    r
}

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

// ==== 遮罩帧自学习（重复触发 → 判定 → 写文件 + 自动拦截）====
static SHADOW_TRACK: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashMap<i32, (u64, u32)>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
static AUTO_BLOCK: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashSet<i32>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

fn shadow_file_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let p = exe.parent()?.join("hachimi").join("cutin_shadow_frames.txt");
    Some(p)
}

pub fn load_shadow_frames() {
    if let Some(p) = shadow_file_path() {
        if let Ok(txt) = std::fs::read_to_string(&p) {
            let Ok(mut set) = AUTO_BLOCK.lock() else { return; };
            for line in txt.lines() {
                if let Ok(v) = line.trim().parse::<i32>() { set.insert(v); }
            }
            info!("[mvfix] loaded {} shadow frames from {}", set.len(), p.display());
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
            info!("[mvfix] auto-block shadow frame {} -> {}", id, shadow_file_path().map(|p| p.display().to_string()).unwrap_or_default());
            append_shadow_frame(id);
        }
    }
}

fn is_blocked_frame(id: i32) -> bool {
    if BLOCKED_IDS.contains(&id) { return true; }
    AUTO_BLOCK.lock().map(|s| s.contains(&id)).unwrap_or(false)
}

pub fn init(umamusume: *const Il2CppImage) {
    if !Hachimi::instance().config.load().debug_mode {
        return;
    }

    unsafe {
        if let Ok(k) = get_class(umamusume, c"Gallop", c"StoryFullMoviePlayer") {
            let addr = get_method_addr(k, c"Init", 2);
            new_hook!(addr, SFMP_Init);
            let addr = get_method_addr(k, c"ChangeOrientationAsync", 1);
            new_hook!(addr, SFMP_ChangeOrientation);
            let addr = get_method_addr(k, c"RequestChangeOrientation", 0);
            new_hook!(addr, SFMP_RequestChangeOrientation);
            let addr = get_method_addr(k, c"AdjustMovieSize", 1);
            new_hook!(addr, SFMP_AdjustMovieSize);
        } else {
            error!("[mvprobe] StoryFullMoviePlayer not found");
        }

        if let Ok(k) = get_class(umamusume, c"Gallop", c"StoryMovieViewController") {
            let addr = get_method_addr(k, c"GetChangeViewOrientation", 1);
            new_hook!(addr, SMVC_GetChangeViewOrientation);
        }

        if let Ok(k) = get_class(umamusume, c"Gallop", c"PartsGachaMovieListItem") {
            let addr = get_method_addr(k, c"OnClick", 0);
            new_hook!(addr, Item_OnClick);
        }

        if let Ok(k) = get_class(umamusume, c"Gallop", c"PartsOthersMovieListItem") {
            let addr = get_method_addr(k, c"OnClick", 0);
            new_hook!(addr, Others_OnClick);
        }

        if let Ok(k) = get_class(umamusume, c"", c"GachaMovie") {
            let addr = get_method_addr(k, c"SetGachaIdAndLoad", 1);
            new_hook!(addr, GM_SetGachaIdAndLoad);
        }

        if let Ok(k) = get_class(umamusume, c"Gallop", c"FullScreenMoviePlayer") {
            let addr = get_method_addr(k, c"Init", 1);
            new_hook!(addr, FMP_Init);
            let addr = get_method_addr(k, c"LoadAndPlay", 3);
            new_hook!(addr, FMP_LoadAndPlay);
            let addr = get_method_addr(k, c"Play", 2);
            new_hook!(addr, FMP_Play);
            let addr = get_method_addr(k, c"AdjustMovieSizeHD", 0);
            new_hook!(addr, FMP_AdjustMovieSizeHD);
        } else {
            error!("[mvprobe] FullScreenMoviePlayer not found");
        }

        if let Ok(k) = get_class(umamusume, c"Gallop", c"FullScreenMoviePlayerEtoE") {
            let addr = get_method_addr(k, c"CalcScreenSize", 0);
            new_hook!(addr, EtoE_CalcScreenSize);
        }

        try_dump_method_addrs(umamusume, "DialogCharacterNoteActMovieList");
        try_dump_method_addrs(umamusume, "PartsGachaMovieListItem");
        try_dump_method_addrs(umamusume, "PartsOthersMovieListItem");
        try_dump_method_addrs(umamusume, "CharacterNoteMainViewController");
        try_dump_method_addrs(umamusume, "StoryMovieViewController");
        try_dump_method_addrs(umamusume, "FullScreenMoviePlayer");
        try_dump_method_addrs(umamusume, "GachaMovie");
        try_dump_method_addrs(umamusume, "CharacterNoteMainUIBase");
        try_dump_method_addrs(umamusume, "CharacterNoteCharaTopUI");
        try_dump_method_addrs(umamusume, "CharacterNoteMainView");
        try_dump_method_addrs(umamusume, "CharacterNoteInfo");
        try_dump_method_addrs(umamusume, "CharacterNoteTopViewController");
        try_dump_method_addrs(umamusume, "CharacterNoteHubViewController");
        try_dump_method_addrs(umamusume, "DialogManager");
        try_dump_method_addrs(umamusume, "UIManager");
        try_dump_method_addrs(umamusume, "SceneManager");
        try_dump_method_addrs(umamusume, "NowLoading");
        try_dump_method_addrs(umamusume, "MainGameInitializer");
        try_dump_method_addrs(umamusume, "StoryMovieView");
        try_dump_method_addrs(umamusume, "CharacterNote3DViewer");
        try_dump_method_addrs(umamusume, "SceneManager");
        try_dump_method_addrs(umamusume, "Screen");
        dump_all_method_addrs(umamusume);

        if let Ok(k) = get_class(umamusume, c"Gallop", c"SceneManager") {
            let addr = get_method_addr(k, c"ChangeView", 7);
            new_hook!(addr, SM_ChangeView);
            let addr = get_method_addr(k, c"get_IsLandscapeMode", 0);
            new_hook!(addr, SM_IsLandscape);
            unsafe { SM_CLASS = k; }
        } else {
            error!("[mvprobe] SceneManager not found");
        }

        scan_by_keywords(umamusume, &["CutIn", "Cutin"]);
        scan_by_keywords(umamusume, &["Gacha"]);
        dump_class_sigs(umamusume, "GachaCutInBase");
        dump_class_sigs(umamusume, "GachaCutInCmn");
        dump_class_sigs(umamusume, "GachaCutInCharaBase");
        dump_class_sigs(umamusume, "GachaCutInSingleCmn");
        dump_class_sigs(umamusume, "GachaCutInR3_1");
        dump_class_sigs(umamusume, "CutInModelController");
        dump_class_sigs(umamusume, "CutInPlayableAnimator");
        dump_class_sigs(umamusume, "CutInTimelineMotionCamera");
        dump_class_sigs(umamusume, "CutInTimelineEffectCamera");
        dump_class_sigs(umamusume, "CameraData");
        dump_class_fields(umamusume, "CameraData");
        dump_class_fields(umamusume, "CutInTimelineMotionCamera");
        dump_class_fields(umamusume, "Camera");
        dump_class_sigs(umamusume, "CharacterNote3DViewer");
        dump_class_fields(umamusume, "CharacterNote3DViewer");
        dump_class_sigs(umamusume, "GachaCutCharaResult");
        dump_class_fields(umamusume, "GachaCutCharaResult");
        dump_class_sigs(umamusume, "GachaCutResultBase");
        dump_class_fields(umamusume, "GachaCutResultBase");
        dump_class_sigs(umamusume, "GachaCutSupportResult");
        dump_class_fields(umamusume, "GachaCutSupportResult");
        dump_class_sigs(umamusume, "GachaResultIconPlayer");
        dump_class_fields(umamusume, "GachaResultIconPlayer");
        dump_class_sigs(umamusume, "GachaResultSet");
        dump_class_fields(umamusume, "GachaResultSet");
        dump_class_sigs(umamusume, "PartsGachaResultCard");
        dump_class_fields(umamusume, "PartsGachaResultCard");
        dump_class_sigs(umamusume, "GachaCardResult");
        dump_class_fields(umamusume, "GachaCardResult");
        dump_class_sigs(umamusume, "GachaCutInHelper");
        dump_class_fields(umamusume, "GachaCutInHelper");
        dump_class_sigs(umamusume, "CharacterButton");
        dump_class_fields(umamusume, "CharacterButton");
        dump_class_sigs(umamusume, "PartsGachaResult");
        dump_class_fields(umamusume, "PartsGachaResult");
        dump_class_sigs(umamusume, "PartsGachaResultPanel");
        dump_class_fields(umamusume, "PartsGachaResultPanel");
        dump_class_sigs(umamusume, "PartsGachaResultIcon");
        dump_class_fields(umamusume, "PartsGachaResultIcon");
        dump_class_sigs(umamusume, "GachaResultViewController");
        dump_class_fields(umamusume, "GachaResultViewController");
        dump_filtered_sigs(umamusume, "CutInTimelineController", &["Time", "Frame", "Skip", "Speed", "Pause", "Elapsed", "Current", "Play", "Update"]);
        dump_class_fields(umamusume, "CutInTimelineController");
        dump_filtered_sigs(umamusume, "CutInTimelineEventPublisher", &["FireEvent", "GetEvent", "BeginPublish", "EndPublish"]);
        dump_class_fields(umamusume, "CutInTimelineEventPublisher");
        dump_class_sigs(umamusume, "TimelineKeyEventData");
        dump_class_fields(umamusume, "TimelineKeyEventData");
        dump_class_fields(umamusume, "EventData");
        dump_enum_names(umamusume, "LiveTimelineEventID");
        dump_enum_names(umamusume, "TimelineEventID");
        dump_class_sigs(umamusume, "MiniCameraControllerBase");
        dump_class_fields(umamusume, "MiniCameraControllerBase");
        dump_class_sigs(umamusume, "LowResolutionCamera");
        dump_class_fields(umamusume, "LowResolutionCamera");
        scan_by_keywords(umamusume, &["Cutt"]);
        scan_by_keywords(umamusume, &["RenderPipeline"]);

        if let Ok(k2) = get_class(umamusume, c"Gallop.CutIn.Cutt", c"CutInTimelineMotionCamera") {
            let a1 = get_method_addr(k2, c"SetTargetCamera", 1);
            if a1 != 0 { new_hook!(a1, CutInSetTargetCamera); info!("[mvfix] hooked SetTargetCamera"); }
            let a2 = get_method_addr(k2, c"AlterLateUpdate", 3);
            if a2 != 0 { new_hook!(a2, CutInAlterLateUpdate); info!("[mvfix] hooked AlterLateUpdate"); }
        } else {
            warn!("[mvfix] CutInTimelineMotionCamera not found in Gallop.CutIn.Cutt");
        }

        let lrc = find_class_any(umamusume, "LowResolutionCamera");
        if !lrc.is_null() {
            let a = get_method_addr(lrc, c"InitializeVertical", 0);
            if a != 0 { new_hook!(a, LRC_InitVertical); info!("[mvfix] hooked InitializeVertical"); }
            let a = get_method_addr(lrc, c"InitializeHorizontal", 1);
            if a != 0 { new_hook!(a, LRC_InitHorizontal); info!("[mvfix] hooked InitializeHorizontal"); }
            let a = get_method_addr(lrc, c"CreateRenderTexture", 1);
            if a != 0 { new_hook!(a, LRC_CreateTex); info!("[mvfix] hooked CreateRenderTexture"); }
        } else {
            warn!("[mvfix] LowResolutionCamera not found");
        }

        let vcb = find_class_any(umamusume, "ViewControllerBase`1");
        if !vcb.is_null() {
            let a = get_method_addr(vcb, c"GetChangeViewOrientation", 1);
            if a != 0 { new_hook!(a, GCVO_Base); info!("[mvfix] hooked ViewControllerBase GCVO"); }
        } else {
            warn!("[mvfix] ViewControllerBase`1 not found");
        }

        // Screen 状态探针（只读日志）
        if let Ok(k) = get_class(umamusume, c"Gallop", c"Screen") {
            let a = get_method_addr(k, c"get_IsVertical", 0);
            if a != 0 { new_hook!(a, IsVerticalHook); info!("[mvfix] hooked get_IsVertical"); }
            let a = get_method_addr(k, c"get_IsSplitWindow", 0);
            if a != 0 { new_hook!(a, IsSplitWindowHook); info!("[mvfix] hooked get_IsSplitWindow"); }
        }
        if let Some(gcb) = cls_opt(umamusume, "GachaCutInBase") {
            let a = get_method_addr(gcb, c"Play", 0);
            if a != 0 { new_hook!(a, CI_Play_Base); info!("[mvfix] hooked GachaCutInBase.Play"); }
        }
        if let Some(gcc) = cls_opt(umamusume, "GachaCutInCmn") {
            let a = get_method_addr(gcc, c"Play", 0);
            if a != 0 { new_hook!(a, CI_Play_Cmn); info!("[mvfix] hooked GachaCutInCmn.Play"); }
            let a = get_method_addr(gcc, c"OnEnd", 0);
            if a != 0 { new_hook!(a, CI_End_Cmn); info!("[mvfix] hooked GachaCutInCmn.OnEnd"); }
            let a = get_method_addr(gcc, c"OnEndFromSkip", 0);
            if a != 0 { new_hook!(a, CI_EndSkip_Cmn); info!("[mvfix] hooked GachaCutInCmn.OnEndFromSkip"); }
        }
        if let Some(gch) = cls_opt(umamusume, "GachaCutInCharaBase") {
            let a = get_method_addr(gch, c"Play", 0);
            if a != 0 { new_hook!(a, CI_Play_CharaBase); info!("[mvfix] hooked GachaCutInCharaBase.Play"); }
        }
        if let Some(gccr) = cls_opt(umamusume, "GachaCutCharaResult") {
            let a = get_method_addr(gccr, c"Play", 0);
            if a != 0 { new_hook!(a, GachaCutCharaResult_Play); info!("[mvfix] hooked GachaCutCharaResult.Play"); }
        }
        if let Some(gcrb) = cls_opt(umamusume, "GachaCutResultBase") {
            let a = get_method_addr(gcrb, c"Play", 0);
            if a != 0 { new_hook!(a, GachaCutResultBase_Play); info!("[mvfix] hooked GachaCutResultBase.Play"); }
        }
        if let Some(grvc) = cls_opt(umamusume, "GachaResultViewController") {
            let a = get_method_addr(grvc, c"PlayInView", 0);
            if a != 0 { new_hook!(a, GachaResultVC_PlayInView); info!("[mvfix] hooked GachaResultViewController.PlayInView"); }
        }
        if let Some(ctl) = cls_opt(umamusume, "CutInTimelineController") {
            let a = get_method_addr(ctl, c"AlterLateUpdate", 0);
            if a != 0 { new_hook!(a, CTL_AlterLateUpdate); info!("[mvfix] hooked CutInTimelineController.AlterLateUpdate"); }
            let a = get_method_addr(ctl, c"AlterUpdate", 0);
            if a != 0 { new_hook!(a, CTL_AlterUpdate); info!("[mvfix] hooked CutInTimelineController.AlterUpdate"); }
            let a = get_method_addr(ctl, c"AlterFixedUpdate", 2);
            if a != 0 { new_hook!(a, CTL_AlterFixedUpdate); info!("[mvfix] hooked CutInTimelineController.AlterFixedUpdate"); }
        }
        if let Some(pubr) = cls_opt(umamusume, "CutInTimelineEventPublisher") {
            let a = get_method_addr(pubr, c"FireEvent", 3);
            if a != 0 { new_hook!(a, FireEvent3); info!("[mvfix] hooked FireEvent(3)"); }
            let a = get_method_addr(pubr, c"FireEvent", 2);
            if a != 0 { new_hook!(a, FireEvent2); info!("[mvfix] hooked FireEvent(2)"); }
        }
        // 遮罩克隆拦截：UnityEngine.Object.Internal_CloneSingleWithParent
        if let Ok(ui_img) = crate::il2cpp::symbols::get_assembly_image(c"UnityEngine.CoreModule") {
            if let Ok(obj_cls) = crate::il2cpp::symbols::get_class(ui_img, c"UnityEngine", c"Object") {
                let a = get_method_addr(obj_cls, c"Internal_CloneSingleWithParent", 3);
                if a != 0 { new_hook!(a, CloneSingle_hook); info!("[mvfix] hooked Internal_CloneSingleWithParent"); }
                let a = get_method_addr(obj_cls, c"Internal_CloneSingle", 1);
                if a != 0 { new_hook!(a, CloneSingle1_hook); info!("[mvfix] hooked Internal_CloneSingle"); }
            }
        }
        // 加载自学习遮罩帧列表
        load_shadow_frames();
        info!("[mvcut] scan done");
    }
}

fn cls_opt(umamusume: *const Il2CppImage, name: &str) -> Option<*mut Il2CppClass> {
    let k = find_class_any(umamusume, name);
    if k.is_null() { None } else { Some(k) }
}
