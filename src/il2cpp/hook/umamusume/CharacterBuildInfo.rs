use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    core::{hachimi::GlobalCharReplaceConfig, Hachimi},
    il2cpp::symbols::Thread,
    il2cpp::{
        sql,
        symbols::{get_field_from_name, get_field_value, get_method_addr, set_field_value},
        types::*,
    },
};

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum UmaControllerType {
    Default = 0x0,
    Race = 0x1,
    Training = 0x2,
    EventTimeline = 0x3,
    Live = 0x4,
    LiveTheater = 0x5,
    HomeStand = 0x6,
    HomeTalk = 0x7,
    HomeWalk = 0x8,
    CutIn = 0x9,
    TrainingTop = 0xa,
    SingleRace = 0xb,
    Simple = 0xc,
    Mini = 0xd,
    Paddock = 0xe,
    Champions = 0xf,
    Orig = 0x1919810,
}

fn is_replacable(controller_type: i32) -> bool {
    !matches!(controller_type, 0x0 | 0x7 | 0x8 | 0xd)
}

fn orig_allowed(allow_orig: bool, controller_type: i32) -> bool {
    allow_orig || controller_type != UmaControllerType::Orig as i32
}

pub fn find_replacement(char_replace: &GlobalCharReplaceConfig, chara_id: i32, mini: bool) -> Option<(i32, i32)> {
    char_replace.data.iter()
        .find(|e| e.orig_char_id == chara_id && (!mini || e.replace_mini))
        .map(|e| (e.new_char_id, e.new_cloth_id))
}

const MINI_FALLBACK_DRESS_ID: i32 = 2;

fn dress_belongs_to(dress_id: i32, chara_id: i32) -> bool {
    match sql::get_dress_chara_id(dress_id) {
        0 => true,
        owner => owner == chara_id
    }
}

fn align_dress_with_chara(dress_id: &mut i32, chara_id: i32) {
    if dress_belongs_to(*dress_id, chara_id) {
        return;
    }
    let fallback = chara_id * 100 + 1;
    if sql::get_dress_info(fallback).is_some() {
        warn!(
            "dressId {} does not belong to chara {}! Replace to {}.",
            *dress_id, chara_id, fallback
        );
        *dress_id = fallback;
    } else {
        warn!(
            "dressId {} does not belong to chara {} and {} is missing!",
            *dress_id, chara_id, fallback
        );
    }
}

pub fn auto_pick_dress(orig_dress_id: i32, target_chara_id: i32, preferred_dress_id: i32) -> i32 {
    if orig_dress_id < 100000 {
        return orig_dress_id;
    }

    let want_head_sub = sql::get_head_id_from_dress_id(orig_dress_id);

    if want_head_sub == 0 && sql::get_dress_chara_id(preferred_dress_id) == target_chara_id {
        return preferred_dress_id;
    }

    let base = target_chara_id * 100;

    if let Some(d) = (base..base + 100).find(|d| {
        sql::get_dress_chara_id(*d) == target_chara_id
            && sql::get_head_id_from_dress_id(*d) == want_head_sub
    }) {
        return d;
    }

    let fallback = base + 1;
    if sql::get_dress_info(fallback).is_some() {
        return fallback;
    }

    orig_dress_id
}

static CHARA_PRELOAD_SCHEDULED: AtomicBool = AtomicBool::new(false);

fn chara_table_ready() -> bool {
    if sql::is_chara_data_ready() {
        return true;
    }

    if !CHARA_PRELOAD_SCHEDULED.swap(true, Ordering::AcqRel) {
        Thread::main_thread().schedule(|| {
            if !sql::preload_chara_data() {
                debug!("[replace] master.mdb 还没就绪，角色表稍后再加载");
            }
            CHARA_PRELOAD_SCHEDULED.store(false, Ordering::Release);
        });
    }

    false
}

static DRESS_PRELOAD_SCHEDULED: AtomicBool = AtomicBool::new(false);

fn dress_table_ready() -> bool {
    if sql::is_dress_info_ready() {
        return true;
    }

    if !DRESS_PRELOAD_SCHEDULED.swap(true, Ordering::AcqRel) {
        Thread::main_thread().schedule(|| {
            if !sql::preload_dress_info() {
                debug!("[replace] master.mdb 还没就绪，服装表稍后再加载");
            }
            DRESS_PRELOAD_SCHEDULED.store(false, Ordering::Release);
        });
    }

    false
}

