#[derive(Clone)]
pub struct Emitter<T> {
    emitted: std::rc::Rc<std::cell::RefCell<Vec<T>>>,
}

impl<T> Emitter<T> {
    pub fn emit(&self, token: impl Into<T>) {
        self.emitted.borrow_mut().push(token.into());
    }
}

impl<T: Clone> super::EventLoop<T> {
    #[must_use]
    pub fn emitter(&self) -> Emitter<T> {
        Emitter {
            emitted: self.emitted.clone(),
        }
    }

    pub(super) fn drain_ready_emitts_into(&mut self, triggered: &mut Vec<T>) {
        if triggered.is_empty() {
            std::mem::swap(triggered, &mut *self.emitted.borrow_mut());
        } else {
            triggered.append(&mut self.emitted.borrow_mut());
        }
    }
}
