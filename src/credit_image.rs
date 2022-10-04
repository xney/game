use bevy::prelude::*;

/// Cycles through credit images
pub struct CreditImagePlugin;

impl Plugin for CreditImagePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActiveImage::default())
            .insert_resource(Timeout {
                timer: Timer::from_seconds(2., true),
            })
            .add_startup_system(spawn_credit_images)
            .add_system(timer_change_credit_image);
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
fn spawn_credit_images(mut commands: Commands, assets: Res<AssetServer>) {


    //Add file names here for credit images
    let credits = vec![
        "hildebrant.png",
        "biggs.png",
        "chakov.png",
        "glazar.png",
        "hopping.png",
        "walsh.png",
        "PlaceHolder.png",
        "PlaceHolder2.png",
    ];

    // TODO: load in actual images
    for i in 0..8u8 {
        let color = i as f32 / 7.;
        commands
            .spawn()
            .insert_bundle(SpriteBundle {
                texture: assets.load(credits[usize::from(i)]),
                transform: Transform {
                    translation: Vec3::from_array([0., 0., 1.]),
                    ..default()
                },
                sprite: Sprite {
                    //color: Color::rgb(1., (1. - color).clamp(0., 1.), color.clamp(0., 1.)),
                    custom_size: Some(Vec2::from_array([1280., 720.])),
                    ..default()
                },
                // all invisible by default, except for the 0th
                visibility: Visibility {
                    is_visible: i == 0,
                },
                ..default()
            })
            .insert(CreditImage { id: i });
        info!("spawned credit image {}", i);
    }
}