#[derive(Clone, Copy, PartialEq)]
enum ReplaceMode {
    Rebuild,
    Ctor,
    Any,
}

fn replace_char_controller(
    chara_id: &mut i32, dress_id: &mut i32, head_id: &mut i32,
    controller_type: i32, allow_orig: bool, mode: ReplaceMode
) -> bool {
    let hachimi = Hachimi::instance();
    let config = hachimi.config.load();
    let char_replace = &config.replace_global_char;

    let enabled = match mode {
        ReplaceMode::Rebuild => char_replace.enable && !config.replace_mode_ctor,
        ReplaceMode::Ctor => config.replace_mode_ctor,
        ReplaceMode::Any => char_replace.enable || config.replace_mode_ctor,
    };

    if !enabled {
        return false;
    }

    if !dress_table_ready() {
        return false;
    }

    if !char_replace.replace_universal && !char_replace.auto_dress && *dress_id < 100000 {
        return false;
    }

    debug!(
        "[replace] 进入 ctrl={} chara={} dress={} head={}",
        controller_type, *chara_id, *dress_id, *head_id
    );

    if controller_type == UmaControllerType::Mini as i32 {
        if let Some((new_chara_id, new_dress_id)) = find_replacement(char_replace, *chara_id, true) {
            if sql::get_dress_have_mini(new_dress_id) {
                *chara_id = new_chara_id;
                *dress_id = new_dress_id;
                align_dress_with_chara(dress_id, new_chara_id);
                *head_id = sql::get_head_id_from_dress_id(*dress_id);
                return true;
            }
            warn!("dressId: {} does not have mini character!", new_dress_id);
            return false;
        }

        if !sql::get_dress_have_mini(*dress_id) {
            warn!("dressId: {} does not have mini character! Replace to {}.", *dress_id, MINI_FALLBACK_DRESS_ID);
            *dress_id = MINI_FALLBACK_DRESS_ID;
            return true;
        }
        return false;
    }

    if !is_replacable(controller_type) || !orig_allowed(allow_orig, controller_type) {
        return false;
    }

    if controller_type == UmaControllerType::EventTimeline as i32 {
        return false;
    }

    if !char_replace.replace_in_cutscene
        && matches!(
            controller_type,
            x if x == UmaControllerType::EventTimeline as i32
                || x == UmaControllerType::CutIn as i32
        )
    {
        return false;
    }

    if *chara_id == 9001 && controller_type == UmaControllerType::HomeStand as i32 {
        return false;
    }

    if let Some((new_chara_id, new_dress_id)) = find_replacement(char_replace, *chara_id, false) {
        if crate::il2cpp::hook::umamusume::SceneManager::is_race_scene_family()
            && (!chara_table_ready() || !sql::is_playable_chara(new_chara_id))
        {
            return false;
        }
        let orig = (*chara_id, *dress_id, *head_id);
        *chara_id = new_chara_id;
        *dress_id = if char_replace.auto_dress {
            auto_pick_dress(orig.1, new_chara_id, new_dress_id)
        } else {
            new_dress_id
        };
        align_dress_with_chara(dress_id, new_chara_id);
        *head_id = sql::get_head_id_from_dress_id(*dress_id);
        debug!(
            "[replace] ctrl={} chara {}->{} dress {}->{} head {}->{}",
            controller_type, orig.0, *chara_id, orig.1, *dress_id, orig.2, *head_id
        );
        return true;
    }

    false
}

