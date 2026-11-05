//! Observe bounded terminal notifications without retaining their message contents.
use super::{PtyEvent, CODEX_ATTENTION_EVENT, CODEX_PROGRAM_EVENT};
use alacritty_terminal::{
    event::{OnResize, WindowSize},
    tty::{ChildEvent, EventedPty, EventedReadWrite},
    vte,
};
use polling::{Event, PollMode, Poller};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        mpsc::Sender,
        Arc, Mutex,
    },
};

/// Explicit Codex OSC 7501 lifecycle, not terminal text or process heuristics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexProgramStatus {
    Idle,
    Working,
    Permission,
    Question,
    Auth,
    Done,
    Error,
    Clear,
}

/// Recognized root-program identity. Child records never enter the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramApp {
    Codex,
    Kimi,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramEvent {
    pub app: Option<ProgramApp>,
    pub state: CodexProgramStatus,
}
impl From<CodexProgramStatus> for ProgramEvent {
    fn from(state: CodexProgramStatus) -> Self {
        Self {
            app: (state != CodexProgramStatus::Clear).then_some(ProgramApp::Codex),
            state,
        }
    }
}

#[derive(Default)]
pub(super) struct ProgramQueue {
    epoch: u64,
    last: Option<ProgramEvent>,
    values: VecDeque<ProgramEvent>,
}
impl ProgramQueue {
    pub fn reset(&mut self, epoch: u64) {
        self.epoch = epoch;
        self.last = None;
        self.values.clear();
    }
    fn push(&mut self, epoch: u64, state: ProgramEvent) -> bool {
        if epoch != self.epoch || self.last == Some(state) {
            return false;
        }
        self.last = Some(state);
        let wake = self.values.is_empty();
        if self.values.len() == 32 {
            self.values.pop_front();
        }
        self.values.push_back(state);
        wake
    }
    pub fn drain(&mut self) -> Vec<ProgramEvent> {
        self.values.drain(..).collect()
    }
}

pub(super) struct Control {
    pub enabled: Arc<AtomicBool>,
    pub epoch: Arc<AtomicU64>,
    pub pending: Arc<AtomicU8>,
    pub program: Arc<Mutex<ProgramQueue>>,
}

fn program_status(params: &[&[u8]]) -> Option<ProgramEvent> {
    if params.len() != 2 || params[0] != b"7501" || params[1].len() > 4096 {
        return None;
    }
    let mut state = None;
    let mut app = None;
    let mut kind = None;
    let mut seen = std::collections::HashSet::new();
    for pair in params[1].split(|b| *b == b':') {
        let split = pair.iter().position(|b| *b == b'=')?;
        let (key, value) = (&pair[..split], &pair[split + 1..]);
        if !seen.insert(key) {
            return None;
        }
        match key {
            b"state" => state = Some(value),
            b"app" => {
                app = Some(match value {
                    b"codex" => ProgramApp::Codex,
                    b"kimi-code" => ProgramApp::Kimi,
                    _ => return None,
                })
            }
            b"kind" => kind = Some(value),
            // Root has no id. Ignore all subagent/hierarchical records.
            b"id" => return None,
            // Deliberately discard optional encoded display text.
            b"title" | b"msg" => {}
            _ => return None,
        }
    }
    use CodexProgramStatus as S;
    let state = match (state?, kind) {
        (b"idle", None) => S::Idle,
        (b"working", None) => S::Working,
        (b"done", None) => S::Done,
        (b"error", None) => S::Error,
        (b"clear", None) => S::Clear,
        (b"blocked", Some(b"permission")) => S::Permission,
        (b"blocked", Some(b"question")) => S::Question,
        (b"blocked", Some(b"auth")) => S::Auth,
        _ => return None,
    };
    if app.is_none() && state != S::Clear {
        return None;
    }
    Some(ProgramEvent { app, state })
}

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
    program: Arc<Mutex<ProgramQueue>>,
}

