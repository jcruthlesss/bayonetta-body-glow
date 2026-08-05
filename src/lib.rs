#![feature(proc_macro_hygiene)]

use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Hash40;
use smash_script::macros;
use smashline::{Agent, Main};

const TARGET_COLOR_SLOT: i32 = 2;
// The diagnostic log placed the visible swap 78-96 frames after Bayonetta's
// shooting state cleared. Start just before the earliest observed switch and
// keep the bloom through the complete measured window.
const POST_SHOOT_DELAY: i32 = 70;
const END_GLOW_FRAMES: i32 = 26;

static mut SMASH_ACTIVE: [bool; 8] = [false; 8];
static mut SAW_SHOOTING: [bool; 8] = [false; 8];
static mut POST_SHOOT_TIMER: [i32; 8] = [-1; 8];
static mut END_GLOW_TIMER: [i32; 8] = [0; 8];

unsafe fn entry_id(boma: *mut smash::app::BattleObjectModuleAccessor) -> Option<usize> {
    let id = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if (0..8).contains(&id) { Some(id as usize) } else { None }
}

unsafe fn is_body_transform_smash_status(status: i32) -> bool {
    status == *FIGHTER_STATUS_KIND_ATTACK_S4
        || status == *FIGHTER_STATUS_KIND_ATTACK_HI4
        || status == *FIGHTER_STATUS_KIND_ATTACK_LW4
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
        }

        if SMASH_ACTIVE[id] {
            if shooting {
                SAW_SHOOTING[id] = true;
            } else if SAW_SHOOTING[id] && POST_SHOOT_TIMER[id] < 0 {
                // This is the repeatable falling edge seen in every logged run.
                POST_SHOOT_TIMER[id] = POST_SHOOT_DELAY;
            } else if !in_smash && POST_SHOOT_TIMER[id] < 0 {
                // Fallback for an unusually early cancel before gun state starts.
                POST_SHOOT_TIMER[id] = POST_SHOOT_DELAY;
            }
        }

        if POST_SHOOT_TIMER[id] >= 0 {
            if POST_SHOOT_TIMER[id] == 0 {
                ending_glow(fighter);
                END_GLOW_TIMER[id] = END_GLOW_FRAMES;
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
            }
        }
    }
}

#[skyline::main(name = "bayonetta_body_glow")]
pub fn main() {
    Agent::new("bayonetta")
        .on_line(Main, bayonetta_body_glow_frame)
        .install();
}
