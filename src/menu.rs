use bevy::{app::AppExit, ecs::system::Resource, prelude::*};

use crate::states::client::GameState;

//crate::states;

const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const BUTTON_BACKGROUND_COLOR: Color = Color::rgb(0.5, 0.5, 0.5);
const NORMAL_BUTTON: Color = Color::rgb(0.717, 0.255, 0.055);
const HOVERED_BUTTON: Color = Color::rgb(0.57, 0.20, 0.04);
const HOVERED_PRESSED_BUTTON: Color = Color::rgb(0.478, 0.776, 0.906);
const PRESSED_BUTTON: Color = Color::rgb(0.478, 0.776, 0.906);

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum MenuState {
    Main,
    ServerSelect,
    FileSelect,
    Settings,
    Disabled,
}

#[derive(Component)]
enum MenuButtonAction {
    Start,
    Settings,
    ServerSelect,
    FileSelect,
    Save,
    Load,
    BackToMain,
    Quit,
}

pub struct MenuPlugin;

#[derive(Component)]
struct OnMainMenuScreen;

#[derive(Component)]
struct OnSettingsMenuScreen;

#[derive(Component)]
struct OnServerSelectMenuScreen;

#[derive(Component)]
struct OnFileSelectMenuScreen;

#[derive(Component)]
struct SelectedButton;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(MenuState::Main)
            .add_system_set(SystemSet::on_enter(GameState::Menu).with_system(menu_setup))
            .add_system_set(
                SystemSet::on_exit(GameState::Menu).with_system(despawn_screen::<OnMainMenuScreen>),
            )
            .add_system_set(SystemSet::on_enter(MenuState::Main).with_system(main_menu_setup))
            .add_system_set(
                SystemSet::on_exit(MenuState::Main).with_system(despawn_screen::<OnMainMenuScreen>),
            )
            .add_system_set(
                SystemSet::on_enter(MenuState::Settings).with_system(settings_menu_setup),
            )
            .add_system_set(
                SystemSet::on_exit(MenuState::Settings)
                    .with_system(despawn_screen::<OnSettingsMenuScreen>),
            )
            .add_system_set(
                SystemSet::on_enter(MenuState::ServerSelect).with_system(server_select_setup),
            )
            .add_system_set(
                SystemSet::on_exit(MenuState::ServerSelect)
                    .with_system(despawn_screen::<OnServerSelectMenuScreen>),
            )
            .add_system_set(
                SystemSet::on_enter(MenuState::FileSelect).with_system(file_select_setup),
            )
            .add_system_set(
                SystemSet::on_exit(MenuState::FileSelect)
                    .with_system(despawn_screen::<OnFileSelectMenuScreen>),
            )
            .add_system_set(
                SystemSet::on_update(MenuState::Main)
                    .with_system(button_system)
                    .with_system(menu_action),
            )
            .add_system_set(
                SystemSet::on_update(MenuState::Settings)
                    .with_system(button_system)
                    .with_system(menu_action),
            )
            .add_system_set(
                SystemSet::on_update(MenuState::ServerSelect)
                    .with_system(button_system)
                    .with_system(menu_action),
            )
            .add_system_set(
                SystemSet::on_update(MenuState::FileSelect)
                    .with_system(button_system)
                    .with_system(menu_action),
            );
    }
}

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut UiColor, Option<&SelectedButton>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, selected) in &mut interaction_query {
        *color = match (*interaction, selected) {
            (Interaction::Clicked, _) | (Interaction::None, Some(_)) => {
                bevy::prelude::UiColor(PRESSED_BUTTON)
            }
            (Interaction::None, None) => bevy::prelude::UiColor(NORMAL_BUTTON),
            (Interaction::Hovered, None) => bevy::prelude::UiColor(HOVERED_BUTTON),
            (Interaction::Hovered, Some(_)) => bevy::prelude::UiColor(HOVERED_PRESSED_BUTTON),
        };
    }
}

