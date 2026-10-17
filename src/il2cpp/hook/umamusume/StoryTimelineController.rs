use std::sync::{atomic::{self, AtomicI32}, Mutex};

use crate::{core::Hachimi, il2cpp::{
    symbols::{get_method_addr, GCHandle},
    types::*,
}};

static mut GET_ISFINISHED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_IsFinished, GET_ISFINISHED_ADDR, bool, this: *mut Il2CppObject);

static mut GET_TIMELINEDATA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_TimelineData, GET_TIMELINEDATA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub static CURRENT: Mutex<Option<GCHandle>> = Mutex::new(None);
static LAST_BLOCK_ID: AtomicI32 = AtomicI32::new(-1);

pub fn last_block_id() -> i32 {
    LAST_BLOCK_ID.load(atomic::Ordering::Relaxed)
}

type GotoBlockFn = extern "C" fn(this: *mut Il2CppObject, block_id: i32, weaken_cy_spring: bool, is_update: bool, is_choice: bool);
pub extern "C" fn GotoBlock(this: *mut Il2CppObject, block_id: i32, weaken_cy_spring: bool, is_update: bool, is_choice: bool) {
    if Hachimi::instance().config.load().enable_ipc {
        let mut guard = CURRENT.lock().unwrap();

        if !(*guard).as_ref().is_none_or(|h| h.target() == this) {
            *guard = Some(GCHandle::new_weak_ref(this, false));
        }
        LAST_BLOCK_ID.store(block_id, atomic::Ordering::Relaxed);
    }

    // 全量跟踪：进入/离开每个 block，附带 IsFinished，用来定位卡在哪个 block。
    let finished_before = if unsafe { GET_ISFINISHED_ADDR } != 0 {
        get_IsFinished(this)
    }
    else {
        false
    };
    warn!(
        "[story-ctl] GotoBlock {} enter (weaken={} update={} choice={} finished={})",
        block_id, weaken_cy_spring, is_update, is_choice, finished_before
    );

    get_orig_fn!(GotoBlock, GotoBlockFn)(this, block_id, weaken_cy_spring, is_update, is_choice);

    let finished_after = if unsafe { GET_ISFINISHED_ADDR } != 0 {
        get_IsFinished(this)
    }
    else {
        false
    };
    warn!(
        "[story-ctl] GotoBlock {} exit  (finished={})",
        block_id, finished_after
    );
}


/// 诊断：列出 Gallop 里所有名字含 "Story" 的类及其方法（名称 + 参数个数），
/// 用来找出实际驱动剧情推进的控制器，而不是只看 StoryTimelineController。
unsafe fn dump_story_classes(umamusume: *const Il2CppImage) {
    use std::ffi::CStr;
    let count = crate::il2cpp::api::il2cpp_image_get_class_count(umamusume);
    warn!("[storydump] image has {} classes", count);
    for i in 0..count {
        let klass = crate::il2cpp::api::il2cpp_image_get_class(umamusume, i);
        if klass.is_null() {
            continue;
        }
        let name = CStr::from_ptr((*klass).name).to_string_lossy();
        if !name.contains("Story") && !name.contains("Timeline") {
            continue;
        }
        let mut methods: Vec<String> = Vec::new();
        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        loop {
            let m = crate::il2cpp::api::il2cpp_class_get_methods(klass as *mut _, &mut iter);
            if m.is_null() {
                break;
            }
            let mname = CStr::from_ptr((*m).name).to_string_lossy();
            let argc = crate::il2cpp::api::il2cpp_method_get_param_count(m);
            methods.push(format!("{}/{}", mname, argc));
        }
        warn!("[storydump] {} -> {}", name, methods.join(", "));
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StoryTimelineController);

    unsafe { dump_story_classes(umamusume); }

    let GotoBlock_addr = get_method_addr(StoryTimelineController, c"GotoBlock", 4);

    new_hook!(GotoBlock_addr, GotoBlock);

    unsafe {
        GET_ISFINISHED_ADDR = get_method_addr(StoryTimelineController, c"get_IsFinished", 0);
        GET_TIMELINEDATA_ADDR = get_method_addr(StoryTimelineController, c"get_TimelineData", 0);
    }
}