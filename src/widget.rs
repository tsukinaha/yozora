use glib::Object;
use gtk::glib;

pub mod imp {
    use std::{
        cell::RefCell,
        os::fd::AsRawFd,
    };

    use gdk::{
        DmabufTexture,
        Texture,
    };

    use gtk::{
        gdk,
        glib,
        prelude::*,
        subclass::prelude::*,
    };
    use smithay::backend::allocator::Buffer;
    use yozora::{
        Builder,
        DMABUF_IMPORTED,
    };

    // Object holding the state
    #[derive(Default)]
    pub struct SWidget {
        pub dmabuf: RefCell<Option<Texture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SWidget {
        const NAME: &'static str = "SWidget";
        type Type = super::SWidget;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for SWidget {
        fn constructed(&self) {
            self.parent_constructed();

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    while let Ok(dmabuf) = DMABUF_IMPORTED.rx.recv_async().await {
                        let mut builder = DmabufTexture::builder()
                            .width(dmabuf.size().w as u32)
                            .height(dmabuf.size().h as u32)
                            .n_planes(dmabuf.num_planes() as u32)
                            .fourcc(dmabuf.format().code as u32);

                        for (plane_idx, (handle, offset)) in
                            dmabuf.handles().zip(dmabuf.offsets()).enumerate()
                        {
                            builder = builder.fd(plane_idx as u32, handle.as_raw_fd());
                            builder = builder.offset(plane_idx as u32, offset);
                        }

                        let dmabuf_texture = unsafe { builder.build() };
                        println!("Received dmabuf texture result: {:?}", dmabuf_texture);
                        imp.dmabuf.replace(dmabuf_texture.ok());
                    }
                }
            ));
        }
    }

    impl WidgetImpl for SWidget {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            self.parent_snapshot(snapshot);
            let width = self.obj().width();
            let height = self.obj().height();
            if let Some(dmabuf) = self.dmabuf.borrow().as_ref() {
                dmabuf.snapshot(snapshot, width as f64, height as f64);
            }
        }
    }

    impl SWidget {}
}

glib::wrapper! {
    pub struct SWidget(ObjectSubclass<imp::SWidget>)
        @extends gtk::Widget;
}

impl Default for SWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl SWidget {
    pub fn new() -> Self {
        Object::new()
    }
}
