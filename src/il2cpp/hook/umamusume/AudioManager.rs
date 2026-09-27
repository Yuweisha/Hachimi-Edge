use std::sync::atomic::Ordering;
use crate::{
    core::{Hachimi, captions, game::Region, gui, utils::{race_seek_seh, race_seek_stage}},
    il2cpp::{
        hook::Cute_Cri_Assembly::{
            AudioPlayback::{self, AudioPlayback_t},
            AtomSourceEx,
        },
        ext::{Il2CppStringExt, StringExt},
        symbols::{
            get_assembly_image, get_class, get_method_addr, get_field_from_name,
            Array, SingletonLike, Thread
        },
        types::*
    }
};
use super::{
    RaceManager,
    RaceBGMController,
    RaceSoundReplay,
};

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Category {
    BGM = 0,
    SE = 1,
    VOICE = 2,
    JIKKYO = 3,
    LIVE = 4
}

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

static mut GET_CRIAUDIOMANAGER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_CriAudioManager, GET_CRIAUDIOMANAGER_ADDR, *mut Il2CppObject,);

def_field_value_accessors!(get__songPlayback, set__songPlayback, _SONGPLAYBACK_FIELD, AudioPlayback_t);
def_field_object_accessors!(get__songCharaPlaybacks, set__songCharaPlaybacks, _SONGCHARAPLAYBACKS_FIELD, Il2CppArray);
def_field_value_accessors!(get__bgmPlayback, set__bgmPlayback, _BGMPLAYBACK_FIELD, AudioPlayback_t);
def_field_object_accessors!(get__atomSourceArrayBGM, set__atomSourceArrayBGM, _ATOMSOURCEARRAYBGM_FIELD, Il2CppArray);

def_method_wrapper_fn!(GetCueLength, GET_CUE_LENGTH_ADDR, f32, this: *mut Il2CppObject, cue_sheet: *mut Il2CppString, cue_id: i32);
def_method_wrapper_fn!(GetVolume, GET_VOLUME_ADDR, f32, category: Category);

def_method_wrapper_fn!(IsAvailableCueSheet, IS_AVAILABLE_CUE_SHEET_ADDR, bool, this: *mut Il2CppObject, cue_sheet: *mut Il2CppString);

///
///
pub fn cue_sheet_available(sheet: &str) -> bool {
    if unsafe { IS_AVAILABLE_CUE_SHEET_ADDR } == 0 {
        return true;
    }
    let manager = instance();
    if manager.is_null() {
        return true;
    }
    let cue = sheet.to_il2cpp_string();
    if cue.is_null() {
        return true;
    }
    IsAvailableCueSheet(manager, cue)
}

pub fn race_slider_music_base() -> bool {
    let audio_manager = instance();
    if audio_manager.is_null() { return false; }

    let mut bgm = get__bgmPlayback(audio_manager);
    if bgm.criAtomExPlayback.id == 0 { return false; }

    let mut num_samples: i64 = 0;
    let mut sampling_rate: i32 = 0;
    if !AudioPlayback::GetNumPlayedSamples(&mut bgm, &mut num_samples, &mut sampling_rate) {
        return false;
    }
    if sampling_rate <= 0 { return false; }

    let secs = num_samples as f32 / sampling_rate as f32;
    if !secs.is_finite() || secs < 0.0 { return false; }

    gui::RACE_SLIDER_MUSIC_TIME.store(secs.to_bits(), Ordering::Release);
    true
}

// public Void PlayBgmFromName(ref String cueName, Boolean isLoop, Single volume, Single fadeInTime, Single fadeOutTime, Single startTime, Boolean isCrossFade, AutoStopType stopType)
def_method_wrapper_fn!(
    PlayBgmFromName, PLAY_BGM_FROM_NAME_ADDR, (),
    this: *mut Il2CppObject, cue_name: *mut *mut Il2CppString, is_loop: bool, volume: f32,
    fade_in_time: f32, fade_out_time: f32, start_time: f32, is_cross_fade: bool, stop_type: i32
);

pub fn play_race_bgm_cue(cue_name: *mut Il2CppString, position_secs: f32, bgm_volume: f32) -> bool {
    let audio_manager = instance();
    if audio_manager.is_null() { return false; }
    if cue_name.is_null() { return false; }

    let position = if position_secs.is_finite() && position_secs > 0.0 { position_secs } else { 0.0 };
    let volume = if bgm_volume.is_finite() && bgm_volume >= 0.0 { bgm_volume } else { 1.0 };
    let mut cue_name = cue_name;
    PlayBgmFromName(
        audio_manager, &mut cue_name, true, volume, 0.1, 0.1, position, false, 0
    );
    true
}

