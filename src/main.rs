mod custom_pass;

use custom_pass::{CustomUniformArgs, Quad, RenderCustom};

use amethyst::{
    input::{
        is_close_requested, is_key_down, InputBundle, InputEvent, ScrollDirection, StringBindings,
    },
    prelude::*,
    renderer::{plugins::RenderToWindow, types::DefaultBackend, RenderingBundle},
    utils::application_root_dir,
    window::ScreenDimensions,
    winit::{Event, MouseButton, VirtualKeyCode, WindowEvent},
};

pub struct CustomShaderState {
    panning: bool,
}

impl SimpleState for CustomShaderState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let world = data.world;

        world
            .create_entity()
            .with(Quad {
                points: [[-1.0, -1.0], [-1.0, 1.0], [1.0, -1.0], [1.0, 1.0]],
            })
            .build();
    }

    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        match &event {
            StateEvent::Window(event) => {
                if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
                    Trans::Quit
                } else {
                    if let Event::WindowEvent {
                        event: WindowEvent::Resized(size),
                        ..
                    } = event
                    {
                        let (x, y): (f64, f64) = (*size).into();
                        let aspect_ratio = (x / y) as f32;
                        let mut args = data.world.write_resource::<CustomUniformArgs>();
                        args.aspect_ratio = aspect_ratio;
                    }
                    Trans::None
                }
            }

            StateEvent::Input(InputEvent::MouseWheelMoved(dir)) => {
                let mut scale = data.world.write_resource::<CustomUniformArgs>();
                match dir {
                    ScrollDirection::ScrollUp => (*scale).scale *= 1.1,
                    ScrollDirection::ScrollDown => (*scale).scale /= 1.1,
                    _ => {}
                }
                Trans::None
            }

            StateEvent::Input(InputEvent::MouseButtonPressed(MouseButton::Left)) => {
                self.panning = true;
                Trans::None
            }
            StateEvent::Input(InputEvent::MouseButtonReleased(MouseButton::Left)) => {
                self.panning = false;
                Trans::None
            }
            StateEvent::Input(InputEvent::CursorMoved { delta_x, delta_y }) => {
                if self.panning {
                    let mut args = data.world.write_resource::<CustomUniformArgs>();
                    let dimensions = data.world.read_resource::<ScreenDimensions>();
                    dbg!(*args);

                    let scale = (*args).scale;
                    let offset: &mut [f32; 2] = (*args).offset.as_mut();

                    offset[0] +=
                        scale * dimensions.aspect_ratio() * 4.0 * delta_x / dimensions.width();
                    offset[1] -= scale * 4.0 * delta_y / dimensions.height();
                }
                Trans::None
            }
            StateEvent::Input(InputEvent::KeyPressed {
                key_code: VirtualKeyCode::Up,
                ..
            }) => {
                let mut args = data.world.write_resource::<CustomUniformArgs>();
                args.max_iters = std::cmp::max(10, (args.max_iters as f32 * 1.1) as i32);
                Trans::None
            }
            StateEvent::Input(InputEvent::KeyPressed {
                key_code: VirtualKeyCode::Down,
                ..
            }) => {
                let mut args = data.world.write_resource::<CustomUniformArgs>();
                args.max_iters = std::cmp::max(10, (args.max_iters as f32 / 1.1) as i32);
                Trans::None
            }
            _ => Trans::None,
        }
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;
    let display_config_path = app_root.join("display.ron");
    let assets_dir = app_root.join("assets");

    let game_data = GameDataBuilder::default()
        .with_bundle(InputBundle::<StringBindings>::new())?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)?.with_clear([1.0; 4]),
                )
                .with_plugin(RenderCustom::default()),
        )?;

    let mut game = Application::new(assets_dir, CustomShaderState { panning: false }, game_data)?;

    game.run();
    Ok(())
}
