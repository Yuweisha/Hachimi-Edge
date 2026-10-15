use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use crate::{
    core::Hachimi,
    il2cpp::{
        api,
        ext::Il2CppObjectExt,
        sql,
        symbols::{get_method_addr, get_field_from_name, get_field_value, get_field_object_value, set_field_value, Array, Thread},
        types::*
    }
};

unsafe fn list_len(list: *mut Il2CppObject) -> i32 {
    if list.is_null() { return -1; }
    let k = (*list).klass();
    let f = get_field_from_name(k, c"_size");
    if f.is_null() { return -2; }
    get_field_value::<i32>(list, f)
}

unsafe fn list_item(list: *mut Il2CppObject, i: usize) -> *mut Il2CppObject {
    if list.is_null() { return std::ptr::null_mut(); }
    let k = (*list).klass();
    let f = get_field_from_name(k, c"_items");
    if f.is_null() { return std::ptr::null_mut(); }
    let arr = get_field_object_value::<Il2CppObject>(list, f);
    if arr.is_null() { return std::ptr::null_mut(); }
    let a = Array::<*mut Il2CppObject>::from(arr as *mut Il2CppArray);
    let slice = a.as_slice();
    if i < slice.len() { slice[i] } else { std::ptr::null_mut() }
}

static PLAYABLE_PRELOAD_SCHEDULED: AtomicBool = AtomicBool::new(false);
static PLAYABLE_IDS: RwLock<Option<Vec<i32>>> = RwLock::new(None);

fn playable_ids() -> Option<Vec<i32>> {
    PLAYABLE_IDS.read().unwrap().clone()
}

fn ensure_playable_ids() {
    if PLAYABLE_IDS.read().unwrap().is_some() { return; }
    if PLAYABLE_PRELOAD_SCHEDULED.swap(true, Ordering::AcqRel) { return; }
    Thread::main_thread().schedule(|| {
        let ids = sql::get_all_chara_ids();
        if !ids.is_empty() {
            info!("[unlock] all chara ids loaded: {} entries", ids.len());
            *PLAYABLE_IDS.write().unwrap() = Some(ids);
        } else {
            warn!("[unlock] chara id list preload failed");
        }
        PLAYABLE_PRELOAD_SCHEDULED.store(false, Ordering::Release);
    });
}

unsafe fn clone_item_with_id(template: *mut Il2CppObject, id: i32) -> *mut Il2CppObject {
    let k = (*template).klass();
    let size = api::il2cpp_object_get_size(template) as usize;
    let new_obj = api::il2cpp_object_new(k);
    if new_obj.is_null() { return std::ptr::null_mut(); }
    std::ptr::copy_nonoverlapping(template as *const u8, new_obj as *mut u8, size);
    let f = get_field_from_name(k, c"Id");
    if !f.is_null() {
        set_field_value::<i32>(new_obj, f, &id);
    }
    new_obj
}

unsafe fn fill_list(list: *mut Il2CppObject, ids: &[i32]) -> (i32, i32) {
    let n0 = list_len(list);
    if n0 < 2 { return (n0, 0); }

    let mut template: *mut Il2CppObject = std::ptr::null_mut();
    for i in 0..n0 as usize {
        let it = list_item(list, i);
        if it.is_null() { continue; }
        let f = get_field_from_name((*it).klass(), c"Id");
        if f.is_null() { continue; }
        let cur = get_field_value::<i32>(it, f);
        if cur >= 1000 { template = it; break; }
    }
    if template.is_null() {
        warn!("[unlock] no template character item in member list, skip");
        return (n0, 0);
    }

    let list_klass = (*list).klass();
    let mut insert_m = api::il2cpp_class_get_method_from_name(list_klass, c"Insert".as_ptr(), 2);
    let append_mode = insert_m.is_null();
    if append_mode {
        insert_m = api::il2cpp_class_get_method_from_name(list_klass, c"Add".as_ptr(), 1);
    }
    if insert_m.is_null() {
        warn!("[unlock] List.Insert/Add not found");
        return (n0, 0);
    }

    let mut added = 0i32;
    for &id in ids {
        let n = list_len(list);
        let mut exists = false;
        let mut pos: i32 = 1;
        let mut j = n - 1;
        while j >= 1 {
            let it = list_item(list, j as usize);
            if !it.is_null() {
                let f = get_field_from_name((*it).klass(), c"Id");
                if !f.is_null() {
                    let cur = get_field_value::<i32>(it, f);
                    if cur == id { exists = true; break; }
                    if cur < id { pos = j + 1; break; }
                }
            }
            j -= 1;
        }
        if exists { continue; }
        if append_mode { pos = list_len(list); }

        let new_obj = clone_item_with_id(template, id);
        if new_obj.is_null() { continue; }
        let mut args: [*mut std::ffi::c_void; 2] = if append_mode {
            [new_obj as *mut std::ffi::c_void, std::ptr::null_mut()]
        } else {
            [&pos as *const i32 as *mut std::ffi::c_void, new_obj as *mut std::ffi::c_void]
        };
        api::il2cpp_runtime_invoke(insert_m, list as *mut std::ffi::c_void, args.as_mut_ptr(), std::ptr::null_mut());
        added += 1;
    }
    (n0, added)
}

