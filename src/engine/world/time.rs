// TODO: Implement fixed timestep so systems can catch up to render

use std::cmp::Ordering;
use std::fmt;
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
    pub const fn from_stamp(time: &Stamp) -> Self {
        let mut raw = 0;
        raw += time.day * 86_400;
        raw += time.hour * 3_600;
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
        self.raw += 60 * seconds;
    }

    // Analog Clock Functions (floor output for digital):
    fn get_day(self) -> f64 { f64::from(self.raw) / 5_184_000.0 }
    fn get_hour(self) -> f64 {
        // hours since new day
        f64::from(self.raw % 5_184_000) / 216_000.0
    }
    fn get_minute(self) -> f64 {
        // minutes since new hour
        f64::from(self.raw % 216_000) / 3_600.0
    }
    fn get_second(self) -> f64 {
        // seconds since new minute
        f64::from(self.raw % 3_600) / 60.0
    }
    const fn get_frame(self) -> f64 { (self.raw % 60) as f64 }
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
    pub const fn copy_and_tick(
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
impl fmt::Debug for GameTime {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("GameTime")
            .field("day", &self.get_day().floor())
            .field("hour", &self.get_hour().floor())
            .field("minute", &self.get_minute().floor())
            .field("second", &self.get_second().floor())
            .field("frame", &self.get_frame())
            .finish()
    }
}

pub struct GameTimeRate(f64);

struct GameInWatch(Stopwatch);

fn advance_time(
    mut localtime: ResMut<GameTime>,
    realtime: Res<Time>,
    mut realtimer: ResMut<GameInWatch>,
    rate: Res<GameTimeRate>,
) {
    realtimer.0.tick(realtime.delta());

    let step = realtimer.0.elapsed().mul_f64(rate.0).mul_f64(60.0);
    let frames = step.as_secs();
    localtime.tick(frames.try_into().ok().unwrap()); // Panics on overflow, which *should* never happen but should cause panic

    let remainder = (step - Duration::new(frames, 0))
        .div_f64(rate.0)
        .div_f64(60.0);
    realtimer.0.set_elapsed(remainder);
}

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
        .insert_resource(GameTimeRate(5.0))
        .add_system_to_stage(CoreStage::First, advance_time.system());
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
