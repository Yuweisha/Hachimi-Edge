use std::sync::{atomic::{self, AtomicI32}, Mutex};

use crate::{core::Hachimi, il2cpp::{
    api::il2cpp_class_get_name,
    ext::Il2CppObjectExt,
    symbols::{get_field_object_value, get_method_addr, FieldsIter, GCHandle, IList},
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

    if Hachimi::instance().config.load().debug_mode {
        static DUMPED: AtomicI32 = AtomicI32::new(0);
        if DUMPED.fetch_add(1, atomic::Ordering::Relaxed) < 4 {
            dump_chara_tracks(get_TimelineData(this), block_id);
        }
    }

    get_orig_fn!(GotoBlock, GotoBlockFn)(this, block_id, weaken_cy_spring, is_update, is_choice);
}

fn class_name(class: *mut Il2CppClass) -> String {
    unsafe { std::ffi::CStr::from_ptr(il2cpp_class_get_name(class)).to_string_lossy().into_owned() }
}

pub fn dump_chara_tracks(timeline_data: *mut Il2CppObject, block_id: i32) {
    let block_list = super::StoryTimelineData::get_BlockList(timeline_data);
    let Some(block_list) = <IList>::new(block_list) else { return };
    let Some(block_data) = block_list.get(block_id) else { return };

    let chara_tracks = super::StoryTimelineBlockData::get_CharacterTrackList(block_data);
    let Some(chara_tracks) = <IList>::new(chara_tracks) else {
        info!("[storydump] block {} 没有角色轨道", block_id);
        return;
    };

    info!("[storydump] block={} 角色轨道数={}", block_id, chara_tracks.count());
    for (i, chara_track) in chara_tracks.iter().enumerate() {
        unsafe {
            let class = (*chara_track).klass();
            info!("[storydump]   track[{}] class={}", i, class_name(class));
            for field in FieldsIter::new(class) {
                let fname = std::ffi::CStr::from_ptr((*field).name).to_string_lossy().into_owned();
                let value = get_field_object_value::<Il2CppObject>(chara_track, field);
                if value.is_null() {
                    continue;
                }
                let vclass = (*value).klass();
                info!("[storydump]     {} = <{} @{:?}>", fname, class_name(vclass), value);
            }
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StoryTimelineController);

    let GotoBlock_addr = get_method_addr(StoryTimelineController, c"GotoBlock", 4);

    new_hook!(GotoBlock_addr, GotoBlock);

    unsafe {
        GET_ISFINISHED_ADDR = get_method_addr(StoryTimelineController, c"get_IsFinished", 0);
        GET_TIMELINEDATA_ADDR = get_method_addr(StoryTimelineController, c"get_TimelineData", 0);
    }
}