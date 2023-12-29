use bevy::math::{Vec2, Vec3};
use bevy::prelude::Component;
use bevy::sprite::SpriteSheetBundle;
use bevy::time::{Timer, TimerMode};
use bevy::transform::components::Transform;

// region:    --- Common Components
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
pub struct Laser;

#[derive(Component)]
pub struct SpriteSize(pub Vec2);

impl From<(f32, f32)> for SpriteSize {
	fn from(val: (f32, f32)) -> Self {
		SpriteSize(Vec2::new(val.0, val.1))
	}
}

// endregion: --- Common Components

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
