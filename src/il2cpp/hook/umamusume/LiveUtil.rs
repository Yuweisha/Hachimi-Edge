use crate::{
    core::Hachimi,
    il2cpp::{symbols::get_method_addr, types::*}
};

type GetSingCharaIdListFn = extern "C" fn(songId: i32, songPartNumber: i32, allCharaIdArray: *mut Il2CppArray, vocalCharaIdArray: *mut Il2CppArray, shuffledCharaDataList: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn GetSingCharaIdList(songId: i32, songPartNumber: i32, allCharaIdArray: *mut Il2CppArray, vocalCharaIdArray: *mut Il2CppArray, shuffledCharaDataList: *mut Il2CppObject) -> *mut Il2CppObject {
    let config = Hachimi::instance().config.load();
    let chara_vo_ids = &config.live_vocals_swap;
    let replace = &config.replace_global_char;
    let force = replace.song_force_chara;

    if songId > 0 {
        unsafe {
            for (array, is_vocal) in [(vocalCharaIdArray, true), (allCharaIdArray, false)] {
                if array.is_null() {
                    continue;
                }

                let len = (*array).max_length as usize;
                let data_ptr = array.add(1) as *mut i32;

                for i in 0..len {
                    let orig = if i < chara_vo_ids.len() { *data_ptr.add(i) } else { 0 };
                    let mut new_id = orig;

                    if i < chara_vo_ids.len() && chara_vo_ids[i] != 0 {
                        new_id = chara_vo_ids[i];
                    } else if force != 0 {
                        new_id = force;
                    } else if replace.enable && orig > 0 {
                        new_id = crate::core::voice_replace::effective_char_id(orig);
                    }

                    if new_id != orig {
                        *data_ptr.add(i) = new_id;
                        if replace.log_audio_cues && is_vocal {
                            debug!("[song] 演唱者 {}: {} -> {}", i, orig, new_id);
                        }
                    }
                }
            }
        }
    }

    get_orig_fn!(GetSingCharaIdList, GetSingCharaIdListFn)(songId, songPartNumber, allCharaIdArray, vocalCharaIdArray, shuffledCharaDataList)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", LiveUtil);

    let GetSingCharaIdList_addr = get_method_addr(LiveUtil, c"GetSingCharaIdList", 5);
    new_hook!(GetSingCharaIdList_addr, GetSingCharaIdList);
}

