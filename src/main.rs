#![warn(clippy::nursery, clippy::pedantic)]
#![allow(
    clippy::needless_pass_by_value,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
use std::cmp::Ordering;

use bevy::{
    prelude::*,
    window::{close_on_esc, WindowMode}, gizmos,
};
use rand::{distributions::Uniform, prelude::Distribution, random, thread_rng};

#[derive(Resource)]
struct Settings {
    speed: f32,
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

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    let mut rng = thread_rng();
    for _ in 0..1000 {
        let r#type = match Uniform::new(0, 3).sample(&mut rng) {
            0 => Shape::Scissors,
            1 => Shape::Paper,
            2 => Shape::Rock,
            _ => unreachable!(),
        };
        commands.spawn((
            r#type,
            Text2dBundle {
                transform: Transform::from_translation({
                    let theta = (Uniform::new(0., 360.).sample(&mut rng) as f32).to_radians();
                    let radius = Uniform::new(100., 800.).sample(&mut rng);
                    Vec3::new(radius * theta.cos(), radius * theta.sin(), 0.0)
                }),
                text_anchor: bevy::sprite::Anchor::Center,
                text: Text::from_section(
                    match r#type {
                        Shape::Scissors => "s",
                        Shape::Paper => "p",
                        Shape::Rock => "r",
                    },
                    TextStyle {
                        font: asset_server.load("SF-Pro.ttf"),
                        font_size: 20.0,
                        color: match r#type {
                            Shape::Scissors => Color::RED,
                            Shape::Paper => Color::WHITE,
                            Shape::Rock => Color::GRAY,
                        },
                    },
                )
                .with_alignment(TextAlignment::Center),
                ..default()
            },
        ));
    }
}

fn simulate(
    mut entities: Query<(&mut Shape, &mut Transform, &mut Text)>,
    time: Res<Time>,
    settings: Res<Settings>,
    mut gizmos: Gizmos,
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
        let this_translation = this_transform.translation;
        // gizmos.ray(
        //     this_translation,
        //     (closest.1.translation - this_translation).normalize()
        //         * time.delta_seconds()
        //         * settings.speed
        //         * match this_shape.cmp(&closest.0) {
        //             Ordering::Greater => 1.,
        //             Ordering::Less => -1.,
        //             Ordering::Equal => unreachable!(),
        //         }
        //         * 40.,
        //     Color::GREEN,
        // );
        this_transform.into_inner().translation = (this_translation
            + (closest.1.translation - this_translation).normalize()
                * time.delta_seconds()
                * settings.speed
                * match this_shape.cmp(&closest.0) {
                    Ordering::Greater => 1.,
                    Ordering::Less => -1.,
                    Ordering::Equal => unreachable!(),
                }
                * random::<f32>()
            + {
                let average = {
                    let close_entities = copy.iter().filter_map(|(_, transform)| {
                        if transform.translation.distance(this_translation) < 20. {
                            Some(transform.translation)
                        } else {
                            None
                        }
                    });
                    close_entities.clone().sum::<Vec3>() / close_entities.count() as f32
                };
                (this_translation - average).normalize_or_zero() * time.delta_seconds() * 60.
            }
        )
        .clamp_length_max(800.);
    }

    let copy: Vec<_> = entities
        .iter()
        .map(|(shape, transform, text)| (*shape, *transform, text.clone()))
        .collect();
    for (this_shape, this_transform, this_text) in copy {
        for (other_shape, other_transform, mut other_text) in &mut entities {
            if this_shape == *other_shape {
                continue;
            }
            if this_transform
                .translation
                .distance(other_transform.translation)
                < 20.
                && this_shape > *other_shape
            {
                *other_shape.into_inner() = this_shape;
                other_text.sections[0].value = this_text.sections[0].value.clone();
                other_text.sections[0].style.color = this_text.sections[0].style.color;
            }
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Settings { speed: 100. })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: WindowMode::BorderlessFullscreen,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, startup)
        .add_systems(Update, (simulate, close_on_esc))
        .run();
}
