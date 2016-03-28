
use ecs;
use super::{
    Position,
    Velocity,
    Rotation,
    Gravity,
    Bounds,
    GameInfo,
};
use world;
use render;
use render::model;
use types::Gamemode;
use collision::{Aabb, Aabb3};
use cgmath::{self, Point3, Vector3, Matrix4, Decomposed, Rotation3, Rad, Angle, Quaternion};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use types::hash::FNVHash;
use sdl2::keyboard::Keycode;

pub fn add_systems(m: &mut ecs::Manager) {
    // Not actually rendering related but the faster
    // we can handle input the better.
    let sys = MovementHandler::new(m);
    m.add_render_system(sys);
    let sys = PlayerRenderer::new(m);
    m.add_render_system(sys);
}

pub fn create_local(m: &mut ecs::Manager) -> ecs::Entity {
    let entity = m.create_entity();
    m.add_component_direct(entity, Position::new(0.0, 0.0, 0.0));
    m.add_component_direct(entity, Rotation::new(0.0, 0.0));
    m.add_component_direct(entity, Velocity::new(0.0, 0.0, 0.0));
    m.add_component_direct(entity, Gamemode::Survival);
    m.add_component_direct(entity, Gravity::new());
    m.add_component_direct(entity, PlayerMovement::new());
    m.add_component_direct(entity, Bounds::new(Aabb3::new(
        Point3::new(-0.3, 0.0, -0.3),
        Point3::new(0.3, 1.8, 0.3)
    )));
    m.add_component_direct(entity, PlayerModel::new(false, false, true));
    entity
}


pub struct PlayerModel {
    model: Option<model::ModelKey>,

    has_head: bool,
    has_name_tag: bool,
    first_person: bool,

    dir: i32,
    time: f64,
    still_time: f64,
    idle_time: f64,
    arm_time: f64,
}

impl PlayerModel {
    pub fn new(has_head: bool, has_name_tag: bool, first_person: bool) -> PlayerModel {
        PlayerModel {
            model: None,

            has_head: has_head,
            has_name_tag: has_name_tag,
            first_person: first_person,

            dir: 0,
            time: 0.0,
            still_time: 0.0,
            idle_time: 0.0,
            arm_time: 0.0,
        }
    }
}

struct PlayerRenderer {
    filter: ecs::Filter,
    player_model: ecs::Key<PlayerModel>,
    position: ecs::Key<Position>,
    rotation: ecs::Key<Rotation>,
    game_info: ecs::Key<GameInfo>,
}

impl PlayerRenderer {
    fn new(m: &mut ecs::Manager) -> PlayerRenderer {
        let player_model = m.get_key();
        let position = m.get_key();
        let rotation = m.get_key();
        PlayerRenderer {
            filter: ecs::Filter::new()
                .with(player_model)
                .with(position)
                .with(rotation),
            player_model: player_model,
            position: position,
            rotation: rotation,
            game_info: m.get_key(),
        }
    }
}

enum PlayerModelPart {
    Head = 0,
    Body = 1,
    LegLeft = 2,
    LegRight = 3,
    ArmLeft = 4,
    ArmRight = 5,
    Cape = 6,
    NameTag = 7
}

// TODO: Setup culling
impl ecs::System for PlayerRenderer {

    fn filter(&self) -> &ecs::Filter {
        &self.filter
    }