pub fn resync_race_music(race_manager: *mut Il2CppObject, target_time: f32) -> bool {
    let race_sound = RaceManager::get_RaceSound(race_manager);
    if !RaceSoundReplay::is_replay_sound(race_sound) { return true; }

    let bgm_controller = RaceSoundReplay::get_BGMController(race_sound);
    if !RaceBGMController::is_bgm_controller(bgm_controller) { return true; }

    race_seek_seh(|| {
        race_seek_stage(12); // music_volume
        let bgm_volume = RaceSoundReplay::GetBGMVolume(race_sound);
        let second_start = RaceBGMController::get_secondBgmStartTime(bgm_controller);
        let race_base = f32::from_bits(gui::RACE_SLIDER_DRAG_START_TIME.load(Ordering::Acquire));
        let music_valid = gui::RACE_SLIDER_MUSIC_VALID.load(Ordering::Acquire);
        let music_base = f32::from_bits(gui::RACE_SLIDER_MUSIC_TIME.load(Ordering::Acquire));

        let audio_manager = instance();
        if audio_manager.is_null() { return; }

        let sources_ptr = get__atomSourceArrayBGM(audio_manager);
        if sources_ptr.is_null() { return; }
        let sources: Array<*mut Il2CppObject> = Array::from(sources_ptr);

        race_seek_stage(13); // music_sweep
        for source in unsafe { sources.as_slice() }.iter() {
            if source.is_null() { continue; }
            if !AtomSourceEx::get_IsInUse(*source) { continue; }
            AtomSourceEx::Stop(*source, 0.0, 0);
        }

        race_seek_stage(14); // music_play_cue
        if second_start.is_finite() && second_start > 0.0 && target_time >= second_start {
            let position = if race_base >= second_start && music_valid {
                music_base + (target_time - race_base)
            } else {
                target_time - second_start
            };
            play_race_bgm_cue(RaceBGMController::get_secondBgmCueName(bgm_controller), position, bgm_volume);
            RaceBGMController::set_isRequestFirstBGM(bgm_controller, true);
            RaceBGMController::set_isStoppedFirstBGM(bgm_controller, true);
            RaceBGMController::set_isPlayedSecondBGM(bgm_controller, true);
        } else {
            let delay = RaceBGMController::get_firstBGMDelayTime(bgm_controller);
            let first_stop = RaceBGMController::get_firstBgmStopTime(bgm_controller);
    
            if target_time >= delay {
                let position = if race_base < first_stop && music_valid {
                    music_base + (target_time - race_base)
                } else {
                    target_time - delay
                };
                play_race_bgm_cue(RaceBGMController::get_firstBgmCueName(bgm_controller), position, bgm_volume);
                RaceBGMController::set_isRequestFirstBGM(bgm_controller, true);
                RaceBGMController::set_isStoppedFirstBGM(bgm_controller, false);
            } else {
                RaceBGMController::set_isRequestFirstBGM(bgm_controller, false);
                RaceBGMController::set_isStoppedFirstBGM(bgm_controller, false);
            }
            RaceBGMController::set_isPlayedSecondBGM(bgm_controller, false);
        }

        if Hachimi::instance().game.region == Region::Japan || Hachimi::instance().game.region == Region::Taiwan {
            let trigger_start = RaceBGMController::get_firstTriggerBgmPlayStartTime(bgm_controller);
            RaceBGMController::set_isPlayedFirstTriggerBgm(bgm_controller, target_time >= trigger_start);
        }

        race_seek_stage(0); // idle
    })
}

fn cue_str(p: *mut Il2CppString) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { &*p }.as_utf16str().to_string()
    }
}

// Cute.Cri.Audio RequestCueInfo
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct RequestCueInfo {
    pub CueSheetName: *mut Il2CppString,
    pub CueName: *mut Il2CppString,
    pub CueId: i32,
}

// Cute.Cri SoundGroup
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum SoundGroup {
    Bgm = 0,
    Se = 1,
    Voice = 2,
}

