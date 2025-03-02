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
        TextureBuilder,
    };

    // Object holding the state
    #[derive(Default)]
    pub struct SWidget {
        pub dmabuf: RefCell<TextureBuilder>,
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

                        for (plane_idx, ((handle, offset), stride)) in dmabuf
                            .handles()
                            .zip(dmabuf.offsets())
                            .zip(dmabuf.strides())
                            .enumerate()
                        {
                            builder = builder
                                .fd(plane_idx as u32, handle.as_raw_fd())
                                .offset(plane_idx as u32, offset)
                                .stride(plane_idx as u32, stride);
                        }

                        imp.dmabuf.replace(builder);
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

            if let Ok(dmabuf) = unsafe { self.dmabuf.borrow().build() } {
                let start_render_time = std::time::Instant::now();
                dmabuf.snapshot(snapshot, width as f64, height as f64);
                println!("Render time: {:?}", start_render_time.elapsed());
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
