use crate::components::{FromPlayer, Laser, Movable, Player, SpriteSize, Velocity, ExplosionToSpawn};
use crate::{
	GameTextures, PlayerState, WinSize, PLAYER_LASER_SIZE, PLAYER_RESPAWN_DELAY, PLAYER_SIZE,
	SPRITE_SCALE, UiState, CodePilotCode
};
use bevy::{prelude::*, ui};
use bevy::time::common_conditions::on_timer;
use rustpython_vm as vm;
use std::f32::consts::PI;
use std::time::Duration;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(PlayerState::default())
			.add_systems(
				Update,
				player_spawn_system.run_if(on_timer(Duration::from_secs_f32(0.5))),
			)
			.add_systems(Update, player_keyboard_event_system)
			.add_systems(Update, player_codepilot_system)
			.add_systems(Update, player_fire_system);
	}
}

fn player_spawn_system(
	mut commands: Commands,
	mut player_state: ResMut<PlayerState>,
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
			.insert(Velocity { x: 0., y: 0., omega: 0.});

		player_state.spawned();
	}
}

fn player_fire_system(
	mut commands: Commands,
	kb: Res<Input<KeyCode>>,
	game_textures: Res<GameTextures>,
	query: Query<&Transform, With<Player>>,
) {
	if let Ok(player_tf) = query.get_single() {
		if kb.just_pressed(KeyCode::Space) {
			let (x, y) = (player_tf.translation.x, player_tf.translation.y);
			let x_offset = PLAYER_SIZE.0 / 2. * SPRITE_SCALE - 5.;

			let mut spawn_laser = |x_offset: f32| {
				let velocity = player_tf.rotation * Vec3::X * 10.0;

				commands
					.spawn(SpriteBundle {
						texture: game_textures.player_laser.clone(),
						transform: Transform {
							translation: Vec3::new(x + x_offset, y, 0.),
							scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
							rotation: player_tf.rotation
						},
						..Default::default()
					})
					.insert(Laser)
					.insert(FromPlayer)
					.insert(SpriteSize::from(PLAYER_LASER_SIZE))
					.insert(Movable { auto_despawn: true })
					.insert(Velocity { x: velocity.x, y: velocity.y, omega: 0.});
			};

			spawn_laser(0.);
		}
	}
}

fn player_codepilot_system(
	kb: Res<Input<KeyCode>>,
	ui_state: Res<UiState>,	
	mut codepilot_code: ResMut<CodePilotCode>,
	mut commands: Commands,
) {

	if kb.pressed(KeyCode::Return) {

		let code_obj = vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
			let scope = vm.new_scope_with_builtins();
			let source = ui_state.player_code.as_str();
			
			let code_obj_res = vm
				.compile(source, vm::compiler::Mode::Exec, "<embedded>".to_owned())
				.map_err(|err| vm.new_syntax_error(&err, Some(source)));

			if let Ok(code_obj) = code_obj_res {
				info!("Compiled Result");
				codepilot_code.compiled = Some(code_obj);
			}
		});
	}
}

fn player_keyboard_event_system(
	kb: Res<Input<KeyCode>>,
	ui_state: Res<UiState>,
	codepilot_code: Res<CodePilotCode>,
	mut commands: Commands,
	mut query: Query<(&mut Velocity, &Transform), With<Player>>,
	enemy_query: Query<(&Velocity, &Transform), Without<Player>>,
) {
	let acceleration = 0.05;
	let ang_acceleration = 0.005;

	let max_speed = 2.0;
	let max_ang_velocity = 0.5;

	if let Ok((mut velocity, transform)) = query.get_single_mut() {

		if let Some(cpc) = codepilot_code.compiled.clone() {
			vm::Interpreter::without_stdlib(Default::default()).enter(|vm | {
				let scope = vm.new_scope_with_builtins();
				
				vm.run_code_obj(cpc, scope);
		
			});
		}

		let heading = transform.rotation * Vec3::X;
		let heading_perp = transform.rotation * Vec3::Y;
		let speed = velocity.y.hypot(velocity.x);
		let course = (velocity.y).atan2(velocity.x);

		// ensure speed is not greater than max speed

		if speed > max_speed {
			velocity.x = course.cos() * max_speed;
			velocity.y = course.sin() * max_speed;
		}

		if kb.pressed(KeyCode::W) {
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

		if kb.pressed(KeyCode::S) {
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

		if kb.pressed(KeyCode::A) {
			// increase angular velocity
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

		if kb.pressed(KeyCode::D) {
			// increase angular velocity
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
		
		// info!("speed: {} x: {} y: {} o: {}", speed, velocity.x, velocity.y, velocity.omega);

	}
}
