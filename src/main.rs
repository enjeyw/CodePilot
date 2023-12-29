#![allow(unused)] // silence unused warnings while exploring (to comment out)

use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use bevy::window::PrimaryWindow;
use components::{
	Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, Movable,
	Player, SpriteSize, Velocity,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use rustpython_vm as vm;
use vm::{builtins::PyCode, PyRef};
use std::sync::atomic::{AtomicBool, Ordering};

use enemy::EnemyPlugin;
use player::PlayerPlugin;
use std::{collections::HashSet, f32::consts::PI};

mod components;
mod enemy;
mod player;

// region:    --- Asset Constants

const PLAYER_SPRITE: &str = "lighter_nose.png";
const PLAYER_SIZE: (f32, f32) = (144., 75.);
const PLAYER_LASER_SPRITE: &str = "laser_a_01.png";
const PLAYER_LASER_SIZE: (f32, f32) = (9., 54.);

const ENEMY_SPRITE: &str = "organic_enemy.png";
const ENEMY_SIZE: (f32, f32) = (144., 75.);
const ENEMY_LASER_SPRITE: &str = "laser_b_01.png";
const ENEMY_LASER_SIZE: (f32, f32) = (17., 55.);

const EXPLOSION_SHEET: &str = "explo_a_sheet.png";
const EXPLOSION_ENGINE_SHEET: &str = "explo_b_sheet.png";
const EXPLOSION_LEN: usize = 16;

const SPRITE_SCALE: f32 = 0.5;

// endregion: --- Asset Constants

// region:    --- Game Constants

const BASE_SPEED: f32 = 200.;
const BASE_ROT_SPEED: f32 = 10.;


const PLAYER_RESPAWN_DELAY: f64 = 2.;
const ENEMY_MAX: u32 = 2;
const FORMATION_MEMBERS_MAX: u32 = 2;

// endregion: --- Game Constants

// region:    --- Resources
#[derive(Resource)]
pub struct WinSize {
	pub w: f32,
	pub h: f32,
}

#[derive(Resource)]
struct GameTextures {
	player: Handle<Image>,
	player_laser: Handle<Image>,
	enemy: Handle<Image>,
	enemy_laser: Handle<Image>,
	explosion: Handle<TextureAtlas>,
	engine: Handle<TextureAtlas>
}

#[derive(Default, Resource)]
pub struct UiState {
    player_code: String,
}

#[derive(Resource)]
pub struct CodePilotCode {
    compiled: Option<PyRef<PyCode>>,
}
impl Default for CodePilotCode {
	fn default() -> Self {
		Self {
			compiled: None
		}
	}
}

#[derive(Resource)]
struct EnemyCount(u32);

#[derive(Resource)]
struct PlayerState {
	on: bool,       // alive
	last_shot: f64, // -1 if not shot
}
impl Default for PlayerState {
	fn default() -> Self {
		Self {
			on: false,
			last_shot: -1.,
		}
	}
}

impl PlayerState {
	pub fn shot(&mut self, time: f64) {
		self.on = false;
		self.last_shot = time;
	}
	pub fn spawned(&mut self) {
		self.on = true;
		self.last_shot = -1.;
	}
}

// endregion: --- Resources

fn main() {
	App::new()
		.insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
		.init_resource::<UiState>()
		.init_resource::<CodePilotCode>()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Codepilot".into(),
				resolution: (1400., 800.).into(),
				..Default::default()
			}),
			..Default::default()
		}))
		.add_plugins(EguiPlugin)
		.add_plugins(PlayerPlugin)
		.add_plugins(EnemyPlugin)
		.add_systems(Startup, setup_system)
			.add_systems(Update, movable_system)
		.add_systems(Update, player_laser_hit_enemy_system)
		.add_systems(Update, enemy_laser_hit_player_system)
		.add_systems(Update, explosion_to_spawn_system)
		.add_systems(Update, explosion_animation_system)
		.add_systems(Update, ui_example_system)
		.run();
}

fn ui_example_system(
	mut ui_state: ResMut<UiState>,
	mut contexts: EguiContexts) {
	let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
			ui.label("Write something: ");
			ui.code_editor(&mut ui_state.player_code);
		});
    });
}

fn setup_system(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
	query: Query<&Window, With<PrimaryWindow>>,
) {
	// camera
	commands.spawn(Camera2dBundle::default());

	// capture window size
	let Ok(primary) = query.get_single() else {
		return;
	};
	let (win_w, win_h) = (primary.width(), primary.height());

	// position window (for tutorial)
	// window.set_position(IVec2::new(2780, 4900));

	// add WinSize resource
	let win_size = WinSize { w: win_w, h: win_h };
	commands.insert_resource(win_size);

	// create explosion texture atlas
	let expl_texture_handle = asset_server.load(EXPLOSION_SHEET);
	let expl_texture_atlas =
		TextureAtlas::from_grid(expl_texture_handle, Vec2::new(64., 64.), 4, 4, None, None);
	let explosion = texture_atlases.add(expl_texture_atlas);

	let eng_texture_handle = asset_server.load(EXPLOSION_ENGINE_SHEET);
	let eng_texture_atlas =
		TextureAtlas::from_grid(eng_texture_handle, Vec2::new(64., 64.), 4, 4, None, None);
	let engine = texture_atlases.add(eng_texture_atlas);

	// add GameTextures resource
	let game_textures = GameTextures {
		player: asset_server.load(PLAYER_SPRITE),
		player_laser: asset_server.load(PLAYER_LASER_SPRITE),
		enemy: asset_server.load(ENEMY_SPRITE),
		enemy_laser: asset_server.load(ENEMY_LASER_SPRITE),
		explosion,
		engine
	};
	commands.insert_resource(game_textures);
	commands.insert_resource(EnemyCount(0));
}

