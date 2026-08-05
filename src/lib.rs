#![feature(proc_macro_hygiene)]

use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Hash40;
use smash_script::macros;
use smashline::{Agent, Main};

const TARGET_COLOR_SLOT: i32 = 2;
// Calibrated from the automatic status exits in the diagnostic log rather
// than the later manual shield markers. Each starts shortly before its model
// transition; up smash receives a longer bloom.
const SIDE_SMASH_DELAY: i32 = 40;
const UP_SMASH_DELAY: i32 = 46;
const DOWN_SMASH_DELAY: i32 = 32;
const STANDARD_GLOW_FRAMES: i32 = 8;
const UP_GLOW_FRAMES: i32 = 10;
const CLEANUP_FRAMES: i32 = 6;

static mut SMASH_ACTIVE: [bool; 8] = [false; 8];
static mut SAW_SHOOTING: [bool; 8] = [false; 8];
static mut POST_SHOOT_TIMER: [i32; 8] = [-1; 8];
static mut END_GLOW_TIMER: [i32; 8] = [0; 8];
static mut CLEANUP_TIMER: [i32; 8] = [0; 8];
static mut ACTIVE_SMASH_STATUS: [i32; 8] = [0; 8];

unsafe fn entry_id(boma: *mut smash::app::BattleObjectModuleAccessor) -> Option<usize> {
    let id = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if (0..8).contains(&id) { Some(id as usize) } else { None }
}

unsafe fn is_body_transform_smash_status(status: i32) -> bool {
    status == *FIGHTER_STATUS_KIND_ATTACK_S4
        || status == *FIGHTER_STATUS_KIND_ATTACK_HI4
        || status == *FIGHTER_STATUS_KIND_ATTACK_LW4
}

unsafe fn delay_for_status(status: i32) -> i32 {
    if status == *FIGHTER_STATUS_KIND_ATTACK_HI4 {
        UP_SMASH_DELAY
    } else if status == *FIGHTER_STATUS_KIND_ATTACK_LW4 {
        DOWN_SMASH_DELAY
    } else {
        SIDE_SMASH_DELAY
    }
}

unsafe fn glow_frames_for_status(status: i32) -> i32 {
    if status == *FIGHTER_STATUS_KIND_ATTACK_HI4 {
        UP_GLOW_FRAMES
    } else {
        STANDARD_GLOW_FRAMES
    }
}

unsafe fn shooting_state_active(boma: *mut smash::app::BattleObjectModuleAccessor) -> bool {
    let step = WorkModule::get_int(
        boma,
        *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_INT_SHOOTING_STEP,
    );
    WorkModule::is_flag(
        boma,
        *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_FLAG_SHOOTING_ACTION,
    ) || WorkModule::is_flag(
        boma,
        *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_FLAG_SHOOTING_CHECK_END,
    ) || WorkModule::is_flag(
        boma,
        *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_FLAG_SHOOTING_MOTION_STOP,
    ) || step != *FIGHTER_BAYONETTA_SHOOTING_STEP_WAIT
}

unsafe fn kill_glow(fighter: &mut L2CFighterCommon) {
    macros::COL_NORMAL(fighter);
    EffectModule::kill_kind(
        fighter.module_accessor,
        Hash40::new("sys_aura_light"),
        true,
        true,
    );
}

unsafe fn aura_at(fighter: &mut L2CFighterCommon, y: f32, scale: f32) {
    macros::EFFECT_FOLLOW(
        fighter,
        Hash40::new("sys_aura_light"),
        Hash40::new("top"),
        0.0, y, 0.0,
        0.0, 0.0, 0.0,
        scale,
        true,
    );
    macros::LAST_EFFECT_SET_COLOR(fighter, 1.0, 0.82, 1.0);
}

unsafe fn ending_glow(fighter: &mut L2CFighterCommon) {
    macros::FLASH(fighter, 16.0, 16.0, 20.0, 1.0);
    aura_at(fighter, 3.0, 2.7);
    aura_at(fighter, 7.5, 3.1);
    aura_at(fighter, 12.0, 2.7);
}

unsafe fn reset(fighter: &mut L2CFighterCommon, id: usize) {
    SMASH_ACTIVE[id] = false;
    SAW_SHOOTING[id] = false;
    POST_SHOOT_TIMER[id] = -1;
    END_GLOW_TIMER[id] = 0;
    CLEANUP_TIMER[id] = 0;
    ACTIVE_SMASH_STATUS[id] = 0;
    kill_glow(fighter);
}

unsafe extern "C" fn bayonetta_body_glow_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let boma = fighter.module_accessor;
        let Some(id) = entry_id(boma) else { return; };

        if WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR) != TARGET_COLOR_SLOT {
            if SMASH_ACTIVE[id] || POST_SHOOT_TIMER[id] >= 0 || END_GLOW_TIMER[id] > 0 {
                reset(fighter, id);
            }
            return;
        }

        let status = StatusModule::status_kind(boma);
        if status == *FIGHTER_STATUS_KIND_ENTRY
            || status == *FIGHTER_STATUS_KIND_REBIRTH
            || status == *FIGHTER_STATUS_KIND_DEAD
        {
            reset(fighter, id);
            return;
        }

        let in_smash = is_body_transform_smash_status(status);
        let shooting = shooting_state_active(boma);

        if in_smash && !SMASH_ACTIVE[id] {
            kill_glow(fighter);
            SMASH_ACTIVE[id] = true;
            SAW_SHOOTING[id] = false;
            POST_SHOOT_TIMER[id] = -1;
            END_GLOW_TIMER[id] = 0;
            CLEANUP_TIMER[id] = 0;
            ACTIVE_SMASH_STATUS[id] = status;
        }

        if SMASH_ACTIVE[id] {
            if shooting {
                SAW_SHOOTING[id] = true;
            } else if SAW_SHOOTING[id] && POST_SHOOT_TIMER[id] < 0 {
                // This is the repeatable falling edge seen in every logged run.
                POST_SHOOT_TIMER[id] = delay_for_status(ACTIVE_SMASH_STATUS[id]);
            } else if !in_smash && POST_SHOOT_TIMER[id] < 0 {
                // Fallback for an unusually early cancel before gun state starts.
                POST_SHOOT_TIMER[id] = delay_for_status(ACTIVE_SMASH_STATUS[id]);
            }
        }

        if POST_SHOOT_TIMER[id] >= 0 {
            if POST_SHOOT_TIMER[id] == 0 {
                ending_glow(fighter);
                END_GLOW_TIMER[id] = glow_frames_for_status(ACTIVE_SMASH_STATUS[id]);
                POST_SHOOT_TIMER[id] = -1;
                SMASH_ACTIVE[id] = false;
                SAW_SHOOTING[id] = false;
            } else {
                POST_SHOOT_TIMER[id] -= 1;
            }
        }

        if END_GLOW_TIMER[id] > 0 {
            END_GLOW_TIMER[id] -= 1;
            if END_GLOW_TIMER[id] == 0 {
                kill_glow(fighter);
                CLEANUP_TIMER[id] = CLEANUP_FRAMES;
            }
        }

        // Some effect instances can finish their current particle update after
        // the first kill request. Repeat cleanup briefly to prevent any aura
        // from lingering on body_norm.
        if CLEANUP_TIMER[id] > 0 {
            kill_glow(fighter);
            CLEANUP_TIMER[id] -= 1;
        }
    }
}

#[skyline::main(name = "bayonetta_body_glow")]
pub fn main() {
    Agent::new("bayonetta")
        .on_line(Main, bayonetta_body_glow_frame)
        .install();
}