    fn update(&mut self, m: &mut ecs::Manager, _: &mut world::World, renderer: &mut render::Renderer) {
        use std::f32::consts::PI;
        use std::f64::consts::PI as PI64;
        let world_entity = m.get_world();
        let delta = m.get_component_mut(world_entity, self.game_info).unwrap().delta;
        for e in m.find(&self.filter) {
            let player_model = m.get_component_mut(e, self.player_model).unwrap();
            let position = m.get_component_mut(e, self.position).unwrap();
            let rotation = m.get_component_mut(e, self.rotation).unwrap();

            if let Some(pmodel) = player_model.model {
                let mdl = renderer.model.get_model(pmodel).unwrap();
                let offset = if player_model.first_person {
                    let ox = (rotation.yaw - PI64/2.0).cos() * 0.25;
                    let oz = -(rotation.yaw - PI64/2.0).sin() * 0.25;
                    Vector3::new(
                        position.position.x as f32 - ox as f32,
                        -position.position.y as f32,
                        position.position.z as f32 - oz as f32,
                    )
                } else {
                    Vector3::new(
                        position.position.x as f32,
                        -position.position.y as f32,
                        position.position.z as f32,
                    )
                };
                let offset_matrix = Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_y(Rad::new((PI + rotation.yaw as f32))),
                    disp: offset,
                });

                mdl.matrix[PlayerModelPart::Head as usize] = offset_matrix * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad::new(-rotation.pitch as f32)),
                    disp: Vector3::new(0.0, -12.0/16.0 - 12.0/16.0, 0.0),
                });
                mdl.matrix[PlayerModelPart::Body as usize] = offset_matrix * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad::new(0.0)),
                    disp: Vector3::new(0.0, -12.0/16.0 - 6.0/16.0, 0.0),
                });

                let mut time = player_model.time;
                let mut dir = player_model.dir;
                if dir == 0 {
                    dir = 1;
                    time = 15.0;
                }
                let ang = ((time / 15.0) - 1.0) * (PI64 / 4.0);

                mdl.matrix[PlayerModelPart::LegRight as usize] = offset_matrix * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad::new(ang as f32)),
                    disp: Vector3::new(2.0/16.0, -12.0/16.0, 0.0),
                });
                mdl.matrix[PlayerModelPart::LegLeft as usize] = offset_matrix * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad::new(-ang as f32)),
                    disp: Vector3::new(-2.0/16.0, -12.0/16.0, 0.0),
                });

                let mut i_time = player_model.idle_time;
                i_time += delta * 0.02;
                if i_time > PI64 * 2.0 {
                    i_time -= PI64 * 2.0;
                }
                player_model.idle_time = i_time;

                if player_model.arm_time <= 0.0 {
                    player_model.arm_time = 0.0;
                } else {
                    player_model.arm_time -= delta;
                }

                mdl.matrix[PlayerModelPart::ArmRight as usize] = offset_matrix * Matrix4::from_translation(
                    Vector3::new(6.0/16.0, -12.0/16.0-12.0/16.0, 0.0)
                ) * Matrix4::from(Quaternion::from_angle_x(Rad::new(-(ang * 0.75) as f32)))
                  * Matrix4::from(Quaternion::from_angle_z(Rad::new((i_time.cos() * 0.06 - 0.06) as f32)))
                  * Matrix4::from(Quaternion::from_angle_x(Rad::new((i_time.sin() * 0.06 - ((7.5 - (player_model.arm_time-7.5).abs()) / 7.5)) as f32)));

                mdl.matrix[PlayerModelPart::ArmLeft as usize] = offset_matrix * Matrix4::from_translation(
                  Vector3::new(-6.0/16.0, -12.0/16.0-12.0/16.0, 0.0)
                ) * Matrix4::from(Quaternion::from_angle_x(Rad::new((ang * 0.75) as f32)))
                  * Matrix4::from(Quaternion::from_angle_z(Rad::new(-(i_time.cos() * 0.06 - 0.06) as f32)))
                  * Matrix4::from(Quaternion::from_angle_x(Rad::new(-(i_time.sin() * 0.06) as f32)));

                let mut update = true;
                if !position.moved {
                    if player_model.still_time > 5.0 {
                        if (time - 15.0).abs() <= 1.5 * delta {
                            time = 15.0;
                            update = false;
                        }
                        dir = (15.0 - time).signum() as i32;
                    } else {
                        player_model.still_time += delta;
                    }
                } else {
                    player_model.still_time = 0.0;
                }

                if update {
                    time += delta * 1.5 * (dir as f64);
                    if time > 30.0 {
                        time = 30.0;
                        dir = -1;
                    } else if time < 0.0 {
                        time = 0.0;
                        dir = 1;
                    }
                }
                player_model.time = time;
                player_model.dir = dir;
            }
        }
    }

    fn entity_added(&mut self, m: &mut ecs::Manager, e: ecs::Entity, _: &mut world::World, renderer: &mut render::Renderer) {
        let player_model = m.get_component_mut(e, self.player_model).unwrap();

        let skin = render::Renderer::get_texture(renderer.get_textures_ref(), "entity/steve");

        macro_rules! srel {
            ($x:expr, $y:expr, $w:expr, $h:expr) => (
                Some(skin.relative(($x) / 64.0, ($y) / 64.0, ($w) / 64.0, ($h) / 64.0))
            );
        }

        let mut head_verts = vec![];
        if player_model.has_head {
            model::append_box(&mut head_verts, -4.0/16.0, 0.0, -4.0/16.0, 8.0/16.0, 8.0/16.0, 8.0/16.0, [
                srel!(8.0, 0.0, 8.0, 8.0), // Up
                srel!(16.0, 0.0, 8.0, 8.0), // Down
                srel!(8.0, 8.0, 8.0, 8.0), // North
                srel!(24.0, 8.0, 8.0, 8.0), // South
                srel!(16.0, 8.0, 8.0, 8.0), // West
                srel!(0.0, 8.0, 8.0, 8.0), // East
            ]);
            model::append_box(&mut head_verts, -4.2/16.0, -0.2, -4.2/16.0, 8.4/16.0, 8.4/16.0, 8.4/16.0, [
                srel!((8.0 + 32.0), 0.0, 8.0, 8.0), // Up
                srel!((16.0 + 32.0), 0.0, 8.0, 8.0), // Down
                srel!((8.0 + 32.0), 8.0, 8.0, 8.0), // North
                srel!((24.0 + 32.0), 8.0, 8.0, 8.0), // South
                srel!((16.0 + 32.0), 8.0, 8.0, 8.0), // West
                srel!((0.0 + 32.0), 8.0, 8.0, 8.0), // East
            ]);
        }

        // TODO: Cape
        let mut body_verts = vec![];
        model::append_box(&mut body_verts, -4.0/16.0, -6.0/16.0, -2.0/16.0, 8.0/16.0, 12.0/16.0, 4.0/16.0, [
            srel!(20.0, 16.0, 8.0, 4.0), // Up
            srel!(28.0, 16.0, 8.0, 4.0), // Down
            srel!(20.0, 20.0, 8.0, 12.0), // North
            srel!(32.0, 20.0, 8.0, 12.0), // South
            srel!(16.0, 20.0, 4.0, 12.0), // West
            srel!(28.0, 20.0, 4.0, 12.0), // East
        ]);
        model::append_box(&mut body_verts, -4.2/16.0, -6.2/16.0, -2.2/16.0, 8.4/16.0, 12.4/16.0, 4.4/16.0, [
            srel!(20.0, 16.0 + 16.0, 8.0, 4.0), // Up
            srel!(28.0, 16.0 + 16.0, 8.0, 4.0), // Down
            srel!(20.0, 20.0 + 16.0, 8.0, 12.0), // North
            srel!(32.0, 20.0 + 16.0, 8.0, 12.0), // South
            srel!(16.0, 20.0 + 16.0, 4.0, 12.0), // West
            srel!(28.0, 20.0 + 16.0, 4.0, 12.0), // East
        ]);

        let mut part_verts = vec![vec![]; 4];

        for (i, offsets) in [
            [16.0, 48.0, 0.0, 48.0],
            [0.0, 16.0, 0.0, 32.0],
            [40.0, 16.0, 40.0, 32.0],
            [32.0, 48.0, 48.0, 48.0],
        ].into_iter().enumerate() {
            let (ox, oy) = (offsets[0], offsets[1]);
            model::append_box(&mut part_verts[i], -2.0/16.0, -12.0/16.0, -2.0/16.0, 4.0/16.0, 12.0/16.0, 4.0/16.0, [
                srel!(ox + 4.0, oy + 0.0, 4.0, 4.0), // Up
                srel!(ox + 8.0, oy + 0.0, 4.0, 4.0), // Down
                srel!(ox + 4.0, oy + 4.0, 4.0, 12.0), // North
                srel!(ox + 12.0, oy + 4.0, 4.0, 12.0), // South
                srel!(ox + 0.0, oy + 4.0, 4.0, 12.0), // West
                srel!(ox + 8.0, oy + 4.0, 4.0, 12.0), // East
            ]);
            let (ox, oy) = (offsets[2], offsets[3]);
            model::append_box(&mut part_verts[i], -2.2/16.0, -12.2/16.0, -2.2/16.0, 4.4/16.0, 12.4/16.0, 4.4/16.0, [
                srel!(ox + 4.0, oy + 0.0, 4.0, 4.0), // Up
                srel!(ox + 8.0, oy + 0.0, 4.0, 4.0), // Down
                srel!(ox + 4.0, oy + 4.0, 4.0, 12.0), // North
                srel!(ox + 12.0, oy + 4.0, 4.0, 12.0), // South
                srel!(ox + 0.0, oy + 4.0, 4.0, 12.0), // West
                srel!(ox + 8.0, oy + 4.0, 4.0, 12.0), // East
            ]);
        }

        player_model.model = Some(renderer.model.create_model(
            model::DEFAULT,
            vec![
                head_verts,
                body_verts,
                part_verts[0].clone(),
                part_verts[1].clone(),
                part_verts[2].clone(),
                part_verts[3].clone(),
            ]
        ));
    }

    fn entity_removed(&mut self, m: &mut ecs::Manager, e: ecs::Entity, _: &mut world::World, renderer: &mut render::Renderer) {
        let player_model = m.get_component_mut(e, self.player_model).unwrap();
        if let Some(model) = player_model.model.take() {
            renderer.model.remove_model(model);
        }
    }
}