impl vte::Perform for Collector {
    fn osc_dispatch(&mut self, params: &[&[u8]], _: bool) {
        if let Some(state) = program_status(params) {
            if self.epoch.load(Ordering::Acquire) == self.observed_epoch
                && self
                    .program
                    .lock()
                    .unwrap()
                    .push(self.observed_epoch, state)
            {
                let _ = self.sender.send((
                    self.id,
                    PtyEvent::Title(format!("{CODEX_PROGRAM_EVENT};{}", self.observed_epoch)),
                ));
                self.ctx.request_repaint_of(egui::ViewportId::ROOT);
            }
            return;
        }
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
        control: Control,
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
                enabled: control.enabled,
                collector: Collector {
                    id,
                    sender,
                    ctx,
                    epoch: control.epoch,
                    observed_epoch: 0,
                    pending: control.pending,
                    program: control.program,
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
    fn native_lifecycle_is_ordered_bounded_and_epoch_scoped() {
        let (tx, rx) = std::sync::mpsc::channel();
        let program = Arc::new(Mutex::new(ProgramQueue::default()));
        program.lock().unwrap().reset(1);
        let mut collector = Collector {
            id: 42,
            sender: tx,
            ctx: egui::Context::default(),
            epoch: Arc::new(AtomicU64::new(1)),
            observed_epoch: 1,
            pending: Arc::new(AtomicU8::new(0)),
            program: program.clone(),
        };
        let mut parser = vte::Parser::new();
        for part in [b"plain state=done\x1b]7501;state=work".as_slice(), b"ing:app=codex\x1b\\\x1b]7501;state=blocked:app=codex:kind=question\x07\x1b]7501;state=done:app=codex\x1b\\".as_slice()] {
            parser.advance(&mut collector, part);
        }
        assert!(
            matches!(rx.try_recv(), Ok((42, PtyEvent::Title(s))) if s == format!("{CODEX_PROGRAM_EVENT};1"))
        );
        assert!(
            rx.try_recv().is_err(),
            "one pending wake, not one message per state"
        );
        assert_eq!(
            program
                .lock()
                .unwrap()
                .drain()
                .into_iter()
                .map(|e| e.state)
                .collect::<Vec<_>>(),
            [
                CodexProgramStatus::Working,
                CodexProgramStatus::Question,
                CodexProgramStatus::Done
            ]
        );
        for _ in 0..100 {
            parser.advance(
                &mut collector,
                b"\x1b]7501;state=working:app=codex\x1b\\\x1b]7501;state=done:app=codex\x1b\\",
            );
        }
        let states = program.lock().unwrap().drain();
        assert_eq!(states.len(), 32);
        assert_eq!(
            states.last().map(|e| e.state),
            Some(CodexProgramStatus::Done)
        );
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
        collector.epoch.store(2, Ordering::Release);
        program.lock().unwrap().reset(2);
        parser.advance(&mut collector, b"\x1b]7501;state=error:app=codex\x1b\\");
        assert!(program.lock().unwrap().drain().is_empty());
        collector.observed_epoch = 2;
        parser.advance(&mut collector, b"\x1b]7501;state=idle:app=codex\x1b\\");
        assert_eq!(
            program
                .lock()
                .unwrap()
                .drain()
                .into_iter()
                .map(|e| e.state)
                .collect::<Vec<_>>(),
            [CodexProgramStatus::Idle]
        );
    }
    #[test]
    fn native_protocol_rejects_text_unknown_apps_and_unrecognized_states() {
        for wire in [
            b"state=done:app=codex".as_slice(),
            b"state=done:app=other",
            b"state=finished:app=codex",
            b"state=blocked:app=codex:kind=unknown",
            b"state=done:app=codex:detail=private",
        ] {
            if wire.starts_with(b"state=") && wire == b"state=done:app=codex" {
                assert!(program_status(&[b"2", wire]).is_none());
            } else {
                assert!(program_status(&[b"7501", wire]).is_none());
            }
        }
        assert_eq!(
            program_status(&[b"7501", b"state=clear"]),
            Some(CodexProgramStatus::Clear.into())
        );
    }
    #[test]
    fn kimi_metadata_is_discarded_and_children_cannot_override_root() {
        let event = program_status(&[
            b"7501",
            b"msg=cHJpdmF0ZQ==:kind=auth:app=kimi-code:state=blocked",
        ])
        .unwrap();
        assert_eq!(event.app, Some(ProgramApp::Kimi));
        assert_eq!(event.state, CodexProgramStatus::Auth);
        for wire in [
            b"state=done:app=kimi-code:id=child".as_slice(),
            b"state=done:app=kimi-code:id=",
            b"state=done:app=kimi-code:app=codex",
            b"?",
            b"state=done:msg=cHJpdmF0ZQ==",
        ] {
            assert!(program_status(&[b"7501", wire]).is_none());
        }
        let mut q = ProgramQueue::default();
        q.reset(1);
        q.push(1, CodexProgramStatus::Idle.into());
        q.push(
            1,
            ProgramEvent {
                app: Some(ProgramApp::Kimi),
                state: CodexProgramStatus::Idle,
            },
        );
        assert_eq!(q.drain().len(), 2, "dedup includes app identity");
    }
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
            program: Arc::new(Mutex::new(ProgramQueue::default())),
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