// fn setting_button<T: Resource + Component + PartialEq + Copy>(
//     interaction_query: Query<(&Interaction, &T, Entity), (Changed<Interaction>, With<Button>)>,
//     mut selected_query: Query<(Entity, &mut UiColor), With<SelectedButton>>,
//     mut commands: Commands,
//     mut setting: ResMut<T>,
// ) {
//     for (interaction, button_setting, entity) in &interaction_query {
//         if *interaction == Interaction::Clicked && *setting != *button_setting {
//             let (previous_button, mut previous_color) = selected_query.single_mut();
//             *previous_color = NORMAL_BUTTON.into();
//             commands.entity(previous_button).remove::<SelectedButton>();
//             commands.entity(entity).insert(SelectedButton);
//             *setting = *button_setting;
//         }
//     }
// }

//TODO in game menu
fn menu_setup(mut menu_state: ResMut<State<MenuState>>) {
    let _ = menu_state.set(MenuState::Main);
}

fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/milky_coffee.ttf");

    let button_style = Style {
        size: Size::new(Val::Px(250.0), Val::Px(65.0)),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    // let button_icon_style = Style {
    //     size: Size::new(Val::Px(30.0), Val::Auto),
    //     position_type: PositionType::Absolute,
    //     position: UiRect {
    //         left: Val::Px(10.0),
    //         right: Val::Auto,
    //         top: Val::Auto,
    //         bottom: Val::Auto,
    //     },
    //     ..default()
    // };
    let button_text_style = TextStyle {
        font: font.clone(),
        font_size: 40.0,
        color: TEXT_COLOR,
    };

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                margin: UiRect::all(Val::Auto),
                flex_direction: FlexDirection::ColumnReverse,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            color: bevy::prelude::UiColor(BUTTON_BACKGROUND_COLOR),
            ..default()
        })
        .insert(OnMainMenuScreen)
        .with_children(|parent| {
            // Display the game name
            parent.spawn_bundle(
                TextBundle::from_section(
                    "Krusty Krabs",
                    TextStyle {
                        font: font.clone(),
                        font_size: 80.0,
                        color: TEXT_COLOR,
                    },
                )
                .with_style(Style {
                    margin: UiRect::all(Val::Px(50.0)),
                    ..default()
                }),
            );

            parent
                .spawn_bundle(ButtonBundle {
                    style: button_style.clone(),
                    color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .insert(MenuButtonAction::Start)
                .with_children(|parent| {
                    parent
                        .spawn_bundle(TextBundle::from_section("Start", button_text_style.clone()));
                });
            // parent
            //     .spawn_bundle(ButtonBundle {
            //         style: button_style.clone(),
            //         color: NORMAL_BUTTON.into(),
            //         ..default()
            //     })
            //     .insert(MenuButtonAction::ServerSelect)
            //     .with_children(|parent| {
            //         parent.spawn_bundle(TextBundle::from_section(
            //             "Server Select",
            //             button_text_style.clone(),
            //         ));
            //     });
            // parent
            //     .spawn_bundle(ButtonBundle {
            //         style: button_style.clone(),
            //         color: NORMAL_BUTTON.into(),
            //         ..default()
            //     })
            //     .insert(MenuButtonAction::Settings)
            //     .with_children(|parent| {
            //         parent.spawn_bundle(TextBundle::from_section(
            //             "Settings",
            //             button_text_style.clone(),
            //         ));
            //     });
            // parent
            //     .spawn_bundle(ButtonBundle {
            //         style: button_style.clone(),
            //         color: NORMAL_BUTTON.into(),
            //         ..default()
            //     })
            //     .insert(MenuButtonAction::FileSelect)
            //     .with_children(|parent| {
            //         parent.spawn_bundle(TextBundle::from_section(
            //             "File Select",
            //             button_text_style.clone(),
            //         ));
            //     });
            parent
                .spawn_bundle(ButtonBundle {
                    style: button_style,
                    color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .insert(MenuButtonAction::Quit)
                .with_children(|parent| {
                    parent
                        .spawn_bundle(TextBundle::from_section("Quit", button_text_style.clone()));
                });
        });
    info!("finished main menu setup");
}

fn settings_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let button_style = Style {
        size: Size::new(Val::Px(200.0), Val::Px(65.0)),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    let button_text_style = TextStyle {
        font: asset_server.load("fonts/milky_coffee.ttf"),
        font_size: 40.0,
        color: TEXT_COLOR,
    };

    commands.spawn_bundle(NodeBundle {
        style: Style {
            margin: UiRect::all(Val::Auto),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        color: bevy::prelude::UiColor(BUTTON_BACKGROUND_COLOR),
        ..default()
    });

    commands
        .spawn_bundle(ButtonBundle {
            style: button_style.clone(),
            color: NORMAL_BUTTON.into(),
            ..default()
        })
        .insert(MenuButtonAction::BackToMain)
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle::from_section(
                "Back To Main Menu",
                button_text_style.clone(),
            ));
        });
}

fn server_select_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let button_style = Style {
        size: Size::new(Val::Px(200.0), Val::Px(65.0)),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    let button_text_style = TextStyle {
        font: asset_server.load("fonts/milky_coffee.ttf"),
        font_size: 40.0,
        color: TEXT_COLOR,
    };

    commands.spawn_bundle(NodeBundle {
        style: Style {
            margin: UiRect::all(Val::Auto),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        color: bevy::prelude::UiColor(BUTTON_BACKGROUND_COLOR),
        ..default()
    });

    commands
        .spawn_bundle(ButtonBundle {
            style: button_style.clone(),
            color: NORMAL_BUTTON.into(),
            ..default()
        })
        .insert(MenuButtonAction::BackToMain)
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle::from_section(
                "Back To Main Menu",
                button_text_style.clone(),
            ));
        });
}

