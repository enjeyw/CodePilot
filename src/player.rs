use crate::combat::{FireWeaponEvent, WeaponType, Allegiance, spawn_shield_sprite};
use crate::components::{FromPlayer, Laser, Movable, Player, SpriteSize, Velocity, ExplosionToSpawn, Enemy, Weapon, Ship, EMP};
use crate::{
	GameTextures, PlayerState, WinSize, PLAYER_LASER_SIZE, PLAYER_RESPAWN_DELAY, PLAYER_SIZE,
	SPRITE_SCALE, CodePilotCode, enemy
};
use bevy::sprite::MaterialMesh2dBundle;
use bevy::{prelude::*, ui};
use bevy::time::common_conditions::on_timer;
use std::f32::consts::PI;
use std::fmt::Result;
use std::time::Duration;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(PlayerState::default())
			.add_systems(
				Update,
				player_spawn_system.run_if(on_timer(Duration::from_secs_f32(0.5))),
			)
			.add_systems(Update, player_upgrade_system)
			.add_systems(Update, player_keyboard_event_system)
			.add_systems(Update, player_fire_system);
		
	}
}

fn player_spawn_system(
	mut commands: Commands,
	mut player_state: ResMut<PlayerState>,
	mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
	time: Res<Time>,
	game_textures: Res<GameTextures>,
	win_size: Res<WinSize>,
) {
	let now = time.elapsed_seconds_f64();
	let last_shot = player_state.last_shot;

	if !player_state.on && (last_shot == -1. || now > last_shot + PLAYER_RESPAWN_DELAY) {
		// add player
		let bottom = -win_size.h / 4.;
		commands

			.spawn(SpriteBundle {
				texture: game_textures.player.clone(),
				transform: Transform {
					translation: Vec3::new(
						0.,
						bottom + PLAYER_SIZE.1 / 2. * SPRITE_SCALE + 5.,
						10.,
					),
					rotation: Quat::from_rotation_z(0.),
					scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
					..Default::default()
				},
				..Default::default()
			})
			.insert(Player)
			.insert(SpriteSize::from(PLAYER_SIZE))
			.insert(Movable { auto_despawn: false })
			.insert(Velocity { x: 0., y: 0., omega: 0.})
			.insert(Ship {
				max_shields: 1.,
				current_shields: 1.0,
				sheild_carge_rate: 0.1,
			})
			.insert(Allegiance::Friendly)
			.with_children(|parent| {
				spawn_shield_sprite(parent, game_textures);
			});

			// .spawn(
			// 	(SpatialBundle {
			// 		transform: Transform {
			// 				translation: Vec3::new(
			// 					0.,
			// 					bottom + PLAYER_SIZE.1 / 2. * SPRITE_SCALE + 5.,
			// 					10.,
			// 				),
			// 				rotation: Quat::from_rotation_z(0.),
			// 				scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
			// 				..Default::default()
			// 			},
			// 		visibility: Default::default(),
			// 		inherited_visibility: Default::default(),
			// 		view_visibility: Default::default(),
			// 		global_transform: Default::default(),
			// 	})).with_children(|parent| {
			// 		parent.spawn(MaterialMesh2dBundle {
			// 			mesh: meshes.add(shape::RegularPolygon::new(100., 3).into()).into(),
			// 			material: materials.add(ColorMaterial::from(Color::rgb(6.0, 6.0, 9.0))),
			// 			transform: Transform {
			// 				translation: Vec3::new(
			// 					0.,
			// 					0.,
			// 					0.
			// 				),
			// 				rotation: Quat::from_rotation_z(-PI/2.),
			// 				scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
			// 				..Default::default()
			// 			},
			// 			..default()
			// 		});

			// 	})
				
			
			

		player_state.spawned();
	}
}

fn player_upgrade_system(
	mut commands: Commands,
	mut player_state: ResMut<PlayerState>,
	mut player_query: Query<Entity, With<Player>>,
	emp_query: Query<Entity, With<EMP>>,
) {

	if emp_query.iter().count() > 0 {
		return;
	}

	if player_state.score >= 2 {
		
		if let Ok(player) = player_query.get_single_mut() {
			let child = commands.spawn(Weapon {
				current_charge: 0.,
				charge_rate: 0.5,
			})
			.insert(EMP)
			.id();

			commands.entity(player).push_children(&[child]);
		}

	}
}

