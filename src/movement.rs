use std::f64::consts::PI;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use rand::{thread_rng, Rng};

use crate::{WinSize, GameTextures, components::{CameraMarker, Player, Tile, Velocity, Movable, Star}, UiState, BASE_SPEED, BASE_ROT_SPEED};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Update, tile_background_system)
        .add_systems(Update, movable_system);
    }
}

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
											color: Color::rgb(1.8, 1.3, 1.3),
											..Default::default()
										},
										transform: Transform {
											scale: Vec3::new(1.0, 1.0, 1.),
											..Default::default()
										},
										..Default::default()									
									});
							
									sp_parent.spawn(
										MaterialMesh2dBundle {
											mesh: meshes.add(shape::Circle::new(3.).into()).into(),
											material: materials.add(ColorMaterial::from(Color::rgb(2., 2., 4.0))),
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

	}
}