// private AudioPlayback PlayInternal(SoundGroup group, RequestCueInfo cueInfo, PlayParameters playParam, AutoStopType stopType) { }
type PlayInternalFn = extern "C" fn(this: *mut Il2CppObject, group: SoundGroup,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback_t;
extern "C" fn PlayInternal(this: *mut Il2CppObject, group: SoundGroup,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback_t {
    let log_cues = !cue_info.is_null()
        && Hachimi::instance().config.load().replace_global_char.log_audio_cues;
    let to_str = cue_str;

    if log_cues {
        let info = unsafe { *cue_info };
        debug!("[cue] group={:?}({}) sheet={} name='{}' id={} stop_type={}",
            group, group as i32, to_str(info.CueSheetName), to_str(info.CueName),
            info.CueId, stop_type);
    }

    if group == SoundGroup::Voice && !cue_info.is_null() {
        let new_sheet = crate::core::voice_replace::rewrite_cue_sheet(unsafe { *cue_info }.CueSheetName);
        if let Some(new_sheet) = new_sheet {
            unsafe { (*cue_info).CueSheetName = new_sheet; }
        }
    }

    let result = get_orig_fn!(PlayInternal, PlayInternalFn)(this, group, cue_info, play_param, stop_type);

    if log_cues {
        debug!("[cue]   -> playback_id={} error={} src_index={} used_sheet={}",
            result.criAtomExPlayback.id, result.isError, result.atomSourceListIndex,
            to_str(result.cueSheetName));

        if group == SoundGroup::Bgm {
            let song = get__songPlayback(this);
            let charas = get__songCharaPlaybacks(this);
            let count = if charas.is_null() {
                0
            } else {
                Array::<*mut Il2CppObject>::from(charas).len()
            };
            debug!("[cue]   songPlayback.id={} songCharaPlaybacks.len={}",
                song.criAtomExPlayback.id, count);
        }
    }

    if group == SoundGroup::Voice && !cue_info.is_null() && Hachimi::instance().config.load().caption.caption_enable {
        let cue_sheet_ptr = unsafe { *cue_info }.CueSheetName;
        if !cue_sheet_ptr.is_null() {
            let cue_sheet_ptr = unsafe { *cue_info }.CueSheetName;
            let cue_sheet = if !cue_sheet_ptr.is_null() {
                unsafe { &*cue_sheet_ptr }.as_utf16str().to_string()
            } else {
                String::new()
            };

            let cue_name_ptr = unsafe { *cue_info }.CueName;
            let cue_name = if !cue_name_ptr.is_null() {
                unsafe { &*cue_name_ptr }.as_utf16str().to_string()
            } else {
                String::new()
            };

            let cue_id = unsafe { *cue_info }.CueId;

            debug!("[captions] PlayInternal Voice: cue_sheet={}, name='{}', id={}", cue_sheet, cue_name, cue_id);

            if let Some(last) = cue_sheet.rsplit('_').next() {
                if last.len() >= 6 {
                    if let Ok(chara_id) = last[..4].parse::<i32>() {
                        let caption_data = captions::CaptionData {
                            text: String::new(), 
                            cue_sheet: cue_sheet.clone(),
                            cue_id,
                            character_id: chara_id,
                            voice_id: 0,
                        };

                        match captions::CAPTION_REQUEST.lock() {
                            Ok(mut slot) => *slot = Some(caption_data),
                            Err(poisoned) => {
                                warn!("[captions] CAPTION_REQUEST mutex poisoned, recovering...");
                                *poisoned.into_inner() = Some(caption_data);
                            }
                        }

                        Thread::main_thread().schedule(captions::process_caption_request);
                    }
                }
            }
        }
    }

    result
}

// private AudioPlayback _prepareSong(Int32, RequestCueInfo, PlayParameters, AutoStopType) { }
//
type PrepareSongFn = extern "C" fn(this: *mut Il2CppObject, part: i32,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback_t;
extern "C" fn PrepareSong(this: *mut Il2CppObject, part: i32,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback_t {
    let log_cues = !cue_info.is_null()
        && Hachimi::instance().config.load().replace_global_char.log_audio_cues;

    if log_cues {
        let info = unsafe { *cue_info };
        debug!("[song] part={} sheet={} name='{}' id={} stop_type={}",
            part, cue_str(info.CueSheetName), cue_str(info.CueName), info.CueId, stop_type);
    }

    if !cue_info.is_null() {
        let force = Hachimi::instance().config.load().replace_global_char.song_force_chara;
        let new_sheet = if force != 0 {
            crate::core::voice_replace::rewrite_cue_sheet_to(
                unsafe { *cue_info }.CueSheetName, force
            )
        } else {
            crate::core::voice_replace::rewrite_cue_sheet(
                unsafe { *cue_info }.CueSheetName
            )
        };
        if let Some(new_sheet) = new_sheet {
            unsafe { (*cue_info).CueSheetName = new_sheet; }
        }
    }

    let result = get_orig_fn!(PrepareSong, PrepareSongFn)(
        this, part, cue_info, play_param, stop_type
    );

    if log_cues {
        debug!("[song]   -> playback_id={} error={} src_index={} used_sheet={}",
            result.criAtomExPlayback.id, result.isError, result.atomSourceListIndex,
            cue_str(result.cueSheetName));
    }

    result
}

// public CriAtomCueSheet AddCueSheetByCueName(String) { }
//
//
type AddCueSheetByCueNameFn = extern "C" fn(this: *mut Il2CppObject,
    cue_name: *mut Il2CppString) -> *mut Il2CppObject;
extern "C" fn AddCueSheetByCueName(this: *mut Il2CppObject,
    cue_name: *mut Il2CppString
) -> *mut Il2CppObject {
    let orig = get_orig_fn!(AddCueSheetByCueName, AddCueSheetByCueNameFn);
    let log_cues = Hachimi::instance().config.load().replace_global_char.log_audio_cues;

    let name = cue_str(cue_name);
    let result = orig(this, cue_name);

    if log_cues && name.contains("live") {
        debug!("[song] AddCueSheetByCueName('{}') -> {}",
            name, if result.is_null() { "null" } else { "ok" });
    }

    if let Some(target) = crate::core::voice_replace::rewrite_cue_sheet_name(&name) {
        if log_cues {
            debug!("[song]   顺带加载替换目标: {}", target);
        }
        let ptr = target.to_il2cpp_string();
        if !ptr.is_null() {
            let added = orig(this, ptr);
            if log_cues {
                debug!("[song]   替换目标加载结果: {}",
                    if added.is_null() { "null（资源不存在）" } else { "ok" });
            }
        }
    }

    result
}

// public Boolean AddSongCueSheet(Int32, String[]) { }
//
//
type AddSongCueSheetFn = extern "C" fn(this: *mut Il2CppObject, music_id: i32,
    sheets: *mut Il2CppArray) -> bool;
extern "C" fn AddSongCueSheet(this: *mut Il2CppObject, music_id: i32,
    sheets: *mut Il2CppArray
) -> bool {
    let orig = get_orig_fn!(AddSongCueSheet, AddSongCueSheetFn);
    let log_cues = Hachimi::instance().config.load().replace_global_char.log_audio_cues;

    let names: Vec<String> = if sheets.is_null() {
        Vec::new()
    } else {
        unsafe { Array::<*mut Il2CppString>::from(sheets).as_slice() }
            .iter()
            .map(|item| cue_str(*item))
            .collect()
    };

    if log_cues {
        debug!("[song] AddSongCueSheet(music_id={}, {} 条): {}",
            music_id, names.len(), names.join(", "));
    }

    let (replaced, changed) = replaced_song_sheets(&names);
    if changed == 0 {
        return orig(this, music_id, sheets);
    }

    let Ok(mscorlib) = get_assembly_image(c"mscorlib.dll") else { return orig(this, music_id, sheets) };
    let Ok(string_class) = get_class(mscorlib, c"System", c"String") else {
        return orig(this, music_id, sheets);
    };

    if log_cues {
        debug!("[song] 名单替换了 {} 条人声轨: {}", changed, replaced.join(", "));
    }

    let array = Array::<*mut Il2CppString>::new(string_class, replaced.len());
    if array.this.is_null() {
        return orig(this, music_id, sheets);
    }

    unsafe {
        let slice = array.as_slice();
        for (i, name) in replaced.iter().enumerate() {
            slice[i] = name.to_il2cpp_string();
        }
    }

    let ok = orig(this, music_id, array.this);

    if log_cues {
        debug!("[song] 合并列表加载结果: {}", ok);
    }

    ok
}

///
fn replaced_song_sheets(names: &[String]) -> (Vec<String>, usize) {
    let config = Hachimi::instance().config.load();
    let char_replace = &config.replace_global_char;
    if !char_replace.enable {
        return (names.to_vec(), 0);
    }

    let force = char_replace.song_force_chara;
    let mut replaced = Vec::with_capacity(names.len());
    let mut changed = 0;

    for name in names {
        let target = if force != 0 {
            match crate::core::voice_replace::chara_id_in(name) {
                Some((chara_id, range)) if chara_id != force => {
                    let mut new_name = name.clone();
                    new_name.replace_range(range, &format!("{:04}", force));
                    Some(new_name)
                }
                _ => None,
            }
        } else {
            crate::core::voice_replace::rewrite_cue_sheet_name(name)
        };

        match target {
            Some(target) => {
                replaced.push(target);
                changed += 1;
            }
            None => replaced.push(name.clone()),
        }
    }

    (replaced, changed)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, AudioManager);

    let play_internal_addr = get_method_addr(AudioManager, c"PlayInternal", 4);
    new_hook!(play_internal_addr, PlayInternal);

    let prepare_song_addr = get_method_addr(AudioManager, c"_prepareSong", 4);
    if prepare_song_addr != 0 {
        new_hook!(prepare_song_addr, PrepareSong);
    }

    let add_song_sheet_addr = get_method_addr(AudioManager, c"AddSongCueSheet", 2);
    if add_song_sheet_addr != 0 {
        new_hook!(add_song_sheet_addr, AddSongCueSheet);
    }

    let add_cue_by_name_addr = get_method_addr(AudioManager, c"AddCueSheetByCueName", 1);
    if add_cue_by_name_addr != 0 {
        new_hook!(add_cue_by_name_addr, AddCueSheetByCueName);
    }

    unsafe {
        CLASS = AudioManager;
        GET_CRIAUDIOMANAGER_ADDR = get_method_addr(AudioManager, c"get_CriAudioManager", 0);
        GET_CUE_LENGTH_ADDR = get_method_addr(AudioManager, c"GetCueLength", 2);
        IS_AVAILABLE_CUE_SHEET_ADDR = get_method_addr(AudioManager, c"IsAvailableCueSheet", 1);
        PLAY_BGM_FROM_NAME_ADDR = get_method_addr(AudioManager, c"PlayBgmFromName", 8);
        GET_VOLUME_ADDR = get_method_addr(AudioManager, c"GetVolume", 1);

        _SONGPLAYBACK_FIELD = get_field_from_name(AudioManager, c"_songPlayback");
        _SONGCHARAPLAYBACKS_FIELD = get_field_from_name(AudioManager, c"_songCharaPlaybacks");
        _BGMPLAYBACK_FIELD = get_field_from_name(AudioManager, c"_bgmPlayback");
        _ATOMSOURCEARRAYBGM_FIELD = get_field_from_name(AudioManager, c"_atomSourceArrayBGM");

        if Hachimi::instance().config.load().debug_mode {
            let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
            let mut names: Vec<String> = Vec::new();
            let mut song_apis: Vec<String> = Vec::new();

            loop {
                let method = crate::il2cpp::api::il2cpp_class_get_methods(AudioManager, &mut iter);
                if method.is_null() { break; }
                if (*method).is_generic() != 0 { continue; }

                let name = std::ffi::CStr::from_ptr((*method).name)
                    .to_string_lossy()
                    .to_string();

                if name.contains("Song") || name.contains("CueSheet")
                    || name == "PrepareCharaPlaybacks" {
                    let type_name = |ty: *const crate::il2cpp::types::Il2CppType| -> String {
                        if ty.is_null() {
                            "?".to_string()
                        } else {
                            let p = crate::il2cpp::api::il2cpp_type_get_name(ty);
                            if p.is_null() {
                                "?".to_string()
                            } else {
                                std::ffi::CStr::from_ptr(p).to_string_lossy().to_string()
                            }
                        }
                    };

                    let count = crate::il2cpp::api::il2cpp_method_get_param_count(method);
                    let mut params = Vec::new();
                    for i in 0..count {
                        params.push(type_name(
                            crate::il2cpp::api::il2cpp_method_get_param(method, i)
                        ));
                    }
                    let ret = type_name(
                        crate::il2cpp::api::il2cpp_method_get_return_type(method)
                    );
                    song_apis.push(format!(
                        "{}({}) -> {}", name, params.join(", "), ret
                    ));
                }

                names.push(name);
            }

            names.sort();
            song_apis.sort();
            debug!("[am] AudioManager methods ({}): {}", names.len(), names.join(", "));
            for api in song_apis {
                debug!("[am] song api: {}", api);
            }
        }
    }
}
