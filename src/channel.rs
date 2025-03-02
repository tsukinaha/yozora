use flume::{
    Receiver,
    Sender,
    unbounded,
};
use once_cell::sync::Lazy;
use smithay::backend::allocator::dmabuf::Dmabuf;

pub struct DmabufImported {
    pub tx: Sender<Dmabuf>,
    pub rx: Receiver<Dmabuf>,
}

pub static DMABUF_IMPORTED: Lazy<DmabufImported> = Lazy::new(|| {
    let (tx, rx) = unbounded::<Dmabuf>();

    DmabufImported { tx, rx }
});
