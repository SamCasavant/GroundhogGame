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
}

#[derive(Copy, Clone, Eq)]
pub struct GameTime {
    raw: u32, // 0 = 12AM day 0; 108000 = 6AM day 1
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
        seconds: u32,
    ) {
        self.raw += seconds;
    }

    // Analog Clock Functions (floor output for digital):
    fn get_day(self) -> f32 { self.raw as f32 / 86400.0 }
    fn get_hour(self) -> f32 {
        // hours since new day
        (self.raw % 86400) as f32 / 3600.0
    }
    fn get_minute(self) -> f32 {
        // minutes since new hour
        (self.raw % 3600) as f32 / 60.0
    }
    fn get_second(self) -> f32 {
        // seconds since new minute
        (self.raw % 60) as f32
    }
    pub fn get_stamp(self) -> Stamp {
        Stamp {
            day:    self.get_day().floor() as u32,
            hour:   self.get_hour().floor() as u32,
            minute: self.get_minute().floor() as u32,
            second: self.get_second().floor() as u32,
        }
    }
    // Helpers
    #[allow(non_snake_case)]
    pub fn is_AM(self) -> bool { self.get_hour() < 12.0 }
    pub fn how_soon(
        self,
        other: Self,
    ) -> u32 {
        let seconds = other.raw.checked_sub(self.raw);
        seconds.unwrap_or(0)
    }
    pub fn copy_and_tick(
        self,
        seconds: u32,
    ) -> Self {
        // Produces a new time instance at a later point for scheduling relative
        // to self
        Self {
            raw: self.raw + seconds,
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

    let step = realtimer.0.elapsed().mul_f32(rate.0);
    let seconds = step.as_secs();
    localtime.tick(seconds as u32);

    let remainder = (step - Duration::new(seconds, 0)).div_f32(rate.0);
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
        }))
        .insert_resource(GameInWatch(Stopwatch::new()))
        .insert_resource(GameTimeRate(2.0))
        .add_system(advance_time.system().label("preparation"));
    }
}