#[derive(Clone, Copy)]
struct DressMeta { chara_id: i32, order: i32 }

static DRESS_PRELOAD: AtomicBool = AtomicBool::new(false);
static DRESS_META: RwLock<Option<std::collections::HashMap<i32, DressMeta>>> = RwLock::new(None);
static CHARA_DRESS_IDS: RwLock<Option<std::collections::HashMap<i32, Vec<i32>>>> = RwLock::new(None);

fn chara_dress_ids_get(chara_id: i32) -> Option<Vec<i32>> {
    CHARA_DRESS_IDS.read().unwrap().as_ref().and_then(|m| m.get(&chara_id).cloned())
}

fn ensure_chara_dress_ids() {
    if CHARA_DRESS_IDS.read().unwrap().is_some() { return; }
    if DRESS_PRELOAD.swap(true, Ordering::AcqRel) { return; }
    Thread::main_thread().schedule(|| {
        let rows = sql::get_dress_sort_rows();
        if !rows.is_empty() {
            let mut meta: std::collections::HashMap<i32, DressMeta> = std::collections::HashMap::new();
            let mut by_chara: std::collections::HashMap<i32, Vec<(i32, i32)>> = std::collections::HashMap::new();
            for (id, chara, order, _use_lt) in rows {
                meta.insert(id, DressMeta { chara_id: chara, order });
                if chara > 0 {
                    by_chara.entry(chara).or_default().push((order, id));
                }
            }
            let mut chara_ids: std::collections::HashMap<i32, Vec<i32>> = std::collections::HashMap::new();
            for (chara, mut v) in by_chara {
                v.sort();
                chara_ids.insert(chara, v.into_iter().map(|(_, id)| id).collect());
            }
            info!("[unlock] dress table loaded: {} dresses, {} charas", meta.len(), chara_ids.len());
            *DRESS_META.write().unwrap() = Some(meta);
            *CHARA_DRESS_IDS.write().unwrap() = Some(chara_ids);
        } else {
            warn!("[unlock] dress table preload failed");
        }
        DRESS_PRELOAD.store(false, Ordering::Release);
    });
}

unsafe fn read_int_list(list: *mut Il2CppObject) -> Vec<i32> {
    let n = list_len(list);
    if n <= 0 { return Vec::new(); }
    let k = (*list).klass();
    let f = get_field_from_name(k, c"_items");
    if f.is_null() { return Vec::new(); }
    let arr = get_field_object_value::<Il2CppObject>(list, f);
    if arr.is_null() { return Vec::new(); }
    let a = Array::<i32>::from(arr as *mut Il2CppArray);
    let slice = a.as_slice();
    let mut out: Vec<i32> = Vec::with_capacity(n as usize);
    for i in 0..(n as usize).min(slice.len()) {
        out.push(slice[i]);
    }
    out
}