pub struct PlayerMovement {
    pub flying: bool,
    pub did_touch_ground: bool,
    pub pressed_keys: HashMap<Keycode, bool, BuildHasherDefault<FNVHash>>,
}

impl PlayerMovement {
    pub fn new() -> PlayerMovement {
        PlayerMovement {
            flying: false,
            did_touch_ground: false,
            pressed_keys: HashMap::with_hasher(BuildHasherDefault::default()),
        }
    }

    fn calculate_movement(&self, player_yaw: f64) -> (f64, f64) {
        use std::f64::consts::PI;
        let mut forward = 0.0f64;
        let mut yaw = player_yaw - (PI/2.0);
        if self.is_key_pressed(Keycode::W) || self.is_key_pressed(Keycode::S) {
            forward = 1.0;
            if self.is_key_pressed(Keycode::S) {
                yaw += PI;
            }
        }
        let mut change = 0.0;
        if self.is_key_pressed(Keycode::A) {
            change = (PI / 2.0) / (forward.abs() + 1.0);
        }
        if self.is_key_pressed(Keycode::D) {
            change = -(PI / 2.0) / (forward.abs() + 1.0);
        }
        if self.is_key_pressed(Keycode::A) || self.is_key_pressed(Keycode::D) {
            forward = 1.0;
        }
        if self.is_key_pressed(Keycode::S) {
            yaw -= change;
        } else {
            yaw += change;
        }

        (forward, yaw)
    }

