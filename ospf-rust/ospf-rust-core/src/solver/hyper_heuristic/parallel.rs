//! worker 本地构造与消息传递，不要求模型或会话为 Send / Worker-local construction and messaging without requiring Send models or sessions

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::model::callback::Solution;
use crate::solver::heuristic::{Individual, Iteration};
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::*;
use crate::solver::heuristic::session::archive::ObjectiveComponents;
use crate::solver::hyper_heuristic::config::{AlgorithmConfig, ComponentSpec};
use crate::solver::hyper_heuristic::operators::PolicySession;
use crate::solver::hyper_heuristic::problem::{ProblemHandle, RandomHandle};
use crate::solver::hyper_heuristic::registry::{AlgorithmContext, SearchRegistry};
use crate::solver::hyper_heuristic::selection::{PolicySnapshot, SelectionTransition};
use crate::solver::hyper_heuristic::{boundary, invalid};
use crate::variable::VariableId;

type WorkerSession<O, V> = (ProblemHandle<O, V>, Box<dyn PolicySession<O, V>>, RandomHandle);

pub(crate) fn execute<O: Clone, V: HeuristicValue>(
    session: &mut dyn PolicySession<O, V>,
    slice: &mut SearchSlice,
    seeds: Vec<Solution<V>>,
    initialized: bool,
) -> SearchStepResult<O, V> {
    let start = slice.used();
    let mut report = SearchStepResult::empty(SearchTermination::SliceComplete);
    let mut collect = |result: Result<SearchStepResult<O, V>>| -> bool {
        match result {
            Ok(result) => {
                report.candidates.extend(result.candidates);
                report.steps += result.steps;
                report.termination = result.termination;
                report.diagnostics.extend(result.diagnostics);
            }
            Err(error) => { report.termination = SearchTermination::Failed; report.diagnostics.push(error.to_string()); }
        }
        report.termination != SearchTermination::Failed && report.diagnostics.is_empty()
    };
    let imported = !initialized || seeds.is_empty() || collect(boundary(|| session.import_solutions(seeds.clone(), slice)));
    if imported && collect(boundary(|| session.step(slice))) && !initialized && session.iteration().iteration() > 0 && !seeds.is_empty() {
        collect(boundary(|| session.import_solutions(seeds, slice)));
    }
    report.evaluations = slice.used().saturating_sub(start);
    report
}

struct Metadata {
    id: String,
    fingerprint: String,
    order: Vec<VariableId>,
    encoding: SearchEncoding,
    multi_objective: bool,
    supports_import: bool,
    has_objective_coordinates: bool,
    has_constraint_violation: bool,
}

enum Command<V> {
    Advance { slice: SearchSlice, seeds: Vec<Solution<V>>, initialized: bool },
    Snapshot,
    Restore(SearchSessionSnapshot),
    SaveProblem,
    LoadProblem(Value),
    Policies,
    RestorePolicies(BTreeMap<String, PolicySnapshot>),
    Close,
}

struct Request<V> { command: Command<V>, random: i64 }
struct Response { result: Result<Value>, random: i64 }

#[derive(Serialize, Deserialize)]
struct WireReport {
    candidates: Value,
    evaluations: u64,
    steps: u64,
    termination: SearchTermination,
    diagnostics: Vec<String>,
    iteration: u64,
    stagnation: u64,
    active_nanos: u64,
    execution_nanos: u64,
    transitions: Vec<SelectionTransition>,
}

struct RemoteHandle<V> {
    sender: Sender<Request<V>>,
    receiver: RefCell<Receiver<Response>>,
    thread: RefCell<Option<JoinHandle<()>>>,
    random: RandomHandle,
    pending: Cell<bool>,
    closed: Cell<bool>,
}

impl<V> RemoteHandle<V> {
    fn send(&self, command: Command<V>) -> Result<()> {
        if self.closed.get() || self.pending.get() { return Err(invalid("remote worker is closed or busy")); }
        self.sender.send(Request { command, random: self.random.state() }).map_err(|_| invalid("worker command channel disconnected"))?;
        self.pending.set(true);
        Ok(())
    }

    fn receive(&self) -> Result<Value> {
        if !self.pending.replace(false) { return Err(invalid("no pending worker response")); }
        let response = self.receiver.borrow().recv().map_err(|_| invalid("worker response channel disconnected"))?;
        self.random.restore(response.random);
        response.result
    }

    fn call(&self, command: Command<V>) -> Result<Value> { self.send(command)?; self.receive() }

    fn close(&self) -> Result<()> {
        if self.closed.get() { return Ok(()); }
        if self.pending.get() { let _ = self.receive(); }
        let result = self.call(Command::Close).map(|_| ());
        self.closed.set(true);
        if let Some(thread) = self.thread.borrow_mut().take() && thread.join().is_err() {
            return Err(invalid("worker thread panicked"));
        }
        result
    }
}

