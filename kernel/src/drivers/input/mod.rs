//! Input device drivers

use super::bus::virtio::VirtIOHal;
use crate::{
    sync::{Condvar, UPIntrFreeCell},
    task::schedule,
};
use alloc::{collections::VecDeque, sync::Arc};
use core::any::Any;
use lazy_static::lazy_static;
use virtio_drivers::{VirtIOHeader, VirtIOInput};

const VIRTIO5: usize = 0x1000_5000;
const VIRTIO6: usize = 0x1000_6000;

lazy_static! {
    pub static ref KEYBOARD_DEVICE: Arc<dyn InputDevice> =
        Arc::new(VirtIOInputWrapper::new(VIRTIO5));
    pub static ref MOUSE_DEVICE: Arc<dyn InputDevice> = Arc::new(VirtIOInputWrapper::new(VIRTIO6));
}

struct VirtIOInputInner {
    virtio_input: VirtIOInput<'static, VirtIOHal>,
    events: VecDeque<u64>,
}

struct VirtIOInputWrapper {
    inner: UPIntrFreeCell<VirtIOInputInner>,
    condvar: Condvar,
}

#[allow(clippy::module_name_repetitions)]
pub trait InputDevice: Send + Sync + Any {
    fn read_event(&self) -> u64;
    fn is_empty(&self) -> bool;
    fn handle_irq(&self);
}

impl VirtIOInputWrapper {
    pub fn new(addr: usize) -> Self {
        let inner = VirtIOInputInner {
            virtio_input: unsafe {
                VirtIOInput::<VirtIOHal>::new(&mut *(addr as *mut VirtIOHeader)).unwrap()
            },
            events: VecDeque::new(),
        };
        Self {
            inner: unsafe { UPIntrFreeCell::new(inner) },
            condvar: Condvar::new(),
        }
    }
}

impl InputDevice for VirtIOInputWrapper {
    fn read_event(&self) -> u64 {
        loop {
            let mut inner = self.inner.exclusive_access();
            if let Some(event) = inner.events.pop_front() {
                return event;
            }
            let task_cx_ptr = self.condvar.wait_no_sched();
            drop(inner);
            schedule(task_cx_ptr);
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.exclusive_access().events.is_empty()
    }

    fn handle_irq(&self) {
        let mut count = 0;

        self.inner.exclusive_session(|inner| {
            while let Some((_token, event)) = inner.virtio_input.pop_pending_event() {
                count += 1;
                let result = u64::from(event.event_type) << 48
                    | u64::from(event.code) << 32
                    | u64::from(event.value);
                inner.events.push_back(result);
            }
        });

        if count > 0 {
            self.condvar.signal();
        }
    }
}
