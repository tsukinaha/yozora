use std::sync::Arc;

use smithay::{
    backend::allocator::Format,
    delegate_compositor,
    delegate_output,
    delegate_seat,
    delegate_shm,
    delegate_viewporter,
    delegate_xdg_shell,
    input::{
        Seat,
        SeatHandler,
        SeatState,
    },
    output::{
        Output,
        PhysicalProperties,
        Subpixel,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::Display,
    },
    utils::Serial,
    wayland::{
        output::OutputHandler,
        shell::xdg::{
            PopupSurface,
            PositionerState,
            ToplevelSurface,
            XdgShellHandler,
            XdgShellState,
        },
        shm::{
            ShmHandler,
            ShmState,
        },
        viewporter::ViewporterState,
    },
};

use smithay::wayland::compositor::{
    CompositorClientState,
    CompositorHandler,
    CompositorState,
};

use wayland_server::{
    Client,
    ListeningSocket,
    backend::{
        ClientData,
        ClientId,
        DisconnectReason,
    },
    protocol::{
        wl_seat,
        wl_shm,
        wl_surface::WlSurface,
    },
};

use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    delegate_dmabuf,
    wayland::{
        buffer::BufferHandler,
        dmabuf::{
            DmabufFeedback,
            DmabufFeedbackBuilder,
            DmabufGlobal,
            DmabufHandler,
            DmabufState,
            ImportNotifier,
        },
    },
};

use crate::channel::DMABUF_IMPORTED;

struct State {
    compositor_state: CompositorState,
    dmabuf_state: DmabufState,
    xdg_shell_state: XdgShellState,
    seat_state: SeatState<Self>,
    _viewporter_state: ViewporterState,
    shm_state: smithay::wayland::shm::ShmState,
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, _surface: &WlSurface) {}
}

// Smithay's "DmabufHandler" also requires the buffer management utilities, you need to implement
// "BufferHandler".
impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &wayland_server::protocol::wl_buffer::WlBuffer) {
        // All renderers can handle buffer destruction at this point. Some parts of window
        // management may also use this function.
        //
        // If you need to mark a dmabuf elsewhere in your state as destroyed, you use the
        // "get_dmabuf" function defined in this module to access the dmabuf associated the
        // "Buffer".
    }
}

impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_imported(
        &mut self, _global: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier,
    ) {
        // Here you should import the dmabuf into your renderer.
        //
        // The notifier is used to communicate whether import was successful. In this example we
        // call successful to notify the client import was successful.
        DMABUF_IMPORTED.tx.send(dmabuf).unwrap();
        notifier.successful::<State>().unwrap();
    }

    fn new_surface_feedback(
        &mut self, _surface: &WlSurface, _global: &DmabufGlobal,
    ) -> Option<DmabufFeedback> {
        // Here you can override the initial feedback sent to a client requesting feedback for a
        // specific surface. Returning `None` instructs the global to return the default
        // feedback to the client which is also the default implementation for this function
        // when not overridden
        None
    }
}

// Delegate dmabuf handling for State to DmabufState.
delegate_dmabuf!(State);

pub fn compositor(
    dev: libc::dev_t, formats: impl IntoIterator<Item = Format>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut display: Display<State> = Display::new()?;
    let dh = display.handle();

    let compositor_state = CompositorState::new::<State>(&dh);

    // First a DmabufState must be created. This type is used to create some "DmabufGlobal"s
    let mut dmabuf_state = DmabufState::new();

    let xdg_shell_state = XdgShellState::new::<State>(&dh);
    let _viewporter_state = ViewporterState::new::<State>(&dh);

    let seat_state = SeatState::new();

    let default_feedback = DmabufFeedbackBuilder::new(dev, formats).build().unwrap();

    // And create the dmabuf global.
    let _dmabuf_global =
        dmabuf_state.create_global_with_default_feedback::<State>(&dh, &default_feedback);

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Toy".into(),
            model: "Winit".into(),
        },
    );

    output.create_global::<State>(&dh);
    let shm_state = ShmState::new::<State>(
        &dh,
        vec![wl_shm::Format::Xbgr8888, wl_shm::Format::Abgr8888],
    );
    // ...identify primary render node and load dmabuf formats supported for rendering...

    let mut state = State {
        compositor_state,
        dmabuf_state,
        xdg_shell_state,
        seat_state,
        _viewporter_state,
        shm_state,
    };

    let listener = ListeningSocket::bind("wayland-5").unwrap();

    let mut clients = Vec::new();

    loop {
        if let Some(stream) = listener.accept().unwrap() {
            println!("Got a client: {:?}", stream);

            let client = display
                .handle()
                .insert_client(stream, Arc::new(ClientState::default()))
                .unwrap();
            clients.push(client);
        }

        display.dispatch_clients(&mut state)?;
        display.flush_clients()?;
    }
}

#[derive(Default)]
struct ClientState {
    compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        println!("initialized");
    }

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        println!("disconnected");
    }
}

impl AsMut<CompositorState> for State {
    fn as_mut(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }
}

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // Handle popup creation here
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        // Handle popup grab here
    }

    fn reposition_request(
        &mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32,
    ) {
        // Handle popup reposition here
    }
}

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
    fn cursor_image(
        &mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
}

impl OutputHandler for State {}

impl ShmHandler for State {
    fn shm_state(&self) -> &smithay::wayland::shm::ShmState {
        &self.shm_state
    }
}

delegate_compositor!(State);
delegate_xdg_shell!(State);
delegate_seat!(State);
delegate_output!(State);
delegate_viewporter!(State);
delegate_shm!(State);
