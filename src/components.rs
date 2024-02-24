use bevy::math::{Vec2, Vec3};
use bevy::prelude::Component;
use bevy::sprite::SpriteSheetBundle;
use bevy::time::{Timer, TimerMode};
use bevy::transform::components::Transform;

// region:    --- Common Components
#[derive(Component)]
pub struct CameraMarker;

#[derive(Component)]
pub struct Velocity {
	pub x: f32,
	pub y: f32,
	pub omega: f32
}

#[derive(Component)]
pub struct Movable {
	pub auto_despawn: bool,
}

#[derive(Component)]
pub struct SpriteSize(pub Vec2);

impl From<(f32, f32)> for SpriteSize {
	fn from(val: (f32, f32)) -> Self {
		SpriteSize(Vec2::new(val.0, val.1))
	}
}

// endregion: --- Common Components

// region:    --- Map Components

#[derive(Component)]
pub struct Tile {
	pub x: i32,
	pub y: i32
}

#[derive(Component)]
pub struct Star;

// endrefion: --- Map Components

#[derive(Component)]
pub struct Ship {
	pub max_shields: f32,
	pub current_shields: f32,
	pub sheild_carge_rate: f32
}

#[derive(Component)]
pub struct Shield;


#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Component)]
pub enum Allegiance {
    Friendly,
    Enemy,
}

impl Default for Allegiance {
    fn default() -> Self {
        Allegiance::Friendly
    }
}

// region:    --- Player Components
#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct FromPlayer;
// endregion: --- Player Components

// region:    --- Enemy Components
#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct FromEnemy;
// endregion: --- Enemy Components

// region:	--- Weapon Components
#[derive(Component)]
pub struct Weapon {
	pub current_charge: f32,
	pub charge_rate: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Component)]
pub enum WeaponType {
    Laser,
    EMP,
}

impl Default for WeaponType {
    fn default() -> Self {
        WeaponType::Laser
    }
}

#[derive(Component)]
pub struct Laser;

#[derive(Component)]
pub struct EMP;


#[derive(Component)]
pub struct EMPAnimator{
	pub timer: Timer,
	pub index: usize
}

impl EMPAnimator {
	pub fn new(duration: f32) -> Self {
		Self {
			timer: Timer::from_seconds(duration, TimerMode::Repeating),
			index: 0
		}
	}
}


// endregion: --- Weapon Components

// region:    --- Explosion Components
#[derive(Component)]
pub struct Explosion;

#[derive(Component)]
pub struct ExplosionToSpawn {
	pub transform: Transform,
	pub duration: f32,
	pub is_engine: bool,
}

#[derive(Component)]
pub struct ExplosionTimer(pub Timer);

impl ExplosionTimer {
	pub fn new(duration: f32) -> Self {
		Self(Timer::from_seconds(duration, TimerMode::Repeating))
	}
}

impl Default for ExplosionTimer {
	fn default() -> Self {
		Self(Timer::from_seconds(0.01, TimerMode::Repeating))
	}
}
// endregion: --- Explosion Components

//region: HUD Components
#[derive(Component)]
pub struct WeaponChargeBar;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct CodePilotActiveText;

#[derive(Component)]
pub struct MaxScoreText;

