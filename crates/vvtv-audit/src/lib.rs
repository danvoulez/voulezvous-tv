use parking_lot::Mutex;
use vvtv_types::AuditEvent;

pub trait AuditSink {
    fn append(&self, event: AuditEvent);
    fn list(&self) -> Vec<AuditEvent>;
}

#[derive(Default)]
pub struct InMemoryAuditSink {
    events: Mutex<Vec<AuditEvent>>,
}

impl InMemoryAuditSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AuditSink for InMemoryAuditSink {
    fn append(&self, event: AuditEvent) {
        self.events.lock().push(event);
    }

    fn list(&self) -> Vec<AuditEvent> {
        self.events.lock().clone()
    }
}
