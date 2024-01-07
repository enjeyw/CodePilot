#![allow(unused)] // silence unused warnings while exploring (to comment out)

use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::{math::Vec3Swizzles, diagnostic::LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use bevy::window::PrimaryWindow;
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


use enemy::EnemyPlugin;
use player::PlayerPlugin;
use post_processing::{PostProcessPlugin, PostProcessSettings};
use std::{collections::HashSet, f32::consts::PI};

mod components;
mod enemy;
mod player;
mod post_processing;

// region:    --- Asset Constants

const STAR_SPRITE: &str = "star2.png"; 
const TEST_SPRITE: &str = "test2.png"; 


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
		.init_resource::<UiState>()
		.init_resource::<CodePilotCode>()
		.add_plugins(FrameTimeDiagnosticsPlugin::default())
		.add_plugins(LogDiagnosticsPlugin::default())
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
		.add_plugins(PlayerPlugin)
		.add_plugins(EnemyPlugin)
		.add_systems(Startup, setup_system)
		.add_systems(Update, ui_system)
		.add_systems(Update, text_update_system)
		.add_systems(Update, tile_background_system)
		.add_systems(Update, movable_system)
		.add_systems(Update, player_laser_hit_enemy_system)
		.add_systems(Update, enemy_laser_hit_player_system)
		.add_systems(Update, explosion_to_spawn_system)
		.add_systems(Update, explosion_animation_system)
		.add_systems(Update, weapon_cooldown_system)
		.run();
}

fn ui_system(
	mut ui_state: ResMut<UiState>,
	mut contexts: EguiContexts) {
	let ctx = contexts.ctx_mut();

    egui::SidePanel::right("right_panel")
	.min_width(300.0)
	.show(ctx, |ui| {
        ui.vertical(|ui| {
			ui.label("Add Codepilot Code: ");
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
	commands.spawn(
		(
			Camera2dBundle {
				camera: Camera {
					hdr: true, // 1. HDR is required for bloom
					..default()
				},
				tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
				..default()
			},
			CameraMarker,
        	BloomSettings::NATURAL,
			PostProcessSettings {
				intensity: 0.0002,
				..default()
			},
		));

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
		player: asset_server.load(PLAYER_SPRITE),
		player_laser: asset_server.load(PLAYER_LASER_SPRITE),
		enemy: asset_server.load(ENEMY_SPRITE),
		enemy_laser: asset_server.load(ENEMY_LASER_SPRITE),
		explosion,
		engine
	};
	commands.insert_resource(game_textures);
	commands.insert_resource(EnemyCount(0));

	commands.spawn(SpriteBundle {
										texture: asset_server.load(TEST_SPRITE),
										sprite: Sprite {
											color: Color::rgb(3.0, 6.0, 5.0),
											..Default::default()
										},
										transform: Transform {
											scale: Vec3::new(0.3, 0.3, 1.),
											..Default::default()
										},
										..Default::default()
									});

	//Setup the HUD

	
	
	//Text in the top left to show current score
    commands.spawn((
        // Create a TextBundle that has a Text with a list of sections.
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
                    font_size: 30.0,
                    ..default()
                },
            ),
            TextSection::from_style(
                TextStyle {
                    font_size: 30.0,
					font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
                    color: Color::GOLD,
                    ..default()
			}),
        ]).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        }),
        ScoreText,
    ));

    // commands.spawn((
    //     // Create a TextBundle that has a Text with a list of sections.
    //     TextBundle::from_sections([
    //         TextSection::new(
    //             "Weapon Charge:",
    //             TextStyle {
    //                 font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
    //                 font_size: 20.0,
    //                 ..default()
    //             },
    //         )
    //     ]).with_style(Style {
    //         position_type: PositionType::Absolute,
    //         bottom: Val::Px(30.0),
    //         left: Val::Px(35.0),
    //         ..default()
    //     }),
    // ));

	//Text in the bottom right to show whether Codepilot is running
	commands.spawn((
		// Create a TextBundle that has a Text with a list of sections.
		TextBundle::from_sections([
			TextSection::new(
				"Codepilot: ",
				TextStyle {
					font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
					font_size: 20.0,
					..default()
				},
			),
			TextSection::from_style(
				TextStyle {
					font_size: 20.0,
					font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
					color: Color::RED,
					..default()
			}),
		])
		.with_text_alignment(TextAlignment::Left)
		.with_style(Style {
			position_type: PositionType::Absolute,
			bottom: Val::Px(10.0),
			right: Val::Px(350.0),
			..default()
		}),
		CodePilotActiveText,
	));

	commands
    .spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
			position_type: PositionType::Absolute,
            justify_content: JustifyContent::FlexStart,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            ..default()
        },
        ..default()
	}).with_children(|parent| {
		spawn_bar(parent, asset_server);
	});
	
}

