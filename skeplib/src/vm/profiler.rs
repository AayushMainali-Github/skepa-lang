use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::bytecode::Instr;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Event {
    VmStartup,
    GlobalsInit,
    MainRun,
    RunChunk,
    Call,
    CallIdx,
    CallValue,
    CallMethod,
    CallBuiltin,
    BuiltinDispatch,
    ArrayMake,
    ArrayMakeRepeat,
    ArrayGet,
    ArraySet,
    ArraySetChain,
    ArrayLen,
    StructMake,
    StructGet,
    StructSetPath,
}

impl Event {
    fn label(self) -> &'static str {
        match self {
            Event::VmStartup => "vm_startup",
            Event::GlobalsInit => "globals_init",
            Event::MainRun => "main_run",
            Event::RunChunk => "run_chunk",
            Event::Call => "call",
            Event::CallIdx => "call_idx",
            Event::CallValue => "call_value",
            Event::CallMethod => "call_method",
            Event::CallBuiltin => "call_builtin",
            Event::BuiltinDispatch => "builtin_dispatch",
            Event::ArrayMake => "array_make",
            Event::ArrayMakeRepeat => "array_make_repeat",
            Event::ArrayGet => "array_get",
            Event::ArraySet => "array_set",
            Event::ArraySetChain => "array_set_chain",
            Event::ArrayLen => "array_len",
            Event::StructMake => "struct_make",
            Event::StructGet => "struct_get",
            Event::StructSetPath => "struct_set_path",
        }
    }
}

#[derive(Default)]
struct OpCounters {
    total: u64,
    load_const: u64,
    load_local: u64,
    store_local: u64,
    load_global: u64,
    store_global: u64,
    pop: u64,
    arith: u64,
    compare: u64,
    logical: u64,
    jump: u64,
    call: u64,
    builtin: u64,
    array: u64,
    structure: u64,
    ret: u64,
}

impl OpCounters {
    fn record(&mut self, instr: &Instr) {
        self.total += 1;
        match instr {
            Instr::LoadConst(_) => self.load_const += 1,
            Instr::LoadLocal(_) => self.load_local += 1,
            Instr::StoreLocal(_) => self.store_local += 1,
            Instr::LoadGlobal(_) => self.load_global += 1,
            Instr::StoreGlobal(_) => self.store_global += 1,
            Instr::Pop => self.pop += 1,
            Instr::NegInt
            | Instr::NotBool
            | Instr::Add
            | Instr::SubInt
            | Instr::MulInt
            | Instr::DivInt
            | Instr::ModInt => self.arith += 1,
            Instr::Eq
            | Instr::Neq
            | Instr::LtInt
            | Instr::LteInt
            | Instr::GtInt
            | Instr::GteInt => self.compare += 1,
            Instr::AndBool | Instr::OrBool => self.logical += 1,
            Instr::Jump(_) | Instr::JumpIfFalse(_) | Instr::JumpIfTrue(_) => self.jump += 1,
            Instr::Call { .. }
            | Instr::CallIdx { .. }
            | Instr::CallValue { .. }
            | Instr::CallMethod { .. }
            | Instr::CallMethodId { .. } => self.call += 1,
            Instr::CallBuiltin { .. } | Instr::CallBuiltinId { .. } => {
                self.call += 1;
                self.builtin += 1;
            }
            Instr::MakeArray(_)
            | Instr::MakeArrayRepeat(_)
            | Instr::ArrayGet
            | Instr::ArraySet
            | Instr::ArraySetChain(_)
            | Instr::ArrayLen => self.array += 1,
            Instr::MakeStruct { .. }
            | Instr::MakeStructId { .. }
            | Instr::StructGet(_)
            | Instr::StructGetSlot(_)
            | Instr::StructSetPath(_)
            | Instr::StructSetPathSlots(_) => self.structure += 1,
            Instr::Return => self.ret += 1,
        }
    }
}

#[derive(Default)]
struct EventStats {
    count: u64,
    total: Duration,
}

impl EventStats {
    fn record(&mut self, elapsed: Duration) {
        self.count += 1;
        self.total += elapsed;
    }
}

#[derive(Default)]
struct ProfileSession {
    label: String,
    started_at: Option<Instant>,
    ops: OpCounters,
    events: Vec<(&'static str, EventStats)>,
}

impl ProfileSession {
    fn with_label(label: &str) -> Self {
        Self {
            label: label.to_string(),
            started_at: Some(Instant::now()),
            ..Self::default()
        }
    }

    fn record_event(&mut self, event: Event, elapsed: Duration) {
        let label = event.label();
        if let Some((_, stats)) = self.events.iter_mut().find(|(name, _)| *name == label) {
            stats.record(elapsed);
            return;
        }
        let mut stats = EventStats::default();
        stats.record(elapsed);
        self.events.push((label, stats));
    }

    fn print(&self) {
        let total = self
            .started_at
            .map(|started| started.elapsed())
            .unwrap_or_default();
        eprintln!(
            "[vm-profile] session={} total_ms={:.3}",
            self.label,
            total.as_secs_f64() * 1_000.0
        );
        eprintln!(
            "[vm-profile] ops total={} load_const={} load_local={} store_local={} load_global={} store_global={} pop={} arith={} compare={} logical={} jump={} call={} builtin={} array={} struct={} return={}",
            self.ops.total,
            self.ops.load_const,
            self.ops.load_local,
            self.ops.store_local,
            self.ops.load_global,
            self.ops.store_global,
            self.ops.pop,
            self.ops.arith,
            self.ops.compare,
            self.ops.logical,
            self.ops.jump,
            self.ops.call,
            self.ops.builtin,
            self.ops.array,
            self.ops.structure,
            self.ops.ret,
        );
        for (name, stats) in &self.events {
            eprintln!(
                "[vm-profile] event={} count={} total_ms={:.3}",
                name,
                stats.count,
                stats.total.as_secs_f64() * 1_000.0
            );
        }
    }
}

thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static ACTIVE_SESSION: RefCell<Option<ProfileSession>> = const { RefCell::new(None) };
}

pub(crate) struct SessionGuard {
    active: bool,
}

impl SessionGuard {
    pub(crate) fn start(label: &str) -> Self {
        let active = enabled();
        if active {
            ACTIVE_SESSION.with(|slot| {
                *slot.borrow_mut() = Some(ProfileSession::with_label(label));
            });
        }
        Self { active }
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        ACTIVE_SESSION.with(|slot| {
            if let Some(session) = slot.borrow_mut().take() {
                session.print();
            }
        });
    }
}

pub(crate) struct ScopedTimer {
    event: Event,
    started_at: Option<Instant>,
}

impl ScopedTimer {
    pub(crate) fn new(event: Event) -> Self {
        let started_at = if enabled() {
            Some(Instant::now())
        } else {
            None
        };
        Self { event, started_at }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let Some(started_at) = self.started_at else {
            return;
        };
        record_event(self.event, started_at.elapsed());
    }
}

pub(crate) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("SKEPA_PROFILE_VM").is_some())
}

pub(crate) fn record_instr(instr: &Instr) {
    if !enabled() {
        return;
    }
    ACTIVE_SESSION.with(|slot| {
        if let Some(session) = slot.borrow_mut().as_mut() {
            session.ops.record(instr);
        }
    });
}

fn record_event(event: Event, elapsed: Duration) {
    if !enabled() {
        return;
    }
    ACTIVE_SESSION.with(|slot| {
        if let Some(session) = slot.borrow_mut().as_mut() {
            session.record_event(event, elapsed);
        }
    });
}