    fn is_key_pressed(&self, key: Keycode) -> bool {
        self.pressed_keys.get(&key).map_or(false, |v| *v)
    }
}

struct MovementHandler {
    filter: ecs::Filter,
    movement: ecs::Key<PlayerMovement>,
    gravity: ecs::Key<Gravity>,
    gamemode: ecs::Key<Gamemode>,
    position: ecs::Key<Position>,
    velocity: ecs::Key<Velocity>,
    game_info: ecs::Key<GameInfo>,
    bounds: ecs::Key<Bounds>,
    rotation: ecs::Key<Rotation>,
}

impl MovementHandler {
    pub fn new(m: &mut ecs::Manager) -> MovementHandler {
        let movement = m.get_key();
        let position = m.get_key();
        let velocity = m.get_key();
        let bounds = m.get_key();
        let rotation = m.get_key();
        MovementHandler {
            filter: ecs::Filter::new()
                .with(movement)
                .with(position)
                .with(velocity)
                .with(bounds)
                .with(rotation),
            movement: movement,
            gravity: m.get_key(),
            gamemode: m.get_key(),
            position: position,
            velocity: velocity,
            game_info: m.get_key(),
            bounds: bounds,
            rotation: rotation,
        }
    }
}

impl ecs::System for MovementHandler {

