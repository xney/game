use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::states;

/// Cycles through credit images
pub struct CreditImagePlugin;

impl Plugin for CreditImagePlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(states::client::GameState::Credits, init_credits)
            .add_system(timer_change_credit_image.run_in_state(states::client::GameState::Credits))
            .add_exit_system(states::client::GameState::Credits, destroy_credits);
    }
}

#[derive(Component)]
struct CreditImage {
    id: u8,
}

/// Keeps track of which credit image should be visible; should be a resource
#[derive(Default)]
struct ActiveImage {
    id: u8,
}

/// Simple timeout; should be a resource
struct Timeout {
    timer: Timer,
}

/// Changes active credit image on timeout
fn timer_change_credit_image(
    time: Res<Time>,
    mut timeout: ResMut<Timeout>,
    mut active: ResMut<ActiveImage>,
    mut query: Query<(&mut Visibility, &CreditImage)>,
) {
    // update our timer
    timeout.timer.tick(time.delta());

    if timeout.timer.just_finished() {
        // cycle active id
        active.id = (active.id + 1) % 8;
        info!("changing active image to {}", active.id);

        // set active to visible and all others to invisible
        for (mut visibility, credit) in query.iter_mut() {
            if credit.id == active.id {
                visibility.is_visible = true;
            } else {
                visibility.is_visible = false;
            }
        }
    }
}

/// Spawns all credit images
fn init_credits(mut commands: Commands, assets: Res<AssetServer>) {
    info!("initializing credits");
    //Add file names here for credit images
    let credits: Vec<&str> = vec![
        "hildebrandt.png",
        "biggs.png",
        "chakov.png",
        "glazar.png",
        "hopping.png",
        "walsh.png",
        "haskovec.png",
        "thompson.png",
    ];

    for (i, credit) in credits.iter().enumerate() {
        commands
            .spawn()
            .insert_bundle(SpriteBundle {
                texture: assets.load(*credit),
                transform: Transform {
                    translation: Vec3::from_array([0., 0., 1.]),
                    ..default()
                },
                sprite: Sprite {
                    custom_size: Some(Vec2::from_array([1280., 720.])),
                    ..default()
                },
                // all invisible by default, except for the 0th
                visibility: Visibility { is_visible: i == 0 },
                ..default()
            })
            .insert(CreditImage { id: i as u8 });
        info!("spawned credit image {}", credit);
    }

    // create necessary resources
    commands.insert_resource(ActiveImage::default());
    commands.insert_resource(Timeout {
        timer: Timer::from_seconds(2., true),
    });
}

fn destroy_credits(mut commands: Commands, query: Query<Entity, With<CreditImage>>) {
    info!("destroying credits");
    // remove all credit image entities
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<ActiveImage>();
    commands.remove_resource::<Timeout>();
}