fn movable_system(
	mut commands: Commands,
	time: Res<Time>,
	win_size: Res<WinSize>,
	ui_state: Res<UiState>,
	mut query: Query<(Entity, &Velocity, &mut Transform, &Movable)>,
) {
	let delta = time.delta_seconds();

	for (entity, velocity, mut transform, movable) in &mut query {
		transform.translation.x += velocity.x * delta * BASE_SPEED;
		transform.translation.y += velocity.y * delta * BASE_SPEED;

		transform.rotate_z(
			velocity.omega * delta * BASE_ROT_SPEED
		);

		if movable.auto_despawn {
			// despawn when out of screen
			const MARGIN: f32 = 200.;
			if transform.translation.y > win_size.h / 2. + MARGIN
				|| transform.translation.y < -win_size.h / 2. - MARGIN
				|| transform.translation.x > win_size.w / 2. + MARGIN
				|| transform.translation.x < -win_size.w / 2. - MARGIN
			{
				commands.entity(entity).despawn();
			}
		} else {
			// wrap on other side of screen
			if transform.translation.y > win_size.h / 2. {
				transform.translation.y = -win_size.h / 2.;
			} else if transform.translation.y < -win_size.h / 2. {
				transform.translation.y = win_size.h / 2.;
			}

			if transform.translation.x > win_size.w / 2. {
				transform.translation.x = -win_size.w / 2.;
			} else if transform.translation.x < -win_size.w / 2. {
				transform.translation.x = win_size.w / 2.;
			}
		}
	}
}

#[allow(clippy::type_complexity)] // for the Query types.
fn player_laser_hit_enemy_system(
	mut commands: Commands,
	mut enemy_count: ResMut<EnemyCount>,
	laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromPlayer>)>,
	enemy_query: Query<(Entity, &Transform, &SpriteSize), With<Enemy>>,
) {
	let mut despawned_entities: HashSet<Entity> = HashSet::new();

	// iterate through the lasers
	for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
		if despawned_entities.contains(&laser_entity) {
			continue;
		}

		let laser_scale = laser_tf.scale.xy();

		// iterate through the enemies
		for (enemy_entity, enemy_tf, enemy_size) in enemy_query.iter() {
			if despawned_entities.contains(&enemy_entity)
				|| despawned_entities.contains(&laser_entity)
			{
				continue;
			}

			let enemy_scale = enemy_tf.scale.xy();

			// determine if collision
			let collision = collide(
				laser_tf.translation,
				laser_size.0 * laser_scale,
				enemy_tf.translation,
				enemy_size.0 * enemy_scale,
			);

			// perform collision
			if collision.is_some() {
				// remove the enemy
				commands.entity(enemy_entity).despawn();
				despawned_entities.insert(enemy_entity);
				enemy_count.0 -= 1;

				commands.spawn((ExplosionToSpawn {
					transform: Transform {
						translation: enemy_tf.translation,
						..Default::default()
					},
					duration: 0.05,
					is_engine: false
				},));
			}
		}
	}
}

#[allow(clippy::type_complexity)] // for the Query types.
fn enemy_laser_hit_player_system(
	mut commands: Commands,
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromEnemy>)>,
	player_query: Query<(Entity, &Transform, &SpriteSize), With<Player>>,
) {
	if let Ok((player_entity, player_tf, player_size)) = player_query.get_single() {
		let player_scale = player_tf.scale.xy();

		for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
			let laser_scale = laser_tf.scale.xy();

			// determine if collision
			let collision = collide(
				laser_tf.translation,
				laser_size.0 * laser_scale,
				player_tf.translation,
				player_size.0 * player_scale,
			);

			// perform the collision
			if collision.is_some() {
				// remove the player
				commands.entity(player_entity).despawn();
				player_state.shot(time.elapsed_seconds_f64());

				// remove the laser
				commands.entity(laser_entity).despawn();

				// spawn the explosionToSpawn
				commands.spawn((ExplosionToSpawn {
					transform: Transform {
						translation: player_tf.translation,
						..Default::default()
					},
					duration: 0.05,
					is_engine: false
				},));

				break;
			}
		}
	}
}

fn explosion_to_spawn_system(
	mut commands: Commands,
	game_textures: Res<GameTextures>,
	query: Query<(Entity, &ExplosionToSpawn)>,
) {
	for (explosion_spawn_entity, explosion_to_spawn) in query.iter() {

		let mut sprite_bundle = {
			SpriteSheetBundle {
				texture_atlas: if (explosion_to_spawn.is_engine) {game_textures.engine.clone()} else {game_textures.explosion.clone()},
				transform: explosion_to_spawn.transform.clone(),
				..Default::default()
			}
		};

		if (explosion_to_spawn.is_engine) {
			sprite_bundle.sprite.index = 6;
		}

		// spawn the explosion sprite
		commands
			.spawn(sprite_bundle)
			.insert(Explosion)
			.insert(ExplosionTimer::new(explosion_to_spawn.duration));

		// despawn the explosionToSpawn
		commands.entity(explosion_spawn_entity).despawn();
	}
}

fn explosion_animation_system(
	mut commands: Commands,
	time: Res<Time>,
	mut query: Query<(Entity, &mut ExplosionTimer, &mut TextureAtlasSprite), With<Explosion>>,
) {
	for (entity, mut timer, mut sprite) in &mut query {
		timer.0.tick(time.delta());
		if timer.0.finished() {
			sprite.index += 1; // move to next sprite cell
			if sprite.index >= EXPLOSION_LEN {
				commands.entity(entity).despawn();
			}
		}
	}
}
