use bevy::prelude::*;

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

#[derive(Default)]
struct ActiveImage {
    id: u8,
}

struct Timeout {
    timer: Timer,
}

fn timer_change_credit_image(
    time: Res<Time>,
    mut timeout: ResMut<Timeout>,
    mut active: ResMut<ActiveImage>,
    mut query: Query<(&mut Visibility, &CreditImage)>,
) {
    timeout.timer.tick(time.delta());

    if timeout.timer.just_finished() {
        active.id += 1;
        active.id %= 8;
        info!("changing active image to {}", active.id);
        for (mut visibility, credit) in query.iter_mut() {
            if credit.id == active.id {
                visibility.is_visible = true;
            } else {
                visibility.is_visible = false;
            }
        }
    }
}

fn spawn_credit_images(mut commands: Commands) {
    // TODO: load in actual images
    for i in 0..8u8 {
        let color = i as f32 / 7.;
        commands
            .spawn()
            .insert_bundle(SpriteBundle {
                transform: Transform {
                    translation: Vec3::from_array([0., 0., 1.]),
                    ..default()
                },
                sprite: Sprite {
                    color: Color::rgb(1., (1. - color).clamp(0., 1.), color),
                    custom_size: Some(Vec2::from_array([1280., 720.])),
                    ..default()
                },
                visibility: Visibility { is_visible: false },
                ..default()
            })
            .insert(CreditImage { id: i });
        info!("spawned credit image {}", i);
    }
}
