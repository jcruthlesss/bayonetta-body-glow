#![feature(proc_macro_hygiene)]

use smash::app::lua_bind::*;
use smash::hash40;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Hash40;
use smash_script::macros;
use smashline::*;

// This build is intentionally restricted to Bayonetta costume slot c02.
const TARGET_COLOR_SLOT: i32 = 2;

// Number of frames body_anim is held after the game requests body_normal.
// Raise this for a longer ending flash; lower it for a snappier transition.
const END_HOLD_FRAMES: i32 = 8;

// Per-player state. Ultimate supports up to eight fighter entry IDs.
static mut WAS_BODY_ANIM: [bool; 8] = [false; 8];
static mut ENDING_FRAMES: [i32; 8] = [0; 8];

unsafe fn entry_id(boma: &mut smash::app::BattleObjectModuleAccessor) -> Option<usize> {
    let id = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if (0..8).contains(&id) {
        Some(id as usize)
    } else {
        None
    }
}

unsafe fn set_body_anim(boma: &mut smash::app::BattleObjectModuleAccessor) {
    VisibilityModule::set_int64(
        boma,
        hash40("body") as i64,
        hash40("body_anim") as i64,
    );
}

unsafe fn set_body_normal(boma: &mut smash::app::BattleObjectModuleAccessor) {
    VisibilityModule::set_int64(
        boma,
        hash40("body") as i64,
        hash40("body_normal") as i64,
    );
}

unsafe fn current_body_is_anim(
    boma: &mut smash::app::BattleObjectModuleAccessor,
) -> bool {
    VisibilityModule::get_int64(boma, hash40("body") as i64)
        == hash40("body_anim") as i64
}

unsafe fn clear_visuals(fighter: &mut L2CFighterCommon) {
    macros::COL_NORMAL(fighter);
    EffectModule::kill_kind(
        fighter.module_accessor,
        Hash40::new("sys_aura_light"),
        true,
        true,
    );
}

unsafe fn opening_flash(fighter: &mut L2CFighterCommon) {
    // The first call is immediate. FLASH_FRM interpolates it back to clear
    // over four frames, so switching motions cannot restart the flash.
    macros::FLASH(fighter, 5.0, 5.0, 6.0, 1.0);
    macros::FLASH_FRM(fighter, 4, 1.0, 1.0, 1.0, 0.0);
}

unsafe fn ending_flash(fighter: &mut L2CFighterCommon) {
    // Values above 1.0 deliberately push the fighter into strong bloom.
    macros::FLASH(fighter, 16.0, 16.0, 20.0, 1.0);
    macros::EFFECT_FOLLOW(
        fighter,
        Hash40::new("sys_aura_light"),
        Hash40::new("top"),
        0.0,
        10.0,
        0.0,
        0.0,
        0.0,
        0.0,
        2.6,
        true,
    );
    macros::LAST_EFFECT_SET_COLOR(fighter, 1.0, 0.85, 1.0);
}

unsafe fn reset_player(fighter: &mut L2CFighterCommon, id: usize) {
    WAS_BODY_ANIM[id] = false;
    ENDING_FRAMES[id] = 0;
    clear_visuals(fighter);
}

#[fighter_frame(agent = FIGHTER_KIND_BAYONETTA)]
fn bayonetta_body_glow_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let boma = fighter.module_accessor;
        let Some(id) = entry_id(boma) else {
            return;
        };

        // Do nothing to every other costume slot.
        if WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR)
            != TARGET_COLOR_SLOT
        {
            if WAS_BODY_ANIM[id] || ENDING_FRAMES[id] > 0 {
                reset_player(fighter, id);
            }
            return;
        }

        let status = StatusModule::status_kind(boma);
        if status == *FIGHTER_STATUS_KIND_ENTRY
            || status == *FIGHTER_STATUS_KIND_REBIRTH
            || status == *FIGHTER_STATUS_KIND_DEAD
        {
            reset_player(fighter, id);
            return;
        }

        // While finishing, override any repeated body_normal request made by
        // the game. This keeps the transformed mesh alive under the bloom.
        if ENDING_FRAMES[id] > 0 {
            set_body_anim(boma);
            ENDING_FRAMES[id] -= 1;

            if ENDING_FRAMES[id] == 0 {
                set_body_normal(boma);
                clear_visuals(fighter);
                WAS_BODY_ANIM[id] = false;
            }
            return;
        }

        let is_anim = current_body_is_anim(boma);

        // Rising edge: body_normal -> body_anim.
        if is_anim && !WAS_BODY_ANIM[id] {
            opening_flash(fighter);
            WAS_BODY_ANIM[id] = true;
            return;
        }

        // Falling edge: the game has requested body_normal. Restore body_anim
        // for a few frames, cover it in bloom, then complete the switch.
        if !is_anim && WAS_BODY_ANIM[id] {
            set_body_anim(boma);
            ENDING_FRAMES[id] = END_HOLD_FRAMES;
            ending_flash(fighter);
            return;
        }

        WAS_BODY_ANIM[id] = is_anim;
    }
}

#[skyline::main(name = "bayonetta_body_glow")]
pub fn main() {
    install_agent_frame_callbacks!(bayonetta_body_glow_frame);
}
