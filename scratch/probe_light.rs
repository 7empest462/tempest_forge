use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    // Probe if AmbientLight exists and if it can be inserted
    app.insert_resource(AmbientLight::default());
}
