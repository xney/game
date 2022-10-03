use bevy::prelude::*;

fn main() {
    App::new().add_system(hello).run();
}

fn hello() {
    println!("hello world");
}