fn spawn_bar(parent: &mut ChildBuilder, asset_server: Res<AssetServer>) {
    parent
        .spawn(NodeBundle {
            style: Style {
				padding: UiRect::all(Val::Px(20.)),
                height: Val::Px(30.0),
                width: Val::Px(400.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {

            parent.spawn(TextBundle::from_section(
				"Weapon Charge:",
				TextStyle {
					font: asset_server.load("fonts/ShareTechMono-Regular.ttf"),
					font_size: 20.0,
					..default()
				}));

            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(100.),
                        height: Val::Px(10.),
                        padding: UiRect::all(Val::Px(1.)),
                        align_items: AlignItems::Stretch,
                        top: Val::Px(2.0),
                        left: Val::Px(6.0),
                        ..Default::default()
                    },
                    background_color: Color::BLACK.into(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        NodeBundle {
                            style: Style {
                                width : Val::Percent(50.0),
                                ..Default::default()
                            },
                            background_color: Color::GREEN.into(),
                            ..Default::default()
                        },
                        WeaponChargeBar,
                    ));
                });
        });
}

//system for weapon cooldown
fn weapon_cooldown_system(
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	win_size: Res<WinSize>,
	mut chargebarquery: Query<(&mut Style, &mut BackgroundColor), With<WeaponChargeBar>>,
) {
	if player_state.weapon_cooldown > 0. {
		player_state.weapon_cooldown -= time.delta_seconds();

		if player_state.weapon_cooldown <= 0. {
			info!("Ready to fire!");
		} 
	}

	for (mut style, mut color) in chargebarquery.iter_mut() {
		color.0 = Color::rgb(1.0 * (player_state.weapon_cooldown / player_state.weapon_cooldown_max), 1.0 * (1. - player_state.weapon_cooldown / player_state.weapon_cooldown_max), 0.2);
		
		style.width = Val::Percent(100.0 * (1.0 - player_state.weapon_cooldown / player_state.weapon_cooldown_max));
	}
}

fn text_update_system(
    player_state: Res<PlayerState>,
	copilotcode: Res<CodePilotCode>,
    mut scorequery: Query<&mut Text, (Without<CodePilotActiveText>, With<ScoreText>)>,
	mut codepilotquery: Query<&mut Text,  (With<CodePilotActiveText>, Without<ScoreText>)>,
) {
	//Update the Score
    for mut text in &mut scorequery {
        // Update the value of the second section
		text.sections[1].value = format!("{0}", player_state.score);
    }

	//Display whether Codepilot is running
	for mut text in codepilotquery.iter_mut() {
		if copilotcode.compiled.is_some() {
			text.sections[1].value = format!("Active");
			text.sections[1].style.color = Color::GREEN;
		} else {
			text.sections[1].value = format!("Disabled");
			text.sections[1].style.color = Color::RED;
		}
	}
}



fn movable_system(
	mut commands: Commands,
	time: Res<Time>,
	win_size: Res<WinSize>,
	ui_state: Res<UiState>,
	mut player_query: Query<(&mut Transform, &mut Velocity, &mut Movable), (With<Player>, Without<CameraMarker>)>,
	mut camera_query: Query<&mut Transform, (With<CameraMarker>, Without<Player>)>,
	mut other_movable_query: Query<(Entity, &Velocity, &mut Transform, &Movable), (Without<Player>, Without<CameraMarker>)>
) {
	let delta = time.delta_seconds();

	if let Ok((mut player_tf, player_velocity, _)) = player_query.get_single_mut() {

		player_tf.translation.x += player_velocity.x * delta * BASE_SPEED;
		player_tf.translation.y += player_velocity.y * delta * BASE_SPEED;

		player_tf.rotate_z(
			player_velocity.omega * delta * BASE_ROT_SPEED
		);

		if let Ok(mut camera_tf) = camera_query.get_single_mut() {
			camera_tf.translation = player_tf.translation;
		}
	}
	
	for (entity, velocity, mut transform, movable) in &mut other_movable_query {
		transform.translation.x += velocity.x * delta * BASE_SPEED;
		transform.translation.y += velocity.y * delta * BASE_SPEED;

		transform.rotate_z(
			velocity.omega * delta * BASE_ROT_SPEED
		);

		// if movable.auto_despawn {
		// 	// despawn when out of screen
		// 	const MARGIN: f32 = 200.;
		// 	if transform.translation.y > win_size.h / 2. + MARGIN
		// 		|| transform.translation.y < -win_size.h / 2. - MARGIN
		// 		|| transform.translation.x > win_size.w / 2. + MARGIN
		// 		|| transform.translation.x < -win_size.w / 2. - MARGIN
		// 	{
		// 		commands.entity(entity).despawn();
		// 	}
		// } else {
		// 	// wrap on other side of screen
		// 	if transform.translation.y > win_size.h / 2. {
		// 		transform.translation.y = -win_size.h / 2.;
		// 	} else if transform.translation.y < -win_size.h / 2. {
		// 		transform.translation.y = win_size.h / 2.;
		// 	}

		// 	if transform.translation.x > win_size.w / 2. - 300. {
		// 		transform.translation.x = -win_size.w / 2.;
		// 	} else if transform.translation.x < -win_size.w / 2. {
		// 		transform.translation.x = win_size.w / 2. - 300.;
		// 	}
		// }
	}
}

