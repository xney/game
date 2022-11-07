use bevy::{prelude::*, render::render_resource::Texture};

mod args;
mod credit_image;
mod menu;
mod network;
mod player;
mod procedural_functions;
mod save;
mod states;
mod world;

const TITLE: &str = "The Krusty Krabs";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;

#[derive(Component)]
pub struct CharacterCamera;

fn main() {
   
    let args = args::get_args();
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugin(states::StatePlugin)
        .add_plugin(credit_image::CreditImagePlugin)
        .add_plugin(menu::MenuPlugin)
        .insert_resource(WindowDescriptor {
            title: String::from(TITLE),
            width: WIN_W,
            height: WIN_H,
            ..default()
        })
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_startup_system(|mut c: Commands| {
            c.spawn_bundle(Camera2dBundle::default())
                .insert(CharacterCamera);
        })
        .add_startup_system(setup_background)
        .add_plugin(world::WorldPlugin)
        .add_plugin(player::PlayerPlugin)
        .add_plugin(save::SaveLoadPlugin);

    match args {
        args::GameArgs::Server(s) => {
            app.add_plugin(network::server::ServerPlugin {
                port: s.port,
                filename: s.filename,
            });
            ()
        }
        args::GameArgs::Client(c) => {
            app.add_plugin(network::client::ClientPlugin {
                server_address: c.server_ip,
                server_port: c.port,
            });
            ()
        }
        args::GameArgs::None => {
            warn!("No command line arguments provided, not adding any network plugin!");
            ()
        }
    }

    app.run();
}

fn setup_background(
    mut c: Commands,
    asset_server: Res<AssetServer>
) {

    
    c.spawn_bundle(SpriteBundle {
        texture: asset_server.load("Background1.png"),
        transform: Transform{
            scale: Vec3::from_array([8.,8.,0.]),
            ..default()
        },
        ..default()
    });
}