unsafe fn fill_dress_list(list: *mut Il2CppObject, extra: &[i32]) -> i32 {
    if list.is_null() || extra.is_empty() { return 0; }
    let guard = match DRESS_META.read() { Ok(g) => g, Err(_) => return 0 };
    let meta = match guard.as_ref() { Some(m) => m, None => return 0 };
    let insert_m = api::il2cpp_class_get_method_from_name((*list).klass(), c"Insert".as_ptr(), 2);
    if insert_m.is_null() {
        warn!("[unlock] List.Insert not found");
        return 0;
    }
    let mut added = 0i32;
    for &id in extra {
        let my = match meta.get(&id) { Some(m) => *m, None => continue };
        let current = read_int_list(list);
        if current.contains(&id) { continue; }

        let mut pos: i32 = -1;
        let mut last_same: i32 = -1;
        for (i, &eid) in current.iter().enumerate() {
            if let Some(em) = meta.get(&eid) {
                if my.chara_id > 0 && em.chara_id == my.chara_id {
                    if em.order > my.order { pos = i as i32; break; }
                    last_same = i as i32;
                }
            }
        }
        if pos < 0 {
            if last_same >= 0 {
                pos = last_same + 1;
            } else {
                let mut p: i32 = -1;
                for (i, &eid) in current.iter().enumerate() {
                    if let Some(em) = meta.get(&eid) {
                        if em.order > my.order { p = i as i32; break; }
                    }
                }
                pos = if p >= 0 { p } else { current.len() as i32 };
            }
        }

        let mut idx: i32 = pos;
        let mut val: i32 = id;
        let mut args: [*mut std::ffi::c_void; 2] = [
            &mut idx as *mut i32 as *mut std::ffi::c_void,
            &mut val as *mut i32 as *mut std::ffi::c_void,
        ];
        api::il2cpp_runtime_invoke(insert_m, list as *mut std::ffi::c_void, args.as_mut_ptr(), std::ptr::null_mut());
        added += 1;
    }
    added
}

type CharaPushDialogFn = extern "C" fn(list: *mut Il2CppObject, a: i32, b: i32, c: bool, cb: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn CharaPushDialog(list: *mut Il2CppObject, a: i32, b: i32, c: bool, cb: *mut Il2CppObject) -> *mut Il2CppObject {
    if Hachimi::instance().config.load().bv_unlock_all {
        unsafe {
            ensure_playable_ids();
            match playable_ids() {
                Some(ids) if !ids.is_empty() => {
                    let (orig, added) = fill_list(list, &ids);
                    if added > 0 {
                        info!("[unlock] member list +{} ({} -> {})", added, orig, list_len(list));
                    }
                }
                _ => {
                    debug!("[unlock] chara ids not ready, skipped");
                }
            }
        }
    }
    get_orig_fn!(CharaPushDialog, CharaPushDialogFn)(list, a, b, c, cb)
}

type DressPushDialogFn = extern "C" fn(a: i32, b: i32, c: bool, list: *mut Il2CppObject, member: i32, cb: *mut Il2CppObject, g: i32, h: i32) -> *mut Il2CppObject;
extern "C" fn DressPushDialog(a: i32, b: i32, c: bool, list: *mut Il2CppObject, member: i32, cb: *mut Il2CppObject, g: i32, h: i32) -> *mut Il2CppObject {
    if Hachimi::instance().config.load().bv_unlock_all {
        unsafe {
            ensure_chara_dress_ids();
            match chara_dress_ids_get(b) {
                Some(extra) => {
                    let added = fill_dress_list(list, &extra);
                    if added > 0 {
                        info!("[unlock] dress list +{} (chara={}, now {})", added, b, list_len(list));
                    }
                }
                None => {
                    debug!("[unlock] dress table not ready, skipped");
                }
            }
        }
    }
    get_orig_fn!(DressPushDialog, DressPushDialogFn)(a, b, c, list, member, cb, g, h)
}

type InitializeViewFn = extern "C" fn(this: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn InitializeView(this: *mut Il2CppObject) -> *mut Il2CppObject {
    if Hachimi::instance().config.load().bv_unlock_all {
        ensure_playable_ids();
        ensure_chara_dress_ids();
    }
    get_orig_fn!(InitializeView, InitializeViewFn)(this)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DialogLiveTheaterCharaSelect);
    get_class_or_return!(umamusume, Gallop, DialogLiveTheaterDressSelect);
    get_class_or_return!(umamusume, Gallop, LiveTheaterViewController);

    let addr = get_method_addr(DialogLiveTheaterCharaSelect, c"PushDialog", 5);
    new_hook!(addr, CharaPushDialog);

    let addr = get_method_addr(DialogLiveTheaterDressSelect, c"PushDialog", 8);
    new_hook!(addr, DressPushDialog);

    let addr = get_method_addr(LiveTheaterViewController, c"InitializeView", 0);
    new_hook!(addr, InitializeView);
}