pub fn try_fire_weapon(
	commands: &mut Commands,
	game_textures: &Res<GameTextures>,
	player_state: &mut ResMut<PlayerState>,
	player_tf: &Transform,

) -> bool {

	if player_state.weapon_cooldown > 0. {
		return false;
	}

	let (x, y) = (player_tf.translation.x, player_tf.translation.y);
	let x_offset = PLAYER_SIZE.0 / 2. * SPRITE_SCALE - 5.;

	let mut spawn_laser = |x_offset: f32| {
		let velocity = player_tf.rotation * Vec3::X * 10.0;

		commands
			.spawn(SpriteBundle {
				texture: game_textures.player_laser.clone(),
				sprite: Sprite {
					color: Color::rgb(5.0, 5.0, 5.0),
					..Default::default()
				},
				transform: Transform {
					translation: Vec3::new(x + x_offset, y, 0.),
					scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
					rotation: player_tf.rotation
				},
				..Default::default()
			})
			.insert(Laser)
			.insert(Allegiance::Friendly)
			.insert(SpriteSize::from(PLAYER_LASER_SIZE))
			.insert(Movable { auto_despawn: true })
			.insert(Velocity { x: velocity.x, y: velocity.y, omega: 0.});
	};

	spawn_laser(0.);

	player_state.weapon_cooldown = player_state.weapon_cooldown_max;

	return true;
}

pub fn accelerate_forward(
	velocity: &mut Velocity,
	transform: &Transform,
	acceleration: f32,
	max_speed: f32,
	heading: Vec3,
	heading_perp: Vec3,
	commands: &mut Commands,
) {
	velocity.x += heading.x * acceleration;
	velocity.y += heading.y * acceleration;

	commands.spawn((ExplosionToSpawn {
		transform: Transform {
			translation: transform.translation - Vec3::Z - heading * 25.,
			scale: Vec3{x: 0.6, y: 0.2, z: 1.},
			rotation: transform.rotation,
			..Default::default()
		},
		duration: 0.001,
		is_engine: true
	},));


	commands.spawn((ExplosionToSpawn {
		transform: Transform {
			translation: transform.translation - Vec3::Z - heading * 15. + heading_perp * 20.,
			scale: Vec3{x: 0.3, y: 0.15, z: 1.},
			rotation: transform.rotation,
			..Default::default()
		},
		duration: 0.001,
		is_engine: true
	},));

	commands.spawn((ExplosionToSpawn {
		transform: Transform {
			translation: transform.translation - Vec3::Z - heading * 15. - heading_perp * 20.,
			scale: Vec3{x: 0.3, y: 0.15, z: 1.},
			rotation: transform.rotation,
			..Default::default()
		},
		duration: 0.001,
		is_engine: true
	},));
}

pub fn accelerate_backward(
	velocity: &mut Velocity,
	transform: &Transform,
	acceleration: f32,
	max_speed: f32,
	heading: Vec3,
	heading_perp: Vec3,
	commands: &mut Commands,
) {
	velocity.x -= heading.x * acceleration;
	velocity.y -= heading.y * acceleration;


	commands.spawn((ExplosionToSpawn {
		transform: Transform {
			translation: transform.translation - Vec3::Z + heading * 15. + heading_perp * 20.,
			scale: Vec3{x: 0.3, y: 0.15, z: 1.},
			rotation: transform.rotation,
			..Default::default()
		},
		duration: 0.001,
		is_engine: true
	},));

	commands.spawn((ExplosionToSpawn {
		transform: Transform {
			translation: transform.translation - Vec3::Z + heading * 15. - heading_perp * 20.,
			scale: Vec3{x: 0.3, y: 0.15, z: 1.},
			rotation: transform.rotation,
			..Default::default()
		},
		duration: 0.001,
		is_engine: true
	},));
}

