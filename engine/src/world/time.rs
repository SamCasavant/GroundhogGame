use std::time::Duration;

use bevy::{core::Stopwatch, prelude::*};

#[derive(Debug)]
pub struct Stamp{
    day: u32,
    hour: u32,
    minute: u32,
    second: u32
}

#[derive(Copy, Clone)]
pub struct GameTime{
    raw: u32, // 0 = 12AM day 0; 108000 = 6AM day 1
}

impl GameTime {
    pub fn from_stamp(time: Stamp) -> GameTime{
        let mut raw = 0;
        raw += time.day * 86400;
        raw += time.hour * 3600;
        raw += time.minute * 60;
        raw += time.second;
        GameTime{ raw: raw }
    }
    pub fn tick(&mut self, seconds: u32){
        self.raw += seconds;
    }
    //Analog Clock Functions (floor output for digital):
    fn get_day(&self) -> f32{
        self.raw as f32 / 86400.0
    }
    fn get_hour(&self) -> f32{ //hours since new day
        (self.raw % 86400) as f32 / 3600.0
    }
    fn get_minute(&self) -> f32{ //minutes since new hour
        (self.raw % 3600) as f32 / 60.0
    }
    fn get_second(&self) -> f32{ //seconds since new minute
        (self.raw % 60) as f32
    }
    pub fn get_stamp(&self) -> Stamp{
        Stamp{
        day: self.get_day().floor() as u32,
        hour: self.get_hour().floor() as u32,
        minute: self.get_minute().floor() as u32,
        second: self.get_second().floor() as u32
        }
    }
    //Helpers
    #[allow(non_snake_case)]
    pub fn is_AM(&self) -> bool{
        self.get_hour() < 12.0
    }
    pub fn how_soon(&self, other: Self) -> u32{
        let seconds = other.raw.checked_sub(self.raw);
        match seconds {
            Some(n) => return n,
            None => return 0
        }
    }
}

pub struct GameTimeRate(f32);

struct GameInWatch(Stopwatch);

fn advance_time(mut localtime: ResMut<GameTime>, realtime: Res<Time>, mut realtimer: ResMut<GameInWatch>, rate: Res<GameTimeRate>){
    realtimer.0.tick(realtime.delta());

    let step = realtimer.0.elapsed().mul_f32(rate.0);//Duration::from_nanos((realtimer.0.elapsed().as_nanos() as f32 * rate.0) as u64); //TODO: This just sucks. Figure it out.
    let seconds = step.as_secs();
    localtime.tick(seconds as u32);

    if seconds > 0{
        println!("Timer: {:?}", localtime.get_stamp());
    }

    let remainder = (step - Duration::new(seconds, 0)).div_f32(rate.0);
    realtimer.0.set_elapsed(remainder);
}

pub struct TimePlugin;
impl Plugin for TimePlugin {
    fn build(&self, app: &mut AppBuilder){
        app
        .insert_resource(GameTime::from_stamp(Stamp{day: 0, hour: 6, minute: 0, second: 0}))
        .insert_resource(GameInWatch(Stopwatch::new()))
        .insert_resource(GameTimeRate(0.5))
        .add_system(advance_time.system());
    }
}