#[allow(clippy::type_complexity)] // for the Query types.
fn player_laser_hit_enemy_system(
	mut commands: Commands,
	mut enemy_count: ResMut<EnemyCount>,
	mut player_state: ResMut<PlayerState>,
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
				player_state.score += 1;

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
				player_state.score = 0;

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
				sprite: TextureAtlasSprite {
					color: Color::rgb(5.0, 5.0, 5.0),
					..Default::default()
				}, 
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

#[derive(Component)]
pub struct Tile {
	pub x: i32,
	pub y: i32
}

#[derive(Component)]
pub struct Star;

fn tile_background_system(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
	win_size: Res<WinSize>,
	game_textures: Res<GameTextures>,
	camera_query: Query<&Transform, (With<CameraMarker>, Without<Player>)>,
	tile_query: Query<(Entity, &Tile)>
	
) {

	if let Ok(camera_tf) = camera_query.get_single() {
		let current_tile_x = (camera_tf.translation.x / win_size.w).floor() as i32;
		let cureent_tile_y = (camera_tf.translation.y / win_size.h).floor() as i32;

		tile_query.for_each(|(ent, tile)| {
			if (tile.x - current_tile_x).abs() > 1 || (tile.y - cureent_tile_y).abs() > 1 {
				commands.entity(ent).despawn();
			}
		});

		// spawn tiles around the camera
		for x in -1..=1 {
			for y in -1..=1 {
				let tile_x = current_tile_x + x;
				let tile_y = cureent_tile_y + y;

				if tile_query.iter().find(|(_, tile)| tile.x == tile_x && tile.y == tile_y).is_none() {
					commands
						.spawn(SpriteBundle {
							transform: Transform {
								translation: Vec3::new((tile_x as f32) * win_size.w, (tile_y as f32)* win_size.h, 0.),
								..Default::default()
							},
							..Default::default()
						})
						.insert(Tile {x: tile_x, y: tile_y})
						.with_children(|parent| {
							//Spawn Star Sprites for tile with deterministic random Transform
							let mut rng = thread_rng();
							// let mut rng = StdRng::seed_from_u64((tile_x.clone() as u64) * 10 + (tile_y.clone() as u64));

							for i in 0..20 {
								let x = rng.gen_range(-win_size.w / 2. .. win_size.w / 2.);
								let y = rng.gen_range(-win_size.h / 2. .. win_size.h / 2.);
								let scale = rng.gen_range(0.5.. 1.0);
								let rotation = rng.gen_range(0. .. 2. * PI);
								parent.spawn(
								(SpatialBundle {
									transform: Transform {
										translation: Vec3::new(x, y, 0.),
										rotation: Quat::from_rotation_z(0.),
										scale: Vec3::new(scale, scale, 1.),
										..Default::default()
									},
									visibility: Default::default(),
									inherited_visibility: Default::default(),
									view_visibility: Default::default(),
									global_transform: Default::default(),
								}))
								.insert(Star)
								.with_children(|sp_parent| {
									sp_parent.spawn(SpriteBundle {
										texture: game_textures.star.clone(),
										sprite: Sprite {
											color: Color::rgb(2.0, 1.5, 1.5),
											..Default::default()
										},
										transform: Transform {
											scale: Vec3::new(1.0, 1.0, 1.),
											..Default::default()
										},
										..Default::default()									
									});
									// sp_parent.spawn(
									// 	MaterialMesh2dBundle {
									// 		mesh: meshes.add(shape::Quad::new(Vec2::new(0.5, 25.)).into()).into(),
									// 		material: materials.add(ColorMaterial::from(Color::rgb(2.5, 2.0, 2.0))),
									// 		..default()
									// 	}
									// );
									// sp_parent.spawn(
									// 	MaterialMesh2dBundle {
									// 		mesh: meshes.add(shape::Quad::new(Vec2::new(25., 0.5)).into()).into(),
									// 		material: materials.add(ColorMaterial::from(Color::rgb(2.5, 2.0, 2.0))),
									// 		..default()
									// 	}
									// );
									sp_parent.spawn(
										MaterialMesh2dBundle {
											mesh: meshes.add(shape::Circle::new(3.).into()).into(),
											material: materials.add(ColorMaterial::from(Color::rgb(6.0, 6.0, 9.0))),
											..default()
										}
									);									
								});
							}
						});

				}
			}
		};


	}
}