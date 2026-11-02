//! Observe bounded terminal notifications without retaining their message contents.
use super::{PtyEvent, CODEX_ATTENTION_EVENT};
use alacritty_terminal::{
    event::{OnResize, WindowSize},
    tty::{ChildEvent, EventedPty, EventedReadWrite},
    vte,
};
use polling::{Event, PollMode, Poller};
use std::{
    io::{self, Read, Write},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        mpsc::Sender,
        Arc, Mutex,
    },
};

pub struct ObservedPty<P: EventedPty> {
    inner: Arc<Mutex<P>>,
    reader: ObservedReader<P>,
    writer: ObservedWriter<P>,
}
pub struct ObservedReader<P: EventedPty> {
    inner: Arc<Mutex<P>>,
    parser: vte::Parser,
    active_epoch: Option<u64>,
    enabled: Arc<AtomicBool>,
    collector: Collector,
}
pub struct ObservedWriter<P: EventedPty> {
    inner: Arc<Mutex<P>>,
}
struct Collector {
    id: u64,
    sender: Sender<(u64, PtyEvent)>,
    ctx: egui::Context,
    epoch: Arc<AtomicU64>,
    observed_epoch: u64,
    pending: Arc<AtomicU8>,
}

impl vte::Perform for Collector {
    fn osc_dispatch(&mut self, params: &[&[u8]], _: bool) {
        // Codex currently emits these text prefixes for the filtered notification
        // kinds. Ignore other OSC 9 messages, including generic command output.
        let Some(message) = params.get(1) else { return };
        if params.first() != Some(&b"9".as_slice()) {
            return;
        }
        let (kind, bit) = if message.starts_with(b"Plan mode prompt:") {
            ("question", 2)
        } else if message.starts_with(b"Approval requested")
            || message.starts_with(b"Codex wants to edit")
        {
            ("approval", 1)
        } else {
            return;
        };
        if self.epoch.load(Ordering::Acquire) != self.observed_epoch
            || self.pending.fetch_or(bit, Ordering::AcqRel) & bit != 0
        {
            return;
        }
        // A session can end between reading its epoch and reserving this bit.
        // Do not let a stale reader suppress the next invocation's notification.
        if self.epoch.load(Ordering::Acquire) != self.observed_epoch {
            self.pending.fetch_and(!bit, Ordering::AcqRel);
            return;
        }
        let notice = format!("{};{};{}", CODEX_ATTENTION_EVENT, self.observed_epoch, kind);
        if self
            .sender
            .send((self.id, PtyEvent::Title(notice)))
            .is_err()
        {
            self.pending.fetch_and(!bit, Ordering::AcqRel);
            return;
        }
        self.ctx.request_repaint_of(egui::ViewportId::ROOT);
    }
}

impl<P: EventedPty> ObservedPty<P> {
    pub fn new(
        pty: P,
        id: u64,
        enabled: Arc<AtomicBool>,
        epoch: Arc<AtomicU64>,
        pending: Arc<AtomicU8>,
        sender: Sender<(u64, PtyEvent)>,
        ctx: egui::Context,
    ) -> Self {
        let inner = Arc::new(Mutex::new(pty));
        Self {
            inner: inner.clone(),
            reader: ObservedReader {
                inner: inner.clone(),
                parser: vte::Parser::new(),
                active_epoch: None,
                enabled,
                collector: Collector {
                    id,
                    sender,
                    ctx,
                    epoch,
                    observed_epoch: 0,
                    pending,
                },
            },
            writer: ObservedWriter { inner },
        }
    }
}
impl<P: EventedPty> Read for ObservedReader<P> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.lock().unwrap().reader().read(buf)?;
        let enabled = self.enabled.load(Ordering::Acquire);
        if enabled {
            let epoch = self.collector.epoch.load(Ordering::Acquire);
            if self.active_epoch != Some(epoch) {
                self.parser = vte::Parser::new();
                self.active_epoch = Some(epoch);
                self.collector.observed_epoch = epoch;
            }
            self.parser.advance(&mut self.collector, &buf[..count]);
        } else {
            self.active_epoch = None;
        }
        Ok(count)
    }
}
impl<P: EventedPty> Write for ObservedWriter<P> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().unwrap().writer().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.lock().unwrap().writer().flush()
    }
}
impl<P: EventedPty> EventedReadWrite for ObservedPty<P> {
    type Reader = ObservedReader<P>;
    type Writer = ObservedWriter<P>;
    unsafe fn register(
        &mut self,
        poller: &Arc<Poller>,
        event: Event,
        mode: PollMode,
    ) -> io::Result<()> {
        // SAFETY: the wrapped PTY remains alive until the event loop deregisters it.
        unsafe { self.inner.lock().unwrap().register(poller, event, mode) }
    }
    fn reregister(&mut self, poller: &Arc<Poller>, event: Event, mode: PollMode) -> io::Result<()> {
        self.inner.lock().unwrap().reregister(poller, event, mode)
    }
    fn deregister(&mut self, poller: &Arc<Poller>) -> io::Result<()> {
        self.inner.lock().unwrap().deregister(poller)
    }
    fn reader(&mut self) -> &mut Self::Reader {
        &mut self.reader
    }
    fn writer(&mut self) -> &mut Self::Writer {
        &mut self.writer
    }
}
impl<P: EventedPty> EventedPty for ObservedPty<P> {
    fn next_child_event(&mut self) -> Option<ChildEvent> {
        self.inner.lock().unwrap().next_child_event()
    }
}
impl<P: EventedPty + OnResize> OnResize for ObservedPty<P> {
    fn on_resize(&mut self, size: WindowSize) {
        self.inner.lock().unwrap().on_resize(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_only_codex_attention_osc_across_split_chunks() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut collector = Collector {
            id: 42,
            sender: tx,
            ctx: egui::Context::default(),
            epoch: Arc::new(AtomicU64::new(1)),
            observed_epoch: 1,
            pending: Arc::new(AtomicU8::new(0)),
        };
        let mut parser = vte::Parser::new();
        for fragment in [
            b"normal\x1b]9;Approval ".as_slice(),
            b"requested: private command\x07".as_slice(),
        ] {
            parser.advance(&mut collector, fragment);
        }
        let (id, event) = rx.try_recv().unwrap();
        parser.advance(&mut collector, b"\x1b]9;Approval requested again\x07");
        assert!(rx.try_recv().is_err(), "pending notices must be coalesced");
        parser.advance(&mut collector, b"\x1b]9;Plan mode prompt: synthetic\x07");
        assert!(
            matches!(rx.try_recv(), Ok((42, PtyEvent::Title(value))) if value.ends_with(";question")),
            "a new question must not be hidden by a pending approval"
        );
        assert_eq!(id, 42);
        assert!(
            matches!(event, PtyEvent::Title(value) if value == format!("{};1;approval", CODEX_ATTENTION_EVENT))
        );
        parser.advance(
            &mut collector,
            b"\x1b]9;Unrelated notification\x07\x1b]2;Approval requested\x07\x07",
        );
        assert!(rx.try_recv().is_err());
        collector.pending.store(0, Ordering::Release);
        collector.epoch.store(2, Ordering::Release);
        parser.advance(&mut collector, b"\x1b]9;Approval requested: stale\x07");
        assert!(
            rx.try_recv().is_err(),
            "the previous invocation must not enqueue notices"
        );
        assert_eq!(collector.pending.load(Ordering::Acquire), 0);
    }
}
