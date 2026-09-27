
use std::ffi::c_void;
use std::ops::Range;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::RwLock;

use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use crate::core::{Hachimi, utils::get_masterdb_path};
use crate::il2cpp::ext::{Il2CppStringExt, StringExt};
use crate::il2cpp::hook::LibNative_Runtime::Sqlite3::{Connection, Query};
use crate::il2cpp::symbols::Thread;
use crate::il2cpp::types::Il2CppString;

const CHARA_ID_MIN: i32 = 1000;
const CHARA_ID_MAX: i32 = 1999;

static LAST_CUE_SHEET: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub fn rewrite_cue_sheet(cue_sheet: *mut Il2CppString) -> Option<*mut Il2CppString> {
    if cue_sheet.is_null() {
        return None;
    }

    let sheet = unsafe { &*cue_sheet }.as_utf16str().to_string();
    let (chara_id, range) = chara_id_in(&sheet)?;

    let hachimi = Hachimi::instance();
    let config = hachimi.config.load();
    let char_replace = &config.replace_global_char;
    if !char_replace.enable {
        return None;
    }

    let entry = char_replace
        .data
        .iter()
        .find(|entry| entry.orig_char_id == chara_id)?;
    if entry.new_char_id == 0 || entry.new_char_id == chara_id {
        return None;
    }

    let mut new_sheet = sheet.clone();
    new_sheet.replace_range(range, &format!("{:04}", entry.new_char_id));

    let new_ptr = new_sheet.to_il2cpp_string();
    if new_ptr.is_null() {
        return None;
    }

    LAST_CUE_SHEET.store(new_ptr as *mut c_void, Ordering::SeqCst);
    debug!("[voice] {} -> {}", sheet, new_sheet);
    Some(new_ptr)
}

pub fn rewrite_cue_sheet_name(sheet: &str) -> Option<String> {
    let (chara_id, range) = chara_id_in(sheet)?;

    let config = Hachimi::instance().config.load();
    let char_replace = &config.replace_global_char;
    if !char_replace.enable {
        return None;
    }

    let entry = char_replace
        .data
        .iter()
        .find(|entry| entry.orig_char_id == chara_id)?;
    if entry.new_char_id == 0 || entry.new_char_id == chara_id {
        return None;
    }

    let mut new_sheet = sheet.to_string();
    new_sheet.replace_range(range, &format!("{:04}", entry.new_char_id));
    Some(new_sheet)
}

pub fn rewrite_cue_sheet_to(cue_sheet: *mut Il2CppString, chara_id: i32) -> Option<*mut Il2CppString> {
    if cue_sheet.is_null() {
        return None;
    }

    let sheet = unsafe { &*cue_sheet }.as_utf16str().to_string();
    let (orig_id, range) = chara_id_in(&sheet)?;
    if orig_id == chara_id {
        return None;
    }

    let mut new_sheet = sheet.clone();
    new_sheet.replace_range(range, &format!("{:04}", chara_id));

    let new_ptr = new_sheet.to_il2cpp_string();
    if new_ptr.is_null() {
        return None;
    }

    debug!("[voice] {sheet} -> {new_sheet}（强制）");
    Some(new_ptr)
}

pub fn effective_char_id(chara_id: i32) -> i32 {
    let hachimi = Hachimi::instance();
    let config = hachimi.config.load();
    let char_replace = &config.replace_global_char;
    if !char_replace.enable {
        return chara_id;
    }

    char_replace
        .data
        .iter()
        .find(|entry| entry.orig_char_id == chara_id && entry.new_char_id != 0)
        .map(|entry| entry.new_char_id)
        .unwrap_or(chara_id)
}

static SYSTEM_TEXT: Lazy<RwLock<FnvHashMap<(i32, i32), String>>> =
    Lazy::new(|| RwLock::new(FnvHashMap::default()));
static SYSTEM_TEXT_LOADING: AtomicBool = AtomicBool::new(false);
static SYSTEM_TEXT_READY: AtomicBool = AtomicBool::new(false);

