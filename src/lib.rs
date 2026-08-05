#![feature(proc_macro_hygiene)]

use smash::app::lua_bind::*;
use smash::hash40;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Hash40;
use smash_script::macros;
use smashline::{Agent, Main};

const TARGET_COLOR_SLOT: i32 = 2;
// Runtime state survives motion cancels. This is the first calibration value;
// it can be adjusted after an in-game timing test.
const TRANSFORM_FRAMES: i32 = 90;
const END_GLOW_FRAMES: i32 = 12;

static mut TRANSFORM_TIMER: [i32; 8] = [0; 8];
static mut WAS_TRIGGER_MOTION: [bool; 8] = [false; 8];
static mut END_GLOW_STARTED: [bool; 8] = [false; 8];

unsafe fn entry_id(boma: *mut smash::app::BattleObjectModuleAccessor) -> Option<usize> {
    let id = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if (0..8).contains(&id) { Some(id as usize) } else { None }
}

unsafe fn is_transform_motion(boma: *mut smash::app::BattleObjectModuleAccessor) -> bool {
    let motion = MotionModule::motion_kind(boma);
    motion == hash40("attack_s4_s")
        || motion == hash40("attack_s4_hi")
        || motion == hash40("attack_s4_lw")
        || motion == hash40("attack_hi4")
        || motion == hash40("attack_lw4")
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
    // Three overlapping body lights create the body-covering bloom requested.
    macros::FLASH(fighter, 16.0, 16.0, 20.0, 1.0);
    aura_at(fighter, 3.0, 2.7);
    aura_at(fighter, 7.5, 3.1);
    aura_at(fighter, 12.0, 2.7);
}

unsafe fn reset(fighter: &mut L2CFighterCommon, id: usize) {
    TRANSFORM_TIMER[id] = 0;
    WAS_TRIGGER_MOTION[id] = false;
    END_GLOW_STARTED[id] = false;
    kill_glow(fighter);
}

unsafe extern "C" fn bayonetta_body_glow_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let boma = fighter.module_accessor;
        let Some(id) = entry_id(boma) else { return; };

        if WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR) != TARGET_COLOR_SLOT {
            if TRANSFORM_TIMER[id] > 0 {
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

        let trigger = is_transform_motion(boma);
        if trigger && !WAS_TRIGGER_MOTION[id] {
            kill_glow(fighter);
            TRANSFORM_TIMER[id] = TRANSFORM_FRAMES;
            END_GLOW_STARTED[id] = false;
        }
        WAS_TRIGGER_MOTION[id] = trigger;

        if TRANSFORM_TIMER[id] > 0 {
            if TRANSFORM_TIMER[id] == END_GLOW_FRAMES && !END_GLOW_STARTED[id] {
                END_GLOW_STARTED[id] = true;
                ending_glow(fighter);
            }

            TRANSFORM_TIMER[id] -= 1;
            if TRANSFORM_TIMER[id] == 0 {
                kill_glow(fighter);
                END_GLOW_STARTED[id] = false;
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