impl<V> Drop for RemoteHandle<V> {
    fn drop(&mut self) { let _ = self.close(); }
}

struct RemoteProblem<V> { handle: Rc<RemoteHandle<V>>, metadata: Rc<Metadata> }

impl<O: Clone, V: HeuristicValue> SearchProblem<O, V> for RemoteProblem<V> {
    fn id(&self) -> &str { &self.metadata.id }
    fn fingerprint(&self) -> &str { &self.metadata.fingerprint }
    fn variable_order(&self) -> Vec<VariableId> { self.metadata.order.clone() }
    fn initial_solutions(&self, _: usize) -> Vec<Solution<V>> { Vec::new() }
    fn objective(&self, _: &Solution<V>) -> Option<O> { None }
    fn dominates(&self, _: &O, _: &O) -> bool { false }
    fn multi_objective(&self) -> bool { self.metadata.multi_objective }
    fn has_objective_coordinates(&self) -> bool { self.metadata.has_objective_coordinates }
    fn objective_coordinates(&self, _: &O) -> Result<Vec<f64>> { Err(invalid("remote metadata cannot execute coordinate callbacks")) }
    fn has_constraint_violation(&self) -> bool { self.metadata.has_constraint_violation }
    fn constraint_violation(&self, _: &Solution<V>) -> Result<f64> { Err(invalid("remote metadata cannot execute violation callbacks")) }
    fn encoding(&self) -> SearchEncoding { self.metadata.encoding }
    fn save_state(&self) -> Result<Value> { self.handle.call(Command::SaveProblem) }
    fn load_state(&mut self, state: &Value) -> Result<()> { self.handle.call(Command::LoadProblem(state.clone())).map(|_| ()) }
}

struct RemoteSession<O, V> {
    handle: Rc<RemoteHandle<V>>,
    metadata: Rc<Metadata>,
    codec: Box<dyn SearchCodec<Vec<Individual<O, V>>>>,
    iteration: Iteration,
    transitions: Vec<SelectionTransition>,
    execution_nanos: u64,
}

impl<O: Clone, V: HeuristicValue> SearchSession<O, V> for RemoteSession<O, V> {
    fn supports_import(&self) -> bool { self.metadata.supports_import }
    fn step(&mut self, _: &mut SearchSlice) -> Result<SearchStepResult<O, V>> { Err(invalid("use remote dispatch for threaded sessions")) }
    fn snapshot(&self) -> Result<SearchSessionSnapshot> {
        serde_json::from_value(self.handle.call(Command::Snapshot)?).map_err(|error| invalid(error.to_string()))
    }
    fn restore(&mut self, saved: &SearchSessionSnapshot) -> Result<()> {
        self.handle.call(Command::Restore(saved.clone()))?;
        self.iteration = Iteration::with_counts(saved.iteration, saved.stagnation);
        self.iteration.restore_time(Duration::from_nanos(saved.active_nanos));
        Ok(())
    }
    fn iteration(&self) -> &Iteration { &self.iteration }
    fn close(&mut self) -> Result<()> { self.handle.close() }
}

impl<O: Clone, V: HeuristicValue> PolicySession<O, V> for RemoteSession<O, V> {
    fn dispatch(&mut self, slice: SearchSlice, seeds: Vec<Solution<V>>, initialized: bool) -> Result<()> {
        self.handle.send(Command::Advance { slice, seeds, initialized })
    }

    fn receive(&mut self) -> Result<SearchStepResult<O, V>> {
        self.execution_nanos = 0;
        let report: WireReport = serde_json::from_value(self.handle.receive()?).map_err(|error| invalid(error.to_string()))?;
        self.iteration = Iteration::with_counts(report.iteration, report.stagnation);
        self.iteration.restore_time(Duration::from_nanos(report.active_nanos));
        self.transitions.extend(report.transitions);
        self.execution_nanos = report.execution_nanos;
        Ok(SearchStepResult { candidates: self.codec.decode(&report.candidates)?, evaluations: report.evaluations,
            steps: report.steps, termination: report.termination, diagnostics: report.diagnostics })
    }

    fn execution_nanos(&self) -> u64 { self.execution_nanos }

    fn policies(&self) -> Result<BTreeMap<String, PolicySnapshot>> {
        serde_json::from_value(self.handle.call(Command::Policies)?).map_err(|error| invalid(error.to_string()))
    }
    fn restore_policies(&mut self, policies: &BTreeMap<String, PolicySnapshot>) -> Result<()> {
        self.handle.call(Command::RestorePolicies(policies.clone())).map(|_| ())
    }
    fn drain_transitions(&mut self) -> Vec<SelectionTransition> { std::mem::take(&mut self.transitions) }
}