pub fn accelerate_counterclockwise(
	velocity: &mut Velocity,
	transform: &Transform,
	ang_acceleration: f32,
	max_ang_velocity: f32,
	heading: Vec3,
	heading_perp: Vec3,
	commands: &mut Commands,
) {
	velocity.omega += ang_acceleration;

	if velocity.omega > max_ang_velocity {
		velocity.omega = max_ang_velocity;
	}

	commands.spawn((ExplosionToSpawn {
		transform: Transform {
			translation: transform.translation - Vec3::Z + heading * 10. - heading_perp * 30.,
			scale: Vec3{x: 0.3, y: 0.1, z: 1.},
			rotation: transform.rotation.mul_quat(Quat::from_rotation_z(PI / 2.)),
			..Default::default()
		},
		duration: 0.001,
		is_engine: true
	},));
}

pub fn accelerate_clockwise (
	velocity: &mut Velocity,
	transform: &Transform,
	ang_acceleration: f32,
	max_ang_velocity: f32,
	heading: Vec3,
	heading_perp: Vec3,
	commands: &mut Commands,
) {
	velocity.omega -= ang_acceleration;

	if velocity.omega < -max_ang_velocity {
		velocity.omega = -max_ang_velocity;
	}

	commands.spawn((ExplosionToSpawn {
		transform: Transform {
			translation: transform.translation - Vec3::Z + heading * 10. + heading_perp * 30.,
			scale: Vec3{x: 0.3, y: 0.1, z: 1.},
			rotation: transform.rotation.mul_quat(Quat::from_rotation_z(PI / 2.)),
			..Default::default()
		},
		duration: 0.001,
		is_engine: true
	},));
}

fn player_fire_system(
	mut commands: Commands,
	kb: Res<Input<KeyCode>>,
	game_textures: Res<GameTextures>,
	mut fire_weapon_event: EventWriter<FireWeaponEvent>,
	mut player_state: ResMut<PlayerState>,
	query: Query<(Entity, &Transform), With<Player>>,
) {
	if let Ok((player_ent, player_tf)) = query.get_single() {
		if kb.just_pressed(KeyCode::Space) {
			try_fire_weapon(&mut commands, &game_textures, &mut player_state, player_tf);
		}

		if kb.just_pressed(KeyCode::M) {
			info!("Sending fire EMP event");
			fire_weapon_event.send({
				FireWeaponEvent {
					weapon_type: WeaponType::EMP,
					weapon_alignment: Allegiance::Friendly,
					firing_entity: player_ent
				}
			});
		}


	}
}

fn player_keyboard_event_system(
	mut commands: Commands,
	kb: Res<Input<KeyCode>>,
	codepilot_code: Res<CodePilotCode>,
	game_textures: Res<GameTextures>,
	mut player_state: ResMut<PlayerState>,
	mut query: Query<(&mut Velocity, &Transform), With<Player>>,
	enemy_query: Query<(&Velocity, &Transform), (Without<Player>, With<Enemy>)>,
) {
	let acceleration = 0.05;
	let ang_acceleration = 0.005;

	let max_speed = 1.6;
	let max_ang_velocity = 0.5;
	

	if let Ok((mut velocity, transform)) = query.get_single_mut() {

		let heading = transform.rotation * Vec3::X;
		let heading_perp = transform.rotation * Vec3::Y;
		let speed = velocity.y.hypot(velocity.x);
		let course = (velocity.y).atan2(velocity.x);

		// ensure speed is not greater than max speed
		if speed > max_speed {
			velocity.x = course.cos() * max_speed;
			velocity.y = course.sin() * max_speed;
		}

		// Player keyboard control section
		if kb.pressed(KeyCode::W) {
			accelerate_forward(
				&mut velocity, transform, acceleration, max_speed, heading, heading_perp, &mut commands
			)
		}

		if kb.pressed(KeyCode::S) {
			accelerate_backward(
				&mut velocity, transform, acceleration, max_speed, heading, heading_perp, &mut commands
			)
		}

		if kb.pressed(KeyCode::A) {
			accelerate_counterclockwise(
				&mut velocity, transform, ang_acceleration, max_ang_velocity, heading, heading_perp, &mut commands)
		}

		if kb.pressed(KeyCode::D) {
			accelerate_clockwise(
				&mut velocity, transform, ang_acceleration, max_ang_velocity, heading, heading_perp, &mut commands)
	
		}
		
		// info!("speed: {} x: {} y: {} o: {}", speed, velocity.x, velocity.y, velocity.omega);
	}
}