fn file_select_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let button_style = Style {
        size: Size::new(Val::Px(200.0), Val::Px(65.0)),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    let button_text_style = TextStyle {
        font: asset_server.load("fonts/milky_coffee.ttf"),
        font_size: 40.0,
        color: TEXT_COLOR,
    };

    commands.spawn_bundle(NodeBundle {
        style: Style {
            margin: UiRect::all(Val::Auto),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        color: bevy::prelude::UiColor(BUTTON_BACKGROUND_COLOR),
        ..default()
    });

    commands
        .spawn_bundle(ButtonBundle {
            style: button_style.clone(),
            color: NORMAL_BUTTON.into(),
            ..default()
        })
        .insert(MenuButtonAction::BackToMain)
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle::from_section(
                "Back To Main Menu",
                button_text_style.clone(),
            ));
        });
}

fn menu_action(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut exit: EventWriter<AppExit>,
    mut menu_state: ResMut<State<MenuState>>,
    mut game_state: ResMut<State<GameState>>,
) {
    for (interaction, menu_button_action) in &interaction_query {
        if *interaction == Interaction::Clicked {
            match menu_button_action {
                MenuButtonAction::Quit => {
                    exit.send(AppExit);
                }
                MenuButtonAction::Start => {
                    game_state.set(GameState::InGame).unwrap();
                    menu_state.set(MenuState::Disabled).unwrap();
                }
                MenuButtonAction::Settings => {
                    menu_state.set(MenuState::Settings).unwrap();
                }
                MenuButtonAction::ServerSelect => {
                    menu_state.set(MenuState::ServerSelect).unwrap();
                }
                MenuButtonAction::FileSelect => {
                    menu_state.set(MenuState::FileSelect);
                }
                MenuButtonAction::Save => todo!(),
                MenuButtonAction::Load => todo!(),
                MenuButtonAction::BackToMain => {
                    menu_state.set(MenuState::Main);
                }
            }
        }
    }
}

fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        info!("despawning {}", entity.id());
        commands.entity(entity).despawn_recursive();
    }
    info!("despawning");
}