static mut CHARA_ID_FIELD: *mut FieldInfo = 0 as _;
type CBICtor0Fn = extern "C" fn(this: *mut Il2CppObject, chara_id: i32, dress_id: i32, controller_type: i32, head_id: i32, zekken: i32, mob_id: i32, back_dancer_color_id: i32, is_use_dress_data_head_model_sub_id: bool, audience_id: i32, motion_dress_id: i32, is_enable_model_cache: bool, extra_bool: bool);
extern "C" fn CharacterBuildInfo_Ctor0(this: *mut Il2CppObject, chara_id: i32, dress_id: i32, controller_type: i32, head_id: i32, zekken: i32, mob_id: i32, back_dancer_color_id: i32, is_use_dress_data_head_model_sub_id: bool, audience_id: i32, motion_dress_id: i32, is_enable_model_cache: bool, extra_bool: bool) {
    let mut cid = chara_id;
    let mut did = dress_id;
    let mut hid = head_id;
    let replaced = replace_char_controller(&mut cid, &mut did, &mut hid, controller_type, false, ReplaceMode::Ctor);
    let (f_cid, f_did, f_hid) = if replaced {
        debug!("[replace-ctor0] chara {}->{} dress {}->{} ctrl={}", chara_id, cid, dress_id, did, controller_type);
        (cid, did, hid)
    } else {
        (chara_id, dress_id, head_id)
    };
    get_orig_fn!(CharacterBuildInfo_Ctor0, CBICtor0Fn)(this, f_cid, f_did, controller_type, f_hid, zekken, mob_id, back_dancer_color_id, is_use_dress_data_head_model_sub_id, audience_id, motion_dress_id, is_enable_model_cache, extra_bool)
}

type CBICtor1Fn = extern "C" fn(this: *mut Il2CppObject, card_id: i32, chara_id: i32, dress_id: i32, controller_type: i32, head_id: i32, zekken: i32, mob_id: i32, back_dancer_color_id: i32, override_cloth_category: i32, is_use_dress_data_head_model_sub_id: bool, audience_id: i32, motion_dress_id: i32, is_enable_model_cache: bool, chara_dress_color_set_id: i32, extra_bool: bool);
extern "C" fn CharacterBuildInfo_Ctor1(this: *mut Il2CppObject, card_id: i32, chara_id: i32, dress_id: i32, controller_type: i32, head_id: i32, zekken: i32, mob_id: i32, back_dancer_color_id: i32, override_cloth_category: i32, is_use_dress_data_head_model_sub_id: bool, audience_id: i32, motion_dress_id: i32, is_enable_model_cache: bool, chara_dress_color_set_id: i32, extra_bool: bool) {
    let mut cid = chara_id;
    let mut did = dress_id;
    let mut hid = head_id;
    let replaced = replace_char_controller(&mut cid, &mut did, &mut hid, controller_type, false, ReplaceMode::Ctor);
    let (f_cid, f_did, f_hid) = if replaced {
        debug!("[replace-ctor1] card={} chara {}->{} dress {}->{} ctrl={}", card_id, chara_id, cid, dress_id, did, controller_type);
        (cid, did, hid)
    } else {
        (chara_id, dress_id, head_id)
    };
    get_orig_fn!(CharacterBuildInfo_Ctor1, CBICtor1Fn)(this, card_id, f_cid, f_did, controller_type, f_hid, zekken, mob_id, back_dancer_color_id, override_cloth_category, is_use_dress_data_head_model_sub_id, audience_id, motion_dress_id, is_enable_model_cache, chara_dress_color_set_id, extra_bool)
}

static mut CARD_ID_FIELD: *mut FieldInfo = 0 as _;
static mut DRESS_ID_FIELD: *mut FieldInfo = 0 as _;
static mut CONTROLLER_TYPE_FIELD: *mut FieldInfo = 0 as _;
static mut HEAD_MODEL_SUB_ID_FIELD: *mut FieldInfo = 0 as _;
static mut MOTION_DRESS_ID_FIELD: *mut FieldInfo = 0 as _;

type CharacterBuildInfoRebuildFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn CharacterBuildInfo_Rebuild(this: *mut Il2CppObject) {
    unsafe {
        if !CHARA_ID_FIELD.is_null() {
            let mut chara_id: i32 = get_field_value(this, CHARA_ID_FIELD);
            let mut dress_id: i32 = get_field_value(this, DRESS_ID_FIELD);
            let controller_type: i32 = get_field_value(this, CONTROLLER_TYPE_FIELD);
            let mut head_model_sub_id: i32 = get_field_value(this, HEAD_MODEL_SUB_ID_FIELD);

            if replace_char_controller(&mut chara_id, &mut dress_id, &mut head_model_sub_id, controller_type, false, ReplaceMode::Rebuild) {
                set_field_value(this, CHARA_ID_FIELD, &chara_id);
                set_field_value(this, DRESS_ID_FIELD, &dress_id);
                set_field_value(this, HEAD_MODEL_SUB_ID_FIELD, &head_model_sub_id);
                set_field_value(this, MOTION_DRESS_ID_FIELD, &dress_id);
                let no_card_id = -1;
                set_field_value(this, CARD_ID_FIELD, &no_card_id);
            }
        }
    }

    get_orig_fn!(CharacterBuildInfo_Rebuild, CharacterBuildInfoRebuildFn)(this);
}

type GetRaceDressIdFn = extern "C" fn(this: *mut Il2CppObject, is_apply_dress_change: bool) -> i32;
extern "C" fn GetRaceDressId(this: *mut Il2CppObject, _is_apply_dress_change: bool) -> i32 {
    let ret = get_orig_fn!(GetRaceDressId, GetRaceDressIdFn)(this, false);

    if ret > 100000 && ret <= 999999 {
        let chara_id = if ret / 10000 == 90 { ret % 10000 } else { ret / 100 };
        let mut new_chara_id = chara_id;
        let mut new_dress_id = ret;
        let mut new_head_id = 0;
        if replace_char_controller(&mut new_chara_id, &mut new_dress_id, &mut new_head_id, UmaControllerType::Orig as i32, true, ReplaceMode::Any) {
            if dress_belongs_to(new_dress_id, chara_id) {
                return new_dress_id;
            }
            debug!(
                "[replace] GetRaceDressId: dress {} 不属于角色 {}，保留原值 {}",
                new_dress_id, chara_id, ret
            );
        }
    }

    ret
}

fn init_character_build_info(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, CharacterBuildInfo);

    let CharacterBuildInfo_Rebuild_addr = get_method_addr(CharacterBuildInfo, c"Rebuild", 0);
    new_hook!(CharacterBuildInfo_Rebuild_addr, CharacterBuildInfo_Rebuild);

    let ctor0 = get_method_addr(CharacterBuildInfo, c".ctor", 12);
    if ctor0 != 0 { new_hook!(ctor0, CharacterBuildInfo_Ctor0); }
    let ctor1 = get_method_addr(CharacterBuildInfo, c".ctor", 15);
    if ctor1 != 0 { new_hook!(ctor1, CharacterBuildInfo_Ctor1); }

    unsafe {
        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        loop {
            let m = crate::il2cpp::api::il2cpp_class_get_methods(CharacterBuildInfo, &mut iter);
            if m.is_null() { break; }
            let name = std::ffi::CStr::from_ptr((*m).name).to_string_lossy();
            if name == ".ctor" {
                let argc = crate::il2cpp::api::il2cpp_method_get_param_count(m);
                let mut sig: Vec<String> = Vec::new();
                for i in 0..argc {
                    let ty = crate::il2cpp::api::il2cpp_method_get_param(m, i);
                    let p = if ty.is_null() { std::ptr::null() } else { crate::il2cpp::api::il2cpp_type_get_name(ty) };
                    sig.push(if p.is_null() { "?".to_string() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().to_string() });
                }
                info!("[replace] CharacterBuildInfo .ctor argc={} addr={:#x} params=[{}]", argc, (*m).methodPointer, sig.join(", "));
            }
        }
    }

    unsafe {
        CHARA_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_charaId");
        CARD_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_cardId");
        DRESS_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_dressId");
        CONTROLLER_TYPE_FIELD = get_field_from_name(CharacterBuildInfo, c"_controllerType");
        HEAD_MODEL_SUB_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_headModelSubId");
        MOTION_DRESS_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_motionDressId");
    }
}

fn init_work_single_mode_chara_data(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, WorkSingleModeCharaData);

    let GetRaceDressId_addr = get_method_addr(WorkSingleModeCharaData, c"GetRaceDressId", 1);
    new_hook!(GetRaceDressId_addr, GetRaceDressId);
}

pub fn init(umamusume: *const Il2CppImage) {
    init_character_build_info(umamusume);
    init_work_single_mode_chara_data(umamusume);
}
