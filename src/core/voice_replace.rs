
use std::ffi::c_void;
use std::ops::Range;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::core::Hachimi;
use crate::il2cpp::ext::{Il2CppStringExt, StringExt};
use crate::il2cpp::hook::umamusume::MasterCharacterSystemText::{self, CharacterSystemText};
use crate::il2cpp::symbols::IList;
use crate::il2cpp::types::{Il2CppObject, Il2CppString};

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

pub fn text_for(chara_id: i32, voice_id: i32) -> Option<*mut Il2CppString> {
    let list = MasterCharacterSystemText::GetByCharaId(chara_id);
    if list.is_null() {
        return None;
    }

    let ilist = IList::<*mut Il2CppObject>::new(list)?;
    for item in ilist.iter() {
        if item.is_null() {
            continue;
        }
        if CharacterSystemText::get_VoiceId(item) != voice_id {
            continue;
        }

        let text = CharacterSystemText::get_Text(item);
        if !text.is_null() {
            return Some(text);
        }
    }

    None
}

fn chara_id_in(sheet: &str) -> Option<(i32, Range<usize>)> {
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
