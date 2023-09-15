#![warn(clippy::nursery, clippy::pedantic)]
#![allow(
    clippy::needless_pass_by_value,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
use std::{cmp::Ordering, ops::Range};

use bevy::{
    prelude::*,
    window::{close_on_esc, WindowMode},
};
use rand::{distributions::Uniform, prelude::Distribution, thread_rng};

#[derive(Resource)]
struct Settings {
    speed: f32,
    number: usize,
    textures: [String; 3],
    start_range: Range<f32>,
    texture_size: f32,
    collision_range: f32,
    collision_speed: f32,
    max_size: f32,
    hit_size: f32,
}

#[derive(Component, Clone, Copy)]
enum Shape {
    Scissors,
    Paper,
    Rock,
}

impl PartialOrd for Shape {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Shape {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Scissors, Self::Paper)
            | (Self::Paper, Self::Rock)
            | (Self::Rock, Self::Scissors) => Ordering::Greater,
            (Self::Scissors, Self::Rock)
            | (Self::Paper, Self::Scissors)
            | (Self::Rock, Self::Paper) => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
}

impl PartialEq for Shape {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl Eq for Shape {}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_entities(mut commands: Commands, asset_server: Res<AssetServer>, settings: Res<Settings>) {
    let mut rng = thread_rng();
    for _ in 0..settings.number {
        let r#type = match Uniform::new(0, 3).sample(&mut rng) {
            0 => Shape::Scissors,
            1 => Shape::Paper,
            2 => Shape::Rock,
            _ => unreachable!(),
        };
        commands.spawn((
            r#type,
            SpriteBundle {
                transform: Transform::from_translation({
                    let theta = (Uniform::new(0., 360.).sample(&mut rng) as f32).to_radians();
                    let radius = Uniform::from(settings.start_range.clone()).sample(&mut rng);
                    Vec3::new(radius * theta.cos(), radius * theta.sin(), 0.0)
                })
                .with_scale(Vec3::splat(settings.texture_size)),
                texture: asset_server.load(format!(
                    "{}.png",
                    match r#type {
                        Shape::Scissors => settings.textures[0].clone(),
                        Shape::Paper => settings.textures[1].clone(),
                        Shape::Rock => settings.textures[2].clone(),
                    }
                )),
                ..default()
            },
        ));
    }
}

fn movement(
    mut entities: Query<(&mut Shape, &mut Transform, &mut Handle<Image>)>,
    time: Res<Time>,
    settings: Res<Settings>,
) {
    let copy: Vec<_> = entities
        .iter()
        .map(|(shape, transform, _)| (*shape, *transform))
        .collect();
    for (this_shape, this_transform, _) in &mut entities {
        let Some(closest) = copy
            .iter()
            .filter(|(shape, _)| shape != this_shape.as_ref())
            .min_by(|(_, first_transform), (_, second_transform)| {
                first_transform
                    .translation
                    .distance(this_transform.translation)
                    .total_cmp(
                        &second_transform
                            .translation
                            .distance(this_transform.translation),
                    )
            })
        else {
            continue;
        };
        let this_transform = this_transform.into_inner();
        let movement = (closest.1.translation - this_transform.translation).normalize()
            * settings.speed
            * match this_shape.cmp(&closest.0) {
                Ordering::Greater => 1.,
                Ordering::Less => -1.,
                Ordering::Equal => unreachable!(),
            };
        let collision = {
            let average = {
                let close_entities = copy.iter().filter_map(|(_, transform)| {
                    if transform.translation.distance(this_transform.translation)
                        < settings.collision_range
                    {
                        Some(transform.translation)
                    } else {
                        None
                    }
                });
                close_entities.clone().sum::<Vec3>() / close_entities.count() as f32
            };
            (this_transform.translation - average).normalize_or_zero() * settings.collision_speed
        };
        let delta = movement + collision;
        this_transform.translation = this_transform.translation.lerp(
            (this_transform.translation + delta * time.delta_seconds())
                .clamp_length_max(settings.max_size),
            time.delta_seconds() * 40.,
        );
    }
}

fn check(
    mut entities: Query<(&mut Shape, &mut Transform, &mut Handle<Image>)>,
    settings: Res<Settings>,
) {
    let copy: Vec<_> = entities
        .iter()
        .map(|(shape, transform, image)| (*shape, *transform, image.clone()))
        .collect();
    for (this_shape, this_transform, this_image) in copy {
        for (other_shape, other_transform, other_image) in &mut entities {
            if this_shape == *other_shape {
                continue;
            }
            if this_transform
                .translation
                .distance(other_transform.translation)
                < settings.hit_size
                && this_shape > *other_shape
            {
                *other_shape.into_inner() = this_shape;
                *other_image.into_inner() = this_image.clone();
            }
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Settings {
            speed: 100.,
            number: 1000,
            textures: ["✂️".to_string(), "📄".to_string(), "🪨".to_string()],
            start_range: 200. ..700.,
            texture_size: 0.1,
            collision_range: 20.,
            collision_speed: 60.,
            max_size: 800.,
            hit_size: 20.,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: WindowMode::BorderlessFullscreen,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (spawn_entities, spawn_camera))
        .add_systems(Update, (movement, close_on_esc, check))
        .run();
}
