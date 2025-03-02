use std::{
    ffi::CStr,
    mem::transmute,
};

use gtk::{
    Application,
    ApplicationWindow,
    prelude::*,
};
use smithay::backend::{
    drm::DrmNode,
    egl::{
        ffi::{
            self,
            egl::{
                DEVICE_EXT,
                QueryDisplayAttribEXT,
                types::EGLAttrib,
            },
        },
        wrap_egl_call_ptr,
    },
};
mod widget;

const APP_ID: &str = "org.gtk_rs.HelloWorld2";

fn main() {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to the "activate" signal to build the UI
    app.connect_activate(build_ui);

    // Run the application
    app.run();
}

fn build_ui(app: &Application) {
    // Create a window and set the title
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .build();

    let widget = widget::SWidget::new();
    window.set_child(Some(&widget));

    let display = gdk::Display::default()
        .unwrap()
        .dynamic_cast::<gdk4_wayland::WaylandDisplay>()
        .unwrap();
    let dmabuf_formats = display.dmabuf_formats();

    let mut formats = Vec::new();
    for i in 0..dmabuf_formats.n_formats() - 1 {
        let format = dmabuf_formats.format(i);
        formats.push(smithay::backend::allocator::Format {
            code: unsafe { transmute::<u32, smithay::backend::allocator::Fourcc>(format.0) },
            modifier: format.1.into(),
        });
    }

    let egl_display = display.egl_display().unwrap();
    ffi::make_sure_egl_is_loaded().unwrap();

    let mut device: EGLAttrib = 0;
    unsafe {
        QueryDisplayAttribEXT(
            egl_display.as_ptr(),
            DEVICE_EXT as i32,
            &mut device as *mut _,
        )
    };

    let raw_path = wrap_egl_call_ptr(|| unsafe {
        ffi::egl::QueryDeviceStringEXT(
            device as *mut _,
            ffi::egl::DRM_RENDER_NODE_FILE_EXT as ffi::egl::types::EGLint,
        )
    })
    .unwrap();
    let device_path = unsafe { CStr::from_ptr(raw_path) }
        .to_str()
        .expect("Non-UTF8 device path name");
    let drm_node = DrmNode::from_path(device_path).unwrap();

    std::thread::spawn(move || {
        yozora::compositor(drm_node.dev_id(), formats).unwrap();
    });

    // Present window
    window.present();
}
