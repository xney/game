use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::states::client::GameState;

//crate::states;

const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const BUTTON_BACKGROUND_COLOR: Color = Color::rgb(0.5, 0.5, 0.5);
const NORMAL_BUTTON: Color = Color::rgb(0.717, 0.255, 0.055);
const HOVERED_BUTTON: Color = Color::rgb(0.57, 0.20, 0.04);
const HOVERED_PRESSED_BUTTON: Color = Color::rgb(0.478, 0.776, 0.906);
const PRESSED_BUTTON: Color = Color::rgb(0.478, 0.776, 0.906);

#[derive(Component)]
enum MenuButtonAction {
    Start,
    Quit,
}

pub struct MenuPlugin;

#[derive(Component)]
struct OnMainMenuScreen;

#[derive(Component)]
struct SelectedButton;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Menu, main_menu_setup)
            .add_exit_system(GameState::Menu, despawn_screen::<OnMainMenuScreen>)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Menu)
                    .with_system(button_system)
                    .with_system(menu_action)
                    .into(),
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

fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/milky_coffee.ttf");

    let button_style = Style {
        size: Size::new(Val::Px(250.0), Val::Px(65.0)),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
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

fn menu_action(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut commands: Commands,
) {
    for (interaction, menu_button_action) in &interaction_query {
        if *interaction == Interaction::Clicked {
            match menu_button_action {
                MenuButtonAction::Quit => {
                    info!("quit button pressed");
                    std::process::exit(0); // exit immediately
                }
                MenuButtonAction::Start => {
                    info!("start button pressed");
                    commands.insert_resource(NextState(GameState::InGame));
                }
            }
        }
    }
}

fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        // info!("despawning {}", entity.id());
        commands.entity(entity).despawn_recursive();
    }
    // info!("despawning");
}