    fn filter(&self) -> &ecs::Filter {
        &self.filter
    }

    fn update(&mut self, m: &mut ecs::Manager, world: &mut world::World, _: &mut render::Renderer) {
        let world_entity = m.get_world();
        let delta = m.get_component(world_entity, self.game_info).unwrap().delta;
        for e in m.find(&self.filter) {
            let movement = m.get_component_mut(e, self.movement).unwrap();
            if movement.flying && m.get_component(e, self.gravity).is_some() {
                m.remove_component(e, self.gravity);
            } else if !movement.flying && m.get_component(e, self.gravity).is_none() {
                m.add_component(e, self.gravity, Gravity::new());
            }
            let gamemode = m.get_component(e, self.gamemode).unwrap();
            movement.flying |= gamemode.always_fly();

            let position = m.get_component_mut(e, self.position).unwrap();
            let rotation = m.get_component(e, self.rotation).unwrap();
            let velocity = m.get_component_mut(e, self.velocity).unwrap();
            let gravity = m.get_component_mut(e, self.gravity);

            let player_bounds = m.get_component(e, self.bounds).unwrap().bounds;

            let prev_position = position.last_position;

            if world.is_chunk_loaded((position.position.x as i32) >> 4, (position.position.z as i32) >> 4) {
                let (forward, yaw) = movement.calculate_movement(rotation.yaw);
                let mut speed = 4.317 / 60.0;
                if movement.is_key_pressed(Keycode::LShift) {
                    speed = 5.612 / 60.0;
                }
                if movement.flying {
                    speed *= 2.5;

                    if movement.is_key_pressed(Keycode::Space) {
                        position.position.y += speed * delta;
                    }
                    if movement.is_key_pressed(Keycode::LCtrl) {
                        position.position.y -= speed * delta;
                    }
                } else if gravity.as_ref().map_or(false, |v| v.on_ground) {
                    if movement.is_key_pressed(Keycode::Space) {
                        velocity.velocity.y = 0.15;
                    } else {
                        velocity.velocity.y = 0.0;
                    }
                } else {
                    velocity.velocity.y -= 0.01 * delta;
                    if velocity.velocity.y < -0.3 {
                        velocity.velocity.y = -0.3;
                    }
                }
                position.position.x += forward * yaw.cos() * delta * speed;
                position.position.z -= forward * yaw.sin() * delta * speed;
                position.position.y += velocity.velocity.y * delta;
            }

            if !gamemode.noclip() {
                let mut target = position.position;
                position.position.y = position.last_position.y;
                position.position.z = position.last_position.z;

                // We handle each axis separately to allow for a sliding
                // effect when pushing up against walls.

                let (bounds, xhit) = check_collisions(world, position, player_bounds);
                position.position.x = bounds.min.x + 0.3;
                position.last_position.x = position.position.x;

                position.position.z = target.z;
                let (bounds, zhit) = check_collisions(world, position, player_bounds);
                position.position.z = bounds.min.z + 0.3;
                position.last_position.z = position.position.z;

                // Half block jumps
                // Minecraft lets you 'jump' up 0.5 blocks
                // for slabs and stairs (or smaller blocks).
                // Currently we implement this as a teleport to the
                // top of the block if we could move there
                // but this isn't smooth.
                if (xhit || zhit) && gravity.as_ref().map_or(false, |v| v.on_ground) {
                    let mut ox = position.position.x;
                    let mut oz = position.position.z;
                    position.position.x = target.x;
                    position.position.z = target.z;
                    for offset in 1 .. 9 {
                        let mini = player_bounds.add_v(cgmath::Vector3::new(0.0, offset as f64 / 16.0, 0.0));
                        let (_, hit) = check_collisions(world, position, mini);
                        if !hit {
                            target.y += offset as f64 / 16.0;
                            ox = target.x;
                            oz = target.z;
                            break;
                        }
                    }
                    position.position.x = ox;
                    position.position.z = oz;
                }

                position.position.y = target.y;
                let (bounds, yhit) = check_collisions(world, position, player_bounds);
                position.position.y = bounds.min.y;
                position.last_position.y = position.position.y;
                if yhit {
                    velocity.velocity.y = 0.0;
                }

                if let Some(gravity) = gravity {
                    let ground = Aabb3::new(
                        Point3::new(-0.3, -0.05, -0.3),
                        Point3::new(0.3, 0.0, 0.3)
                    );
                    let prev = gravity.on_ground;
                    let (_, hit) = check_collisions(world, position, ground);
                    gravity.on_ground = hit;
                    if !prev && gravity.on_ground {
                        movement.did_touch_ground = true;
                    }
                }
            }

            position.moved = position.position != prev_position;
        }
    }
}


