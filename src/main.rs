#![allow(unused)] // silence unused warnings while exploring (to comment out)

use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::text::BreakLineOn;
use bevy::{math::Vec3Swizzles, diagnostic::LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui::Id;
use components::{
	CameraMarker, Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, Movable,
	Player, SpriteSize, Velocity, ScoreText, MaxScoreText, CodePilotActiveText, WeaponChargeBar
};
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use rustpython_vm as vm;
use vm::{builtins::PyCode, PyRef};
use std::sync::atomic::{AtomicBool, Ordering};
use rand::{Rng, rngs::StdRng, SeedableRng, thread_rng};

use ui::UIPlugin;
use movement::MovementPlugin;
use enemy::EnemyPlugin;
use player::PlayerPlugin;
use codepilot::CodePilotPlugin;
use combat::CombatPlugin;
use post_processing::{PostProcessPlugin, PostProcessSettings};
use std::{collections::HashSet, f32::consts::PI};

mod autocomplete;
mod ui;
mod movement;
mod post_processing;
mod components;
mod enemy;
mod player;
mod codepilot;
mod combat;

// region:    --- Asset Constants

const SHIELD_SPRITE: &str = "shield2.png";
const STAR_SPRITE: &str = "star2.png"; 
const TEST_SPRITE: &str = "test2.png"; 

const EMP_SPRITE: &str = "shield2.png";

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

const NEAR_WHITE: Color = Color::rgb(3.0, 3.0, 5.0);

// endregion: --- Asset Constants

// region:    --- Game Constants

const BASE_SPEED: f32 = 200.;
const BASE_ROT_SPEED: f32 = 10.;


const PLAYER_RESPAWN_DELAY: f64 = 2.;
const ENEMY_MAX: u32 = 3;
const FORMATION_MEMBERS_MAX: u32 = 3;

// endregion: --- Game Constants

// region:    --- Resources
#[derive(Resource)]
pub struct WinSize {
	pub w: f32,
	pub h: f32,
}

#[derive(Resource)]
struct GameTextures {
	star: Handle<Image>,
	emp: Handle<Image>,
	shield: Handle<Image>,
	player: Handle<Image>,
	player_laser: Handle<Image>,
	enemy: Handle<Image>,
	enemy_laser: Handle<Image>,
	explosion: Handle<TextureAtlas>,
	engine: Handle<TextureAtlas>
}

#[derive(Resource)]
pub struct CodePilotCode {
	raw_code: String,
    compiled: Option<PyRef<PyCode>>,
	completions: Vec<String>,
	autocomplete_token: String,
	cursor_index: Option<usize>,
	selected_completion: usize,
}
impl Default for CodePilotCode {
	fn default() -> Self {
		Self {
			raw_code: String::new(),
			compiled: None,
			completions: Vec::new(),
			autocomplete_token: String::new(),
			cursor_index: None,
			selected_completion: 0
		}
	}
}

#[derive(Resource)]
struct EnemyCount(u32);

#[derive(Resource)]
struct CollidedEntities(HashSet<(Entity,Entity)>);


#[derive(Resource)]
struct PlayerState {
	on: bool,       // alive
	weapon_cooldown: f32, // time until next shot
	weapon_cooldown_max: f32, // time between shots
	last_shot: f64, // -1 if not shot
	score: u32,
}
impl Default for PlayerState {
	fn default() -> Self {
		Self {
			on: false,
			weapon_cooldown: 0.,
			weapon_cooldown_max: 1.,
			last_shot: -1.,
			score: 0
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

// #[derive(Resource)]


// endregion: --- Resources

fn main() {

	App::new()
		// .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
		.insert_resource(ClearColor(Color::rgb(0.00, 0.00, 0.08)))
		.init_resource::<CodePilotCode>()
		.add_plugins(FrameTimeDiagnosticsPlugin::default())
		// .add_plugins(LogDiagnosticsPlugin::default())
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Codepilot".into(),
				resolution: (1400., 800.).into(),
				..Default::default()
			}),
			..Default::default()
		}))
		// .add_plugins(PostProcessPlugin) //Can't have bloom and Post Pipeline :(
		.add_plugins(EguiPlugin)
		.add_plugins(UIPlugin)
		.add_plugins(MovementPlugin)
		.add_plugins(PlayerPlugin)
		.add_plugins(CodePilotPlugin)
		.add_plugins(EnemyPlugin)
		.add_plugins(CombatPlugin)
		.add_systems(Startup, setup_system)
		.run();
}

fn setup_system(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
	query: Query<&Window, With<PrimaryWindow>>,
) {

	let mut bloom_settings = BloomSettings::NATURAL;
	bloom_settings.intensity = 0.15;
	bloom_settings.high_pass_frequency = 0.6;

	// camera
	let camera_id: Entity = commands.spawn(
		(
			Camera2dBundle {
				camera: Camera {
					hdr: true,
					..default()
				},
				tonemapping: Tonemapping::TonyMcMapface,
				..default()
			},
        	bloom_settings,
			PostProcessSettings {
				intensity: 0.0002,
				..default()
			},
			CameraMarker
		)).id();

	camera_id.index();
	


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
		star: asset_server.load(STAR_SPRITE),
		shield: asset_server.load(SHIELD_SPRITE),
		emp: asset_server.load(EMP_SPRITE),
		player: asset_server.load(PLAYER_SPRITE),
		player_laser: asset_server.load(PLAYER_LASER_SPRITE),
		enemy: asset_server.load(ENEMY_SPRITE),
		enemy_laser: asset_server.load(ENEMY_LASER_SPRITE),
		explosion,
		engine
	};
	commands.insert_resource(game_textures);
	commands.insert_resource(EnemyCount(0));
	commands.insert_resource(CollidedEntities(HashSet::new()));

	commands.spawn(SpriteBundle {
			texture: asset_server.load(TEST_SPRITE),
			sprite: Sprite {
				color: Color::rgb(1.4, 2.0, 1.8),
				..Default::default()
			},
			transform: Transform {
				scale: Vec3::new(0.3, 0.3, 1.),
				..Default::default()
			},
			..Default::default()
	}).with_children(|parent| {
		parent.spawn(Text2dBundle {
			text: Text {
				sections: vec![TextSection::new(
					"Shields: 100%",
					TextStyle {
						font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
						font_size: 50.0,
						color: Color::rgb(1.0, 4.0, 2.0),
						..default()
					}
				)],
				alignment: TextAlignment::Left,
				linebreak_behavior: BreakLineOn::WordBoundary,
			},
			transform: Transform {
				translation: Vec3::new(-300., -200., 0.0),
				..Default::default()
			},
			..default()
		});
	});
	
}