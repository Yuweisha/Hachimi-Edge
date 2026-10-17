use std::ffi::CStr;

use crate::il2cpp::{ext::Il2CppObjectExt, symbols::{get_field_from_name, get_field_object_value, get_field_value, set_field_value, FieldsIter}, types::*};

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


/// Diagnostic: dump every MotionTrackData sub-object's fields for a track whose
/// dress id is in the character-specific range. Used to find where the
/// `_30`-style head/tail sub id in asset paths comes from.
pub fn dump_motion_track_data(this: *mut Il2CppObject, tag: &str) {
    unsafe {
        let mut idx = 0usize;
        for f in MOTION_TRACK_DATA_FIELDS.iter() {
            let name = CStr::from_ptr((**f).name).to_string_lossy().to_string();
            let obj: *mut Il2CppObject = get_field_object_value(this, *f);
            if obj.is_null() {
                warn!("[mtdump] {} #{} {} = null", tag, idx, name);
            } else {
                let klass = (*obj).klass();
                let mut klass_name = "?".to_string();
                let mut parts: Vec<String> = Vec::new();
                // 字段可能定义在父类里，逐级向上遍历
                let mut k = klass;
                let mut depth = 0;
                while !k.is_null() && depth < 6 {
                    if depth == 0 {
                        klass_name = CStr::from_ptr((*k).name).to_string_lossy().to_string();
                    }
                    for f2 in FieldsIter::new(k) {
                        let n2 = CStr::from_ptr((*f2).name).to_string_lossy().to_string();
                        let off = (*f2).offset;
                        if off <= 0 || off > 0x400 { continue; }
                        let v: i32 = *((obj as usize + off as usize) as *const i32);
                        parts.push(format!("{}={}", n2, v));
                    }
                    k = crate::il2cpp::api::il2cpp_class_get_parent(k);
                    depth += 1;
                }
                warn!("[mtdump] {} #{} {} -> {} [{}]", tag, idx, name, klass_name, parts.join(", "));
            }
            idx += 1;
        }
    }
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