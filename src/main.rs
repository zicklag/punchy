use bevy::{prelude::*, utils::HashMap};
use bevy_parallax::{LayerData, ParallaxCameraComponent, ParallaxPlugin, ParallaxResource};
use heron::{prelude::*, SensorShape};

mod animation;
mod camera;
mod consts;
mod controller;
mod state;
mod y_sort;

use animation::*;
use camera::*;
use controller::*;
use state::State;
use y_sort::*;

#[derive(PhysicsLayer)]
enum BodyLayers {
    Enemy,
    Player,
}

#[derive(Component)]
pub struct Player {
    pub movement_speed: f32,
    pub facing_left: bool,
    pub state: State,
}

#[derive(Component)]
pub struct Stats {
    pub health: i32,
    pub damage: i32,
}

fn main() {
    let tile_size = Vec2::new(896., 480.);

    App::new()
        .insert_resource(ClearColor(Color::rgb(0.494, 0.658, 0.650)))
        .insert_resource(WindowDescriptor {
            title: "Fish Fight Punchy".to_string(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .insert_resource(ParallaxResource {
            layer_data: vec![
                LayerData {
                    speed: -0.15,
                    path: "background_01.png".to_string(),
                    tile_size,
                    cols: 2,
                    rows: 1,
                    z: 2.,
                    transition_factor: 0.9,
                    ..Default::default()
                },
                LayerData {
                    speed: -0.05,
                    path: "background_02.png".to_string(),
                    tile_size,
                    cols: 2,
                    rows: 1,
                    z: 1.,
                    transition_factor: 0.9,
                    ..Default::default()
                },
                LayerData {
                    speed: -0.0025,
                    path: "background_03.png".to_string(),
                    tile_size,
                    cols: 2,
                    rows: 1,
                    z: 0.,
                    transition_factor: 0.9,
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .add_plugin(ParallaxPlugin)
        .add_startup_system(setup)
        .add_system(player_controller)
        .add_system(camera_follow_player)
        .add_system(animation_cycling)
        .add_system(animation_flipping)
        .add_system(player_animation_state)
        .add_system(player_attack)
        .add_system(helper_camera_controller)
        .add_system(y_sort)
        .add_system(knock_enemies)
        .add_system(kill_entities)
        .add_system(knocked_state)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    // camera_bundle.orthographic_projection.depth_calculation = DepthCalculation::Distance;
    camera_bundle.orthographic_projection.scale = 0.65;
    commands
        .spawn_bundle(camera_bundle)
        .insert(ParallaxCameraComponent);

    let texture_handle = asset_server.load("PlayerFishy(96x80).png");
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
        Vec2::new(consts::PLAYER_WIDTH, consts::PLAYER_HEIGHT),
        14,
        7,
    );
    let atlas_handle = texture_atlases.add(texture_atlas);

    //Layers mapping to state
    let mut animation_map = HashMap::default();
    animation_map.insert(State::IDLE, 0..13);
    animation_map.insert(State::RUNNING, 14..19);
    animation_map.insert(State::KNOCKED, 71..76);
    animation_map.insert(State::DYING, 71..76);
    animation_map.insert(State::ATTACKING, 85..90);

    //Insert player
    commands
        .spawn_bundle(SpriteSheetBundle {
            sprite: TextureAtlasSprite::new(0),
            texture_atlas: atlas_handle,
            transform: Transform::from_xyz(0., consts::GROUND_Y, 0.),
            ..Default::default()
        })
        .insert(Player {
            movement_speed: 150.0,
            facing_left: false,
            state: State::IDLE,
        })
        .insert(Stats {
            health: 100,
            damage: 35,
        })
        .insert(RigidBody::Sensor)
        .insert(Collisions::default())
        .insert(CollisionShape::Cuboid {
            half_extends: Vec3::new(consts::PLAYER_WIDTH, consts::PLAYER_HITBOX_HEIGHT, 0.) / 8.,
            border_radius: None,
        })
        .insert(CollisionLayers::new(BodyLayers::Player, BodyLayers::Enemy))
        .insert(SensorShape)
        .insert(Animation::new(7. / 60., animation_map.clone()))
        .insert(YSort(100.));

    /*  commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("preview_stage.png"),
        transform: Transform::from_xyz(0., 0., 10.),
        ..Default::default()
    }); */

    let enemy_texture_handle = asset_server.load("PlayerSharky(96x80).png");
    let enemy_texture_atlas =
        TextureAtlas::from_grid(enemy_texture_handle, Vec2::new(96., 80.), 14, 7);
    let enemy_atlas_handle = texture_atlases.add(enemy_texture_atlas);

    //Insert enemies
    for pos in vec![
        (100., consts::GROUND_Y + 25.),
        (400., consts::GROUND_Y - 15.),
    ] {
        commands
            .spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite::new(0),
                texture_atlas: enemy_atlas_handle.clone(),
                transform: Transform::from_xyz(pos.0, pos.1, 0.),
                ..Default::default()
            })
            .insert(Stats {
                health: 100,
                damage: 35,
            })
            .insert(RigidBody::Sensor)
            .insert(Collisions::default())
            .insert(CollisionShape::Cuboid {
                half_extends: Vec3::new(consts::PLAYER_WIDTH, consts::PLAYER_HITBOX_HEIGHT, 0.)
                    / 8.,
                border_radius: None,
            })
            .insert(CollisionLayers::new(BodyLayers::Enemy, BodyLayers::Player))
            .insert(Animation::new(7. / 60., animation_map.clone()))
            .insert(YSort(100.));
    }

    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("floor.png"),
        transform: Transform::from_xyz(0., consts::GROUND_Y, 5.),
        ..Default::default()
    });
}

fn player_attack(
    mut query: Query<(&mut Player, &mut Transform, &Animation)>,
    keyboard: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let (mut player, mut transform, animation) = query.single_mut();

    if player.state != State::ATTACKING {
        if keyboard.just_pressed(KeyCode::Space) {
            player.state = State::ATTACKING;
        }
    } else {
        if animation.is_finished() {
            player.state = State::IDLE;
        } else {
            //TODO: Fix hacky way to get a forward jump
            if animation.current_frame < 3 {
                if player.facing_left {
                    transform.translation.x -= 200. * time.delta_seconds();
                } else {
                    transform.translation.x += 200. * time.delta_seconds();
                }
            }

            if animation.current_frame < 1 {
                transform.translation.y += 180. * time.delta_seconds();
            } else if animation.current_frame < 3 {
                transform.translation.y -= 90. * time.delta_seconds();
            }
        }
    }
}

fn knock_enemies(
    mut events: EventReader<CollisionEvent>,
    mut query: Query<(&mut Animation, &mut Stats)>,
) {
    events.iter().filter(|e| e.is_started()).for_each(|e| {
        let (e1, e2) = e.rigid_body_entities();
        let (l1, l2) = e.collision_layers();

        if l1.contains_group(BodyLayers::Player) && l2.contains_group(BodyLayers::Enemy) {
            let (player_anim, player_stats) = query.get(e1).unwrap();
            if let Ok((mut anim, mut stats)) = query.get_mut(e2) {
                if player_anim.current_state == Some(State::ATTACKING) {
                    stats.health = stats.health - player_stats.damage;
                    anim.set(State::KNOCKED);
                }
            }
        }
    })
}

fn kill_entities(mut commands: Commands, mut query: Query<(Entity, &Stats, &mut Animation)>) {
    for (entity, stats, mut animation) in query.iter_mut() {
        if stats.health <= 0 {
            animation.set(State::DYING);
        }

        if animation.current_state == Some(State::DYING) && animation.is_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn knocked_state(mut query: Query<&mut Animation>) {
    for mut animation in query.iter_mut() {
        if animation.current_state == Some(State::KNOCKED) && animation.is_finished() {
            animation.set(State::IDLE);
        }
    }
}
