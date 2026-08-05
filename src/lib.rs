#![feature(proc_macro_hygiene)]

use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smashline::{Agent, Main};
use std::fs::{File, OpenOptions};
use std::io::Write;

const TARGET_COLOR_SLOT: i32 = 2;
const LOG_PATH: &str = "sd:/ultimate/bayonetta_body_debug.txt";

static mut LOG_FILE: Option<File> = None;
static mut FRAME_COUNTER: [u64; 8] = [0; 8];
static mut WAS_GUARD_ON: [bool; 8] = [false; 8];

unsafe fn entry_id(boma: *mut smash::app::BattleObjectModuleAccessor) -> Option<usize> {
    let id = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if (0..8).contains(&id) { Some(id as usize) } else { None }
}

unsafe fn log_line(line: &str, flush: bool) {
    if let Some(file) = LOG_FILE.as_mut() {
        let _ = writeln!(file, "{}", line);
        if flush {
            let _ = file.flush();
        }
    }
}

unsafe extern "C" fn bayonetta_debug_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let boma = fighter.module_accessor;
        let Some(id) = entry_id(boma) else { return; };
        if WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR) != TARGET_COLOR_SLOT {
            return;
        }

        FRAME_COUNTER[id] += 1;
        let frame_id = FRAME_COUNTER[id];
        let status = StatusModule::status_kind(boma);
        let motion = MotionModule::motion_kind(boma);
        let motion_frame = MotionModule::frame(boma);
        let motion_end = MotionModule::end_frame(boma);
        let motion_changing = MotionModule::is_changing(boma);
        let shooting_action = WorkModule::is_flag(
            boma,
            *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_FLAG_SHOOTING_ACTION,
        );
        let shooting_keep = WorkModule::is_flag(
            boma,
            *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_FLAG_SHOOTING_KEEP,
        );
        let shooting_check_end = WorkModule::is_flag(
            boma,
            *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_FLAG_SHOOTING_CHECK_END,
        );
        let shooting_motion_stop = WorkModule::is_flag(
            boma,
            *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_FLAG_SHOOTING_MOTION_STOP,
        );
        let shooting_step = WorkModule::get_int(
            boma,
            *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_INT_SHOOTING_STEP,
        );
        let shooting_frame = WorkModule::get_int(
            boma,
            *FIGHTER_BAYONETTA_INSTANCE_WORK_ID_INT_SHOOTING_FRAME,
        );
        let guard_on = ControlModule::check_button_on(boma, *CONTROL_PAD_BUTTON_GUARD);
        let guard_marker = guard_on && !WAS_GUARD_ON[id];
        WAS_GUARD_ON[id] = guard_on;

        let line = format!(
            "f={frame_id} entry={id} status={status} motion=0x{motion:016x} motion_frame={motion_frame:.2} motion_end={motion_end:.2} changing={} shoot_action={} shoot_keep={} shoot_check_end={} shoot_stop={} shoot_step={} shoot_frame={} marker_guard={}",
            motion_changing as u8,
            shooting_action as u8,
            shooting_keep as u8,
            shooting_check_end as u8,
            shooting_motion_stop as u8,
            shooting_step,
            shooting_frame,
            guard_marker as u8,
        );
        log_line(&line, guard_marker || frame_id % 60 == 0);
    }
}

#[skyline::main(name = "bayonetta_body_glow_debug")]
pub fn main() {
    unsafe {
        LOG_FILE = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(LOG_PATH)
            .ok();
        log_line("bayonetta body transition diagnostic v1", true);
    }

    Agent::new("bayonetta")
        .on_line(Main, bayonetta_debug_frame)
        .install();
}