pub fn load_system_text() {
    let mut map = FnvHashMap::default();
    let db_path = get_masterdb_path();
    let conn = Connection::new();

    if Connection::Open(conn, db_path.to_il2cpp_string(), ptr::null_mut(), ptr::null_mut(), 0) {
        let sql = "SELECT character_id, voice_id, text FROM character_system_text";
        let query = Connection::Query(conn, sql.to_il2cpp_string());
        if !query.is_null() {
            while Query::Step(query) {
                let text_ptr = Query::GetText(query, 2);
                let text = unsafe { text_ptr.as_ref() }
                    .map(|s| s.as_utf16str().to_string())
                    .unwrap_or_default();
                map.insert((Query::GetInt(query, 0), Query::GetInt(query, 1)), text);
            }
            Query::Dispose(query);
        }
        Connection::CloseDB(conn);
    }

    if map.is_empty() {
        warn!("[voice] character_system_text 为空，稍后再试");
    } else {
        debug!("[voice] 已缓存 {} 条台词文本", map.len());
        *SYSTEM_TEXT.write().unwrap() = map;
        SYSTEM_TEXT_READY.store(true, Ordering::Release);
    }

    SYSTEM_TEXT_LOADING.store(false, Ordering::Release);
}

pub fn text_for(chara_id: i32, voice_id: i32) -> Option<*mut Il2CppString> {
    if let Some(text) = SYSTEM_TEXT.read().unwrap().get(&(chara_id, voice_id)) {
        return Some(text.to_il2cpp_string());
    }

    if !SYSTEM_TEXT_READY.load(Ordering::Acquire)
        && !SYSTEM_TEXT_LOADING.swap(true, Ordering::AcqRel)
    {
        Thread::main_thread().schedule(load_system_text);
    }

    None
}

pub fn chara_id_in(sheet: &str) -> Option<(i32, Range<usize>)> {
    if let Some(at) = sheet.rfind("_chara_") {
        let start = at + "_chara_".len();
        if let Some(head) = sheet.get(start..start + 4) {
            if let Ok(chara_id) = head.parse::<i32>() {
                if (CHARA_ID_MIN..=CHARA_ID_MAX).contains(&chara_id) {
                    return Some((chara_id, start..start + 4));
                }
            }
        }
    }

    if let Some(at) = sheet.rfind("_vo_") {
        let start = at + "_vo_".len();
        if let Some(head) = sheet.get(start..start + 4) {
            if let Ok(chara_id) = head.parse::<i32>() {
                if (CHARA_ID_MIN..=CHARA_ID_MAX).contains(&chara_id) {
                    return Some((chara_id, start..start + 4));
                }
            }
        }
    }

    let start = sheet.rfind('_').map(|index| index + 1).unwrap_or(0);
    let segment = &sheet[start..];
    if segment.len() < 4 {
        return None;
    }

    let head = &segment[..4];
    if !head.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let chara_id: i32 = head.parse().ok()?;
    if !(CHARA_ID_MIN..=CHARA_ID_MAX).contains(&chara_id) {
        return None;
    }

    Some((chara_id, start..start + 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(sheet: &str) -> Option<i32> {
        chara_id_in(sheet).map(|(id, _)| id)
    }

    #[test]
    fn reads_character_ids_from_real_cue_names() {
        assert_eq!(parse("snd_voi_training_110301"), Some(1103));
        assert_eq!(parse("snd_voi_training_110300"), Some(1103));
        assert_eq!(parse("snd_voi_live_111400"), Some(1114));
        assert_eq!(parse("snd_voi_home_111401"), Some(1114));
        assert_eq!(parse("snd_voi_outgame_102901"), Some(1029));
        assert_eq!(parse("snd_voi_title_1053"), Some(1053));
        assert_eq!(parse("snd_voi_tc_1053"), Some(1053));

        assert_eq!(parse("1157/snd_bgm_live_1157_chara_1003_01"), Some(1003));
        assert_eq!(parse("1157/snd_bgm_live_1157_chara_1022_01"), Some(1022));
        assert_eq!(parse("1157/snd_bgm_live_1157_vo_1006_02"), Some(1006));
        assert_eq!(parse("1157/snd_bgm_live_1157_oke_01"), None);
        assert_eq!(parse("1157/snd_bgm_live_1157_preview_02"), None);
    }

    #[test]
    fn ignores_names_without_a_character_id() {
        assert_eq!(parse("snd_sfx_common"), None);
        assert_eq!(parse("snd_voi_abc_123"), None);
        assert_eq!(parse("snd_voi_xyz_9999"), None);
        assert_eq!(parse("snd_voi_"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn only_the_id_is_replaced() {
        let (id, range) = chara_id_in("snd_voi_live_111400").unwrap();
        assert_eq!(id, 1114);
        let mut out = "snd_voi_live_111400".to_owned();
        out.replace_range(range, "1030");
        assert_eq!(out, "snd_voi_live_103000");
    }
}
