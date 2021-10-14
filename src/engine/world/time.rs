// TODO: Implement fixed timestep so systems can catch up to render

use std::cmp::Ordering;
use std::time::Duration;

use bevy::{core::Stopwatch, prelude::*};

#[derive(Debug)]
pub struct Stamp {
    pub day:    u32,
    pub hour:   u32,
    pub minute: u32,
    pub second: u32,
    pub frame:  u32, // Sixtieth of a second
}

#[derive(Copy, Clone, Eq)]
pub struct GameTime {
    raw: u32, /* Time in sixtieths of a second from 0 = 12AM day 0; 6480000
               * = 6AM day 1 */
}

impl GameTime {
    pub fn from_stamp(time: &Stamp) -> Self {
        let mut raw = 0;
        raw += time.day * 86400;
        raw += time.hour * 3600;
        raw += time.minute * 60;
        raw += time.second;
        Self { raw }
    }
    pub fn tick(
        &mut self,
        frames: u32,
    ) {
        self.raw += frames;
    }
    pub fn tick_seconds(
        &mut self,
        seconds: u32,
    ) {
        self.raw += 60 * seconds
    }

    // Analog Clock Functions (floor output for digital):
    fn get_day(self) -> f32 { self.raw as f32 / 5184000.0 }
    fn get_hour(self) -> f32 {
        // hours since new day
        (self.raw % 5184000) as f32 / 216000.0
    }
    fn get_minute(self) -> f32 {
        // minutes since new hour
        (self.raw % 216000) as f32 / 3600.0
    }
    fn get_second(self) -> f32 {
        // seconds since new minute
        (self.raw % 3600) as f32 / 60.0
    }
    fn get_frame(self) -> f32 { (self.raw % 60) as f32 }
    pub fn get_stamp(self) -> Stamp {
        Stamp {
            day:    self.get_day().floor() as u32,
            hour:   self.get_hour().floor() as u32,
            minute: self.get_minute().floor() as u32,
            second: self.get_second().floor() as u32,
            frame:  self.get_frame().floor() as u32,
        }
    }
    // Helpers
    #[allow(non_snake_case)]
    pub fn is_AM(self) -> bool { self.get_hour() < 12.0 }
    pub fn how_soon(
        self,
        other: Self,
    ) -> u32 {
        let frames = other.raw.checked_sub(self.raw);
        frames.unwrap_or(0) // Returns 0 when other.raw is the current time or
                            // earlier
    }
    pub fn copy_and_tick_seconds(
        self,
        seconds: u32,
    ) -> Self {
        // Produces a new time instance at a later point for scheduling relative
        // to self
        Self {
            raw: self.raw + 60 * seconds,
        }
    }
    pub fn copy_and_tick(
        self,
        frames: u32,
    ) -> Self {
        Self {
            raw: self.raw + frames,
        }
    }
}
impl Ord for GameTime {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl PartialOrd for GameTime {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for GameTime {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.raw == other.raw
    }
}

pub struct GameTimeRate(f32);

struct GameInWatch(Stopwatch);

fn advance_time(
    mut localtime: ResMut<GameTime>,
    realtime: Res<Time>,
    mut realtimer: ResMut<GameInWatch>,
    rate: Res<GameTimeRate>,
) {
    realtimer.0.tick(realtime.delta());

    let step = realtimer.0.elapsed().mul_f32(rate.0).mul_f32(60.0);
    let frames = step.as_secs();
    localtime.tick(frames as u32);

    let remainder = (step - Duration::new(frames, 0))
        .div_f32(rate.0)
        .div_f32(60.0);
    realtimer.0.set_elapsed(remainder);
}

use bevy::core::CoreSystem;

pub struct TimePlugin;
impl Plugin for TimePlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        app.insert_resource(GameTime::from_stamp(&Stamp {
            day:    0,
            hour:   6,
            minute: 0,
            second: 0,
            frame:  0,
        }))
        .insert_resource(GameInWatch(Stopwatch::new()))
        .insert_resource(GameTimeRate(1.5))
        .add_system_to_stage(
            CoreStage::First,
            advance_time.system().after(CoreSystem::Time),
        );
    }
}

// EXPERIMENTAL: Rewrite this when you understand it TODO FIXME
// use bevy::ecs::{schedule::ShouldRun,
// system::{Res, ResMut}};
//
// pub struct FixedUpdate;
// impl FixedUpdate {
// pub fn step(mut accumulator: ResMut<TimeAccumulator>) -> ShouldRun {
// if let Some(_) = accumulator.sub_step() {
// ShouldRun::YesAndCheckAgain
// } else {
// ShouldRun::No
// }
// }
// }
//
// #[derive(Debug, Clone)]
// pub struct TimeAccumulator {
// time:  Duration,
// steps: usize,
// }
//
// impl Default for TimeAccumulator {
// fn default() -> Self {
// Self {
// time:  Duration::from_secs(0),
// steps: 0,
// }
// }
// }
//
// impl TimeAccumulator {
// pub fn new(
// time: Duration,
// steps: usize,
// ) -> Self {
// Self { time, steps }
// }
//
// The number of accrued steps.
// #[inline]
// pub fn steps(&self) -> usize { self.steps }
//
// Add to the stored time, then convert into as many steps as possible.
// pub(crate) fn add_time(
// &mut self,
// time: Duration,
// timestep: Duration,
// ) {
// self.time += time;
// while self.time >= timestep {
// self.time -= timestep;
// self.steps += 1;
// }
// }
//
// pub(crate) fn sub_step(&mut self) -> Option<usize> {
// self.steps.checked_sub(1)
// }
// }
//
// add to CoreStage::First after CoreSystem::Time (time_system)
// pub(crate) fn time_accumulator_system(
// time: Res<GameTime>,
// mut accumulator: ResMut<TimeAccumulator>,
// ) {
// accumulator.add_time(
// time.delta().mul_f64(time.dilation_f64()),
// time.fixed_delta(),
// );
// }
