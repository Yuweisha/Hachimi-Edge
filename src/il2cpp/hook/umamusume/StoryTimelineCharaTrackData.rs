use std::ffi::CStr;

use crate::il2cpp::{symbols::{get_field_from_name, get_field_object_value, get_field_value, set_field_value, FieldsIter}, types::*};

// Special adaptation for the fields in this class because there are SO MANY OF THEM
static mut MOTION_TRACK_DATA_FIELDS: Vec<*mut FieldInfo> = Vec::new();
pub fn motion_track_data_values(this: *mut Il2CppObject) -> impl Iterator<Item = *mut Il2CppObject> {
    unsafe { MOTION_TRACK_DATA_FIELDS.iter().map(move |f| get_field_object_value(this, *f)) }
}

static mut CHARA_ID_FIELD: *mut FieldInfo = 0 as _;
static mut DRESS_ID_FIELD: *mut FieldInfo = 0 as _;
static mut HEAD_ID_FIELD: *mut FieldInfo = 0 as _;

pub fn get_CharaId(this: *mut Il2CppObject) -> i32 {
    get_field_value(this, unsafe { CHARA_ID_FIELD })
}

pub fn get_DressId(this: *mut Il2CppObject) -> i32 {
    get_field_value(this, unsafe { DRESS_ID_FIELD })
}

pub fn get_HeadId(this: *mut Il2CppObject) -> i32 {
    get_field_value(this, unsafe { HEAD_ID_FIELD })
}

pub fn set_track_chara(this: *mut Il2CppObject, chara_id: i32, dress_id: i32, head_id: i32) {
    set_field_value(this, unsafe { CHARA_ID_FIELD }, &chara_id);
    set_field_value(this, unsafe { DRESS_ID_FIELD }, &dress_id);
    set_field_value(this, unsafe { HEAD_ID_FIELD }, &head_id);
}


pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StoryTimelineCharaTrackData);

    unsafe {
        MOTION_TRACK_DATA_FIELDS = FieldsIter::new(StoryTimelineCharaTrackData)
            .filter(|f|
                CStr::from_ptr((**f).name)
                    .to_str()
                    .map(|s| s.ends_with("MotionTrackData"))
                    .unwrap_or(false)
            )
            .collect();

        CHARA_ID_FIELD = get_field_from_name(StoryTimelineCharaTrackData, c"CharaId");
        DRESS_ID_FIELD = get_field_from_name(StoryTimelineCharaTrackData, c"DressId");
        HEAD_ID_FIELD = get_field_from_name(StoryTimelineCharaTrackData, c"HeadId");
        if CHARA_ID_FIELD.is_null() || DRESS_ID_FIELD.is_null() || HEAD_ID_FIELD.is_null() {
            warn!(
                "[track] 字段查找不全: CharaId={:#x} DressId={:#x} HeadId={:#x}",
                CHARA_ID_FIELD as usize, DRESS_ID_FIELD as usize, HEAD_ID_FIELD as usize
            );
        } else {
            warn!(
                "[track] 字段就绪: CharaId@{} DressId@{} HeadId@{}",
                (*CHARA_ID_FIELD).offset, (*DRESS_ID_FIELD).offset, (*HEAD_ID_FIELD).offset
            );
        }

        // 调试：打印全部字段（找可能漏改的角色相关字段）
        for f in FieldsIter::new(StoryTimelineCharaTrackData) {
            let fname = CStr::from_ptr((*f).name).to_string_lossy().to_string();
            warn!("[track] 全字段: {} @{}", fname, (*f).offset);
        }
    }
}