fn check_collisions(world: &world::World, position: &mut Position, bounds: Aabb3<f64>) -> (Aabb3<f64>, bool) {
    let mut bounds = bounds.add_v(position.position);

    let dir = position.position - position.last_position;

    let min_x = (bounds.min.x - 1.0) as i32;
    let min_y = (bounds.min.y - 1.0) as i32;
    let min_z = (bounds.min.z - 1.0) as i32;
    let max_x = (bounds.max.x + 1.0) as i32;
    let max_y = (bounds.max.y + 1.0) as i32;
    let max_z = (bounds.max.z + 1.0) as i32;

    let mut hit = false;
    for y in min_y .. max_y {
        for z in min_z .. max_z {
            for x in min_x .. max_x {
                let block = world.get_block(x, y, z);
                for bb in block.get_collision_boxes() {
                    let bb = bb.add_v(cgmath::Vector3::new(x as f64, y as f64, z as f64));
                    if bb.collides(&bounds) {
                        bounds = bounds.move_out_of(bb, dir);
                        hit = true;
                    }
                }
            }
        }
    }

    (bounds, hit)
}

trait Collidable<T> {
    fn collides(&self, t: &T) -> bool;
    fn move_out_of(self, other: Self, dir: cgmath::Vector3<f64>) -> Self;
}

impl Collidable<Aabb3<f64>> for Aabb3<f64> {
    fn collides(&self, t: &Aabb3<f64>) -> bool {
        !(
            t.min.x >= self.max.x ||
            t.max.x <= self.min.x ||
            t.min.y >= self.max.y ||
            t.max.y <= self.min.y ||
            t.min.z >= self.max.z ||
            t.max.z <= self.min.z
        )
    }

    fn move_out_of(mut self, other: Self, dir: cgmath::Vector3<f64>) -> Self {
        if dir.x != 0.0 {
            if dir.x > 0.0 {
                let ox = self.max.x;
                self.max.x = other.min.x - 0.0001;
                self.min.x += self.max.x - ox;
            } else {
                let ox = self.min.x;
                self.min.x = other.max.x + 0.0001;
                self.max.x += self.min.x - ox;
            }
        }
        if dir.y != 0.0 {
            if dir.y > 0.0 {
                let oy = self.max.y;
                self.max.y = other.min.y - 0.0001;
                self.min.y += self.max.y - oy;
            } else {
                let oy = self.min.y;
                self.min.y = other.max.y + 0.0001;
                self.max.y += self.min.y - oy;
            }
        }
        if dir.z != 0.0 {
            if dir.z > 0.0 {
                let oz = self.max.z;
                self.max.z = other.min.z - 0.0001;
                self.min.z += self.max.z - oz;
            } else {
                let oz = self.min.z;
                self.min.z = other.max.z + 0.0001;
                self.max.z += self.min.z - oz;
            }
        }
        self
    }
}