pub(crate) fn spawn<O: ObjectiveComponents + 'static, V: HeuristicValue>(
    registry: SearchRegistry<O, V>,
    problem_spec: ComponentSpec,
    algorithm: AlgorithmConfig,
    random: SearchRandom,
    learning: bool,
    reward: ComponentSpec,
    completed: Sender<String>,
) -> Result<WorkerSession<O, V>> {
    let (sender, commands) = mpsc::channel::<Request<V>>();
    let (responses, receiver) = mpsc::channel();
    let (ready, startup) = mpsc::sync_channel(1);
    let local_random = RandomHandle::new(random.clone());
    let id = algorithm.id.clone();
    let decoder_registry = registry.clone();
    let thread = thread::Builder::new().name(format!("hyper-{id}")).spawn(move || {
        let initialized = boundary(|| {
            let problem = registry.problem(&problem_spec, random.clone())?;
            let random = RandomHandle::new(random);
            let session = registry.session_configured(AlgorithmContext { config: algorithm, problem: problem.clone(), random: random.clone() }, learning, &reward)?;
            let codec = registry.archive_codec(problem.variable_order()).ok_or_else(|| invalid("parallel execution requires a lossless archive codec"))?;
            Ok((problem, random, session, codec))
        });
        let (mut problem, random, mut session, codec) = match initialized {
            Ok(value) => value,
            Err(error) => { let _ = ready.send(Err(error)); return; }
        };
        let metadata = Metadata { id: problem.id().into(), fingerprint: problem.fingerprint().into(), order: problem.variable_order(),
            encoding: problem.encoding(), multi_objective: problem.multi_objective(), supports_import: session.supports_import(),
            has_objective_coordinates: problem.has_objective_coordinates(), has_constraint_violation: problem.has_constraint_violation() };
        if ready.send(Ok(metadata)).is_err() { let _ = boundary(|| session.close()); return; }
        while let Ok(request) = commands.recv() {
            random.restore(request.random);
            let notify = matches!(&request.command, Command::Advance { .. });
            let closing = matches!(&request.command, Command::Close);
            let result = boundary(|| match request.command {
                Command::Advance { mut slice, seeds, initialized } => {
                    let started = Instant::now();
                    let report = execute(session.as_mut(), &mut slice, seeds, initialized);
                    let execution_nanos = started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
                    let wire = WireReport { candidates: codec.encode(&report.candidates)?, evaluations: slice.used(), steps: report.steps,
                        termination: report.termination, diagnostics: report.diagnostics, iteration: session.iteration().iteration(),
                        stagnation: session.iteration().not_better_iteration(), active_nanos: session.iteration().time().as_nanos().min(u64::MAX as u128) as u64,
                        transitions: session.drain_transitions(), execution_nanos };
                    serde_json::to_value(wire).map_err(|error| invalid(error.to_string()))
                }
                Command::Snapshot => serde_json::to_value(session.snapshot()?).map_err(|error| invalid(error.to_string())),
                Command::Restore(saved) => { session.restore(&saved)?; Ok(Value::Null) }
                Command::SaveProblem => problem.save_state(),
                Command::LoadProblem(state) => { problem.load_state(&state)?; Ok(Value::Null) }
                Command::Policies => serde_json::to_value(session.policies()?).map_err(|error| invalid(error.to_string())),
                Command::RestorePolicies(policies) => { session.restore_policies(&policies)?; Ok(Value::Null) }
                Command::Close => { session.close()?; Ok(Value::Null) }
            });
            if responses.send(Response { result, random: random.state() }).is_err() { break; }
            if notify { let _ = completed.send(id.clone()); }
            if closing { return; }
        }
        let _ = boundary(|| session.close());
    }).map_err(|error| invalid(format!("cannot spawn worker: {error}")))?;
    let metadata = match startup.recv() {
        Ok(Ok(metadata)) => Rc::new(metadata),
        result => { let _ = thread.join(); return Err(match result { Ok(Err(error)) => error, _ => invalid("worker startup disconnected") }); }
    };
    let codec = match decoder_registry.archive_codec(metadata.order.clone()) {
        Some(codec) => codec,
        None => { drop(sender); let _ = thread.join(); return Err(invalid("missing parallel archive decoder")); }
    };
    let handle = Rc::new(RemoteHandle { sender, receiver: RefCell::new(receiver), thread: RefCell::new(Some(thread)),
        random: local_random.clone(), pending: Cell::new(false), closed: Cell::new(false) });
    let problem = ProblemHandle::new(Box::new(RemoteProblem { handle: handle.clone(), metadata: metadata.clone() }));
    let session = Box::new(RemoteSession { handle, metadata, codec, iteration: Iteration::new(), transitions: Vec::new(), execution_nanos: 0 });
    Ok((problem, session, local_random))
}
