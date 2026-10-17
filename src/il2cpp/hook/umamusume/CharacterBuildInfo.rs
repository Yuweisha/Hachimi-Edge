use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    core::{hachimi::GlobalCharReplaceConfig, Hachimi},
    il2cpp::{ext::Il2CppObjectExt, symbols::Thread},
    il2cpp::{
        sql,
        symbols::{get_field_from_name, get_field_value, get_method_addr, set_field_value, FieldsIter},
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
    if controller_type == UmaControllerType::Orig as i32 {
        return false;
    }
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
    if *dress_id < 100000 {
        return;
    }

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

const SHARED_UNIVERSAL_DRESSES: &[i32] = &[3, 9];

fn is_universal_dress(dress_id: i32) -> bool {
    if sql::is_general_purpose_dress(dress_id) {
        return true;
    }
    SHARED_UNIVERSAL_DRESSES.contains(&dress_id)
}

pub fn auto_pick_dress(orig_dress_id: i32, target_chara_id: i32, preferred_dress_id: i32) -> i32 {
    let base = target_chara_id * 100;

    if orig_dress_id < 100000 {
        if is_universal_dress(orig_dress_id) {
            return orig_dress_id;
        }
        let fallback = base + 1;
        if sql::get_dress_info(fallback).is_some() {
            return fallback;
        }
        return orig_dress_id;
    }

    let want_head_sub = sql::get_head_id_from_dress_id(orig_dress_id);

    if want_head_sub == 0 && sql::get_dress_chara_id(preferred_dress_id) == target_chara_id {
        return preferred_dress_id;
    }

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

pub fn auto_pick_dress_for_track(
    orig_dress_id: i32,
    target_chara_id: i32,
    preferred_dress_id: i32,
) -> i32 {
    let base = target_chara_id * 100;
    if orig_dress_id < 100000 {
        if is_universal_dress(orig_dress_id) {
            return orig_dress_id;
        }
        let fallback = base + 1;
        if sql::get_dress_info(fallback).is_some() {
            return fallback;
        }
        return orig_dress_id;
    }

    let want_head_sub = sql::get_head_id_from_dress_id(orig_dress_id);
    if want_head_sub == 0 && sql::get_dress_chara_id(preferred_dress_id) == target_chara_id {
        return preferred_dress_id;
    }
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

fn replace_char_controller(
    chara_id: &mut i32, dress_id: &mut i32, head_id: &mut i32,
    controller_type: i32, allow_orig: bool
) -> bool {
    let hachimi = Hachimi::instance();
    let config = hachimi.config.load();
    let char_replace = &config.replace_global_char;

    if !char_replace.enable {
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

    if controller_type == UmaControllerType::EventTimeline as i32
        && crate::il2cpp::hook::umamusume::StoryTimelineData::SKIP_CUTSCENE_REPLACE.load(Ordering::Acquire)
    {
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
        if char_replace.chara_only {
            // 只换角色：服装/头部变体保持原样，用来判断卡死是否由换装导致
            debug!(
                "[replace] chara_only: chara {}->{} dress {} kept head {} kept",
                orig.0, new_chara_id, orig.1, orig.2
            );
            return true;
        }
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

pub static CURRENT_CONTROLLER_TYPE: std::sync::atomic::AtomicI32 =
    std::sync::atomic::AtomicI32::new(-1);

static mut CHARA_ID_FIELD: *mut FieldInfo = 0 as _;
static mut CARD_ID_FIELD: *mut FieldInfo = 0 as _;
static mut DRESS_ID_FIELD: *mut FieldInfo = 0 as _;
static mut CONTROLLER_TYPE_FIELD: *mut FieldInfo = 0 as _;
static mut HEAD_MODEL_SUB_ID_FIELD: *mut FieldInfo = 0 as _;
static mut MOTION_DRESS_ID_FIELD: *mut FieldInfo = 0 as _;
static mut USE_DRESS_DATA_HEAD_FIELD: *mut FieldInfo = 0 as _;
static mut BODY_MODEL_SUB_ID_FIELD: *mut FieldInfo = 0 as _;
static mut MOTION_BODY_MODEL_SUB_ID_FIELD: *mut FieldInfo = 0 as _;

static CBINFO_DUMPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Diagnostic: dump every CharacterBuildInfo field for one character build, so
/// we can see where an outfit/head sub id that never appears in the timeline
/// track (e.g. dress 100230 / head 30) is coming from.
unsafe fn dump_build_info(this: *mut Il2CppObject, tag: &str) {
    let klass = (*this).klass();
    if klass.is_null() {
        return;
    }
    let mut parts: Vec<String> = Vec::new();
    let mut klass_name = String::new();
    // 实际类型可能是子类，字段散在父类链上 —— 逐级向上收集
    let mut k = klass;
    let mut depth = 0;
    while !k.is_null() && depth < 8 {
        if depth == 0 {
            klass_name = std::ffi::CStr::from_ptr((*k).name).to_string_lossy().to_string();
        }
        for f in FieldsIter::new(k) {
            let name = std::ffi::CStr::from_ptr((*f).name).to_string_lossy().to_string();
            let off = (*f).offset;
            if off <= 0 || off > 0x800 {
                continue;
            }
            let v: i32 = *((this as usize + off as usize) as *const i32);
            parts.push(format!("{}={}", name, v));
        }
        k = crate::il2cpp::api::il2cpp_class_get_parent(k);
        depth += 1;
    }
    warn!("[cbinfo] {} ({}) [{}]", tag, klass_name, parts.join(", "));
}

type CharacterBuildInfoRebuildFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn CharacterBuildInfo_Rebuild(this: *mut Il2CppObject) {
    unsafe {
        if !CHARA_ID_FIELD.is_null() {
            let mut chara_id: i32 = get_field_value(this, CHARA_ID_FIELD);
            let mut dress_id: i32 = get_field_value(this, DRESS_ID_FIELD);
            let controller_type: i32 = get_field_value(this, CONTROLLER_TYPE_FIELD);
            CURRENT_CONTROLLER_TYPE.store(controller_type, Ordering::Release);
            let mut head_model_sub_id: i32 = get_field_value(this, HEAD_MODEL_SUB_ID_FIELD);

            if (controller_type == UmaControllerType::EventTimeline as i32
                || head_model_sub_id != 0)
                && CBINFO_DUMPS.fetch_add(1, Ordering::Relaxed) < 20
            {
                dump_build_info(
                    this,
                    &format!(
                        "ctrl={} chara={} dress={} head={}",
                        controller_type, chara_id, dress_id, head_model_sub_id
                    ),
                );
            }

            if replace_char_controller(&mut chara_id, &mut dress_id, &mut head_model_sub_id, controller_type, false) {
                set_field_value(this, CHARA_ID_FIELD, &chara_id);
                set_field_value(this, DRESS_ID_FIELD, &dress_id);
                set_field_value(this, HEAD_MODEL_SUB_ID_FIELD, &head_model_sub_id);
                // 游戏默认按 dress_data[dressId].head_sub_id 反推头部变体；dress 换掉之后
                // 那个值仍指向原服装的子类型（100230 -> 30），导致资源加载 chr{原角色}_30
                // 与替换后的骨架对不上。清 0 让游戏只用我们写好的 _headModelSubId。
                if !USE_DRESS_DATA_HEAD_FIELD.is_null() {
                    let no = 0i32;
                    set_field_value(this, USE_DRESS_DATA_HEAD_FIELD, &no);
                }

                // 动作资源按 type 分包，沿用原角色的 type 会去取目标角色没有的那几档。
                // 用 chara_type 查目标角色在该场景下的 type；查不到就保持原值。
                if controller_type == UmaControllerType::EventTimeline as i32 {
                    let orig_type: i32 = get_field_value(this, BODY_MODEL_SUB_ID_FIELD);
                    if let Some(t) = sql::get_chara_motion_type(chara_id, 3, 0)
                        .or_else(|| sql::get_chara_motion_type(chara_id, 0, 0))
                    {
                        if t != orig_type {
                            warn!(
                                "[replace] motion type {} -> {} (chara {})",
                                orig_type, t, chara_id
                            );
                            set_field_value(this, BODY_MODEL_SUB_ID_FIELD, &t);
                            if !MOTION_BODY_MODEL_SUB_ID_FIELD.is_null() {
                                set_field_value(this, MOTION_BODY_MODEL_SUB_ID_FIELD, &t);
                            }
                        }
                    }
                }

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
        if replace_char_controller(&mut new_chara_id, &mut new_dress_id, &mut new_head_id, UmaControllerType::Orig as i32, true) {
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

    unsafe {
        CHARA_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_charaId");
        CARD_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_cardId");
        DRESS_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_dressId");
        CONTROLLER_TYPE_FIELD = get_field_from_name(CharacterBuildInfo, c"_controllerType");
        HEAD_MODEL_SUB_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_headModelSubId");
        MOTION_DRESS_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_motionDressId");
        USE_DRESS_DATA_HEAD_FIELD = get_field_from_name(CharacterBuildInfo, c"_isUseDressDataHeadModelSubId");
        BODY_MODEL_SUB_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_bodyModelSubId");
        MOTION_BODY_MODEL_SUB_ID_FIELD = get_field_from_name(CharacterBuildInfo, c"_motionBodyModelSubId");
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
