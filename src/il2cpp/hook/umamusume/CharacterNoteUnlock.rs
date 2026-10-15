use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use crate::{
    core::Hachimi,
    il2cpp::{
        api,
        ext::Il2CppObjectExt,
        sql,
        symbols::{get_method_addr, get_field_from_name, get_field_value, get_field_object_value, Array, Thread},
        types::*
    }
};

static PRELOAD_SCHEDULED: AtomicBool = AtomicBool::new(false);
static CHAR_CARD_IDS: RwLock<Option<std::collections::HashMap<i32, Vec<i32>>>> = RwLock::new(None);

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

fn ensure_card_table() {
    if CHAR_CARD_IDS.read().unwrap().is_some() { return; }
    if PRELOAD_SCHEDULED.swap(true, Ordering::AcqRel) { return; }
    Thread::main_thread().schedule(|| {
        let mut cards: std::collections::HashMap<i32, Vec<i32>> = std::collections::HashMap::new();
        for (id, chara) in sql::get_card_rows() {
            if chara > 0 && id < 9000000 { cards.entry(chara).or_default().push(id); }
        }
        if !cards.is_empty() {
            info!("[unlock] card table loaded: {} charas", cards.len());
            *CHAR_CARD_IDS.write().unwrap() = Some(cards);
        } else {
            warn!("[unlock] card table preload failed");
        }
        PRELOAD_SCHEDULED.store(false, Ordering::Release);
    });
}

unsafe fn get_owned_card_ids(this: *mut Il2CppObject, chara_id: i32) -> Option<Vec<i32>> {
    let m = api::il2cpp_class_get_method_from_name((*this).klass(), c"GetCardIdList".as_ptr(), 1);
    if m.is_null() { return None; }
    let mut val: i32 = chara_id;
    let mut args: [*mut std::ffi::c_void; 1] = [&mut val as *mut i32 as *mut std::ffi::c_void];
    let mut exc: *mut Il2CppException = std::ptr::null_mut();
    let r = api::il2cpp_runtime_invoke(m, this as *mut std::ffi::c_void, args.as_mut_ptr(), &mut exc);
    if !exc.is_null() || r.is_null() { return None; }
    Some(read_int_list(r))
}

unsafe fn get_any_card_template(this: *mut Il2CppObject) -> *mut Il2CppObject {
    let m = api::il2cpp_class_get_method_from_name((*this).klass(), c"GetCardList".as_ptr(), 0);
    if m.is_null() { return std::ptr::null_mut(); }
    let mut exc: *mut Il2CppException = std::ptr::null_mut();
    let r = api::il2cpp_runtime_invoke(m, this as *mut std::ffi::c_void, std::ptr::null_mut(), &mut exc);
    if !exc.is_null() || r.is_null() { return std::ptr::null_mut(); }
    list_item(r, 0)
}

unsafe fn clone_item_with_id(template: *mut Il2CppObject, id: i32) -> *mut Il2CppObject {
    let k = (*template).klass();
    let size = api::il2cpp_object_get_size(template) as usize;
    let new_obj = api::il2cpp_object_new(k);
    if new_obj.is_null() { return std::ptr::null_mut(); }
    std::ptr::copy_nonoverlapping(template as *const u8, new_obj as *mut u8, size);
    let setter = api::il2cpp_class_get_method_from_name(k, c"set_CardId".as_ptr(), 1);
    if !setter.is_null() {
        let mut val: i32 = id;
        let mut args: [*mut std::ffi::c_void; 1] = [&mut val as *mut i32 as *mut std::ffi::c_void];
        api::il2cpp_runtime_invoke(setter, new_obj as *mut std::ffi::c_void, args.as_mut_ptr(), std::ptr::null_mut());
    }
    new_obj
}

unsafe fn append_obj(list: *mut Il2CppObject, obj: *mut Il2CppObject) -> bool {
    let add_m = api::il2cpp_class_get_method_from_name((*list).klass(), c"Add".as_ptr(), 1);
    if add_m.is_null() { return false; }
    let mut args: [*mut std::ffi::c_void; 1] = [obj as *mut std::ffi::c_void];
    api::il2cpp_runtime_invoke(add_m, list as *mut std::ffi::c_void, args.as_mut_ptr(), std::ptr::null_mut());
    true
}

unsafe fn extend_card_list(list: *mut Il2CppObject, wanted: &[i32], owned: &[i32], template: *mut Il2CppObject) -> i32 {
    if list.is_null() || wanted.is_empty() || template.is_null() { return 0; }
    let mut added = 0i32;
    for &id in wanted {
        if owned.contains(&id) { continue; }
        let new_obj = clone_item_with_id(template, id);
        if new_obj.is_null() { continue; }
        if append_obj(list, new_obj) { added += 1; }
    }
    if added > 0 {
        info!("[unlock] card movie list +{} (now {})", added, list_len(list));
    }
    added
}

type WCDGetCardList1Fn = extern "C" fn(this: *mut Il2CppObject, chara_id: i32) -> *mut Il2CppObject;
extern "C" fn WCDGetCardList1(this: *mut Il2CppObject, chara_id: i32) -> *mut Il2CppObject {
    let ret = get_orig_fn!(WCDGetCardList1, WCDGetCardList1Fn)(this, chara_id);
    if Hachimi::instance().config.load().bv_unlock_all {
        ensure_card_table();
        unsafe {
            let wanted = CHAR_CARD_IDS.read().unwrap().as_ref().and_then(|m| m.get(&chara_id).cloned());
            if let Some(mut w) = wanted {
                w.sort();
                if !w.is_empty() {
                    let mut template = list_item(ret, 0);
                    if template.is_null() {
                        template = get_any_card_template(this);
                    }
                    match get_owned_card_ids(this, chara_id) {
                        Some(owned) => {
                            extend_card_list(ret, &w, &owned, template);
                        }
                        None => {
                            warn!("[unlock] GetCardIdList failed, skip extend");
                        }
                    }
                }
            }
        }
    }
    ret
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, WorkCardData);

    let addr = get_method_addr(WorkCardData, c"GetCardList", 1);
    new_hook!(addr, WCDGetCardList1);
}
