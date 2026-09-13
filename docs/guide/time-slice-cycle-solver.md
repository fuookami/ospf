# Time Slice Round Robin Solver

Time-slice round robin lets long-running solve tasks share resources in recoverable segments. A task runs for a quantum, returns safely with results and recovery data, releases its resources, and becomes eligible for another turn. Heterogeneous scheduling also chooses where each slice runs, balancing solution quality, waiting time, recovery overhead, and cost.

This page builds on [Remote Solving](./remote-solver) and describes the design and implementation boundaries of heterogeneous time slicing. The policy formulas and tuning examples below illustrate the scheduling design; they are not copyable configuration or new client APIs. Available capabilities depend on the deployed protocol, advertised node capabilities, and actual execution backend. Rotation does not change the mathematical model or guarantee hard deadlines or faster optimality proofs.

## 1. Choose the task, node, and quantum together

A scheduling decision is a tuple $(T,n,Q)$: which task $T$ to run, on which node $n$, and for how much solve time $Q$.

| Object | Information needed for scheduling |
|---|---|
| Task | Model type and fingerprint, tenant, priority, deadline, budget, estimated runtime and memory, recovery capabilities, trusted checkpoint |
| Node | Supported models and backends, available concurrency and licenses, health, task-family performance, price and billing granularity, recovery compatibility |
| Slice | Task and attempt identity, node, time allowance, input checkpoint, output result and checkpoint, actual runtime and cost |

The dispatcher owns admission, queues, node selection, quantum selection, and rescheduling. The calculator and execution bridge own model reconstruction, backend execution, and checkpoint production. Scheduling consumes structured capabilities and reports; it neither guesses capabilities from logs nor directly depends on vendor SDKs.

Simple tasks should normally run to completion. When solving is much faster than saving, transferring, and rebuilding state, slicing only adds overhead. Run-to-completion still respects task limits, budgets, and cancellation.

## 2. Recoverability is an admission requirement

Rotation is allowed only when the backend can safely end the current slice and provide valid recovery input for the next one.

| Capability combination | Execution approach |
|---|---|
| Native interrupt + native checkpoint | Restore native state on a compatible backend; the backend declares how much state is preserved |
| Controlled return + portable checkpoint + warm start | Rebuild the model and inject an incumbent or other supported information; no search-tree recovery promise |
| Neither combination | Non-preemptible execution; killing a process is not normal time slicing |

A portable file does not imply portability across arbitrary solvers. Every recovery checks tenant, task, model fingerprint, protocol and format versions, backend capabilities, and object integrity. Native checkpoints may also depend on solver versions and execution environments.

Portable CP recovery can use a model snapshot, incumbent, and fingerprint to rebuild and warm start. This is distinct from native search-tree recovery and must not carry the same naming or performance promises.

## 3. Filter nodes before comparing benefits

First retain healthy, compatible nodes with available capacity and licenses, tenant permission, and an affordable minimum execution cost. An incompatible recovery target is not a candidate, however cheap it may be.

Compare candidates using normalized components, with a lower score preferred:

$$
S(T,n)=w_c\widehat C+w_d\widehat D+w_q\widehat W+w_m\widehat M-w_p\widehat P.
$$

Here $C$ is estimated cost, $D$ deadline risk, $W$ queue delay, $M$ migration overhead, and $P$ task-family performance. Hats denote dimensionless values normalized against explicit baselines. Do not directly add seconds, money, and throughput. Record weights, normalization baselines, and estimator versions for auditing.

For billing granularity $b_n$, estimated runtime $t$, price per second $c_n$, and fixed per-slice fee $\ell_n$, one billing model is:

$$
C(T,n,t)=\left\lceil\frac{t}{b_n}\right\rceil b_n c_n+\ell_n.
$$

Add transfer, storage, recovery, and cold-start charges according to the actual contract. With per-slice rounding to 30 seconds, six 5-second slices may incur 180 billable seconds, whereas one 30-second slice incurs 30. Shorter slices are not necessarily cheaper.

For remaining deadline time $d>0$, estimated lateness risk can be expressed as:

$$
D=\frac{\max(0,\operatorname{ETA}-d)}{\max(d,\varepsilon)}.
$$

Handle expired tasks separately instead of using the small denominator to justify further execution. If even the fastest node cannot meet a deadline, explicitly report the risk and let task policy choose rejection, degradation, or best-effort continuation.

## 4. Weighted rotation and starvation prevention

The scheduling unit is a recoverable task slice, not an operating-system thread. Dynamic priority can combine urgency, waiting age, lack of progress, and starvation compensation. Bound and normalize these factors so that one urgent task cannot suppress all others indefinitely.

Repeatedly sorting by priority is not a fairness guarantee. A weighted round-robin implementation needs explicit per-round service quotas or deficit accounting, queue rotation, deterministic tie-breaking, and service opportunities for long-waiting tasks.

For example, with one execution slot and task weights A:B:C = 2:1:1, an illustrative service sequence is:

```text
Round 1: A → B → A → C
Round 2: A → B → A → C
```

This illustrates service opportunities, not equal runtime or cost. With variable quanta, fairness by slice count differs from fairness by compute time; choose and record the accounting basis. Bounded waiting also requires assumptions about capacity, admitted load, maximum slice duration, and safe-return latency. Persistent overload requires admission control, not just priority boosts.

## 5. Dynamic quanta: overhead versus responsiveness

Adapt the quantum to estimated solve time, recovery cost, node price, and deadlines. The following is an explainable policy form, not a promise about existing configuration:

$$
Q_0=\frac{\alpha\widehat t_{\mathrm{solve}}+\beta\widehat t_{\mathrm{checkpoint}}}{1+\gamma\widehat c},
\qquad
Q=\min(Q_{\max},\max(Q_{\min},Q_0)).
$$

The two time estimates use the same time unit, $\widehat c$ is a normalized cost rate, and $\alpha,\beta,\gamma$ are dimensionless parameters. Higher checkpoint overhead favors longer slices; higher rates may favor shorter ones. Billing rounding and total recovery cost still require a final check.

Let $h$ be the combined save and next-slice recovery overhead. To keep its fraction below $r$:

$$
\frac{h}{Q+h}\le r
\quad\Longrightarrow\quad
Q\ge\frac{1-r}{r}h.
$$

For $h=5$ seconds and $r=20\%$, at least 20 seconds of useful solving is needed. If a deadline or budget cannot accommodate that, change policy, use a single execution, or stop explicitly. Clipping the quantum does not preserve the overhead target.

An initial benchmark policy might use a 5-second minimum, 30-second baseline, and 300-second maximum. These are tuning starting points, not deployment defaults. Backend safe-return latency also limits the minimum. Reserve time for checkpointing, result upload, and finalization before the task deadline. Do not dispatch when insufficient time remains or let a minimum quantum override the deadline.

## 6. Slice lifecycle and trusted results

Organize each slice as follows:

1. Select a task and node, compute the quantum, and reserve estimated cost.
2. Validate the model and latest trusted checkpoint; create fresh slice and execution-attempt identities.
3. Rebuild or restore the model and execute with supported time controls.
4. Finish or return safely, producing a report, candidate result, and recovery data.
5. Validate identities, fingerprints, and digests; persist objects reliably before updating authoritative state.
6. Reconcile charges, update progress, and release the execution slot; complete the task or schedule further work.

A slice may follow `PLANNED → RUNNING → CHECKPOINTING → SUSPENDED`, or become `COMPLETED` or `FAILED`. Requeueing belongs to the task lifecycle. The next execution creates another slice; the task's `QUEUED` state is not a slice state. See [Remote Solving](./remote-solver) for task transitions.

An expected quantum return is not a failure timeout. Slice allowance, total task time limit, process hard timeout, and node heartbeat govern different boundaries. Late results from old attempts must not overwrite newer attempts; updates need idempotency and concurrency-version checks.

If checkpointing fails, retain the previous trusted recovery point and best trusted incumbent. Following node loss, resume only from qualified recovery input; otherwise policy may restart from the fixed model input. Successful scheduling, slice completion, or file creation does not prove feasibility, much less optimality.

## 7. Heterogeneous migration and progress feedback

Include upload, download, cold start, model rebuilding, warm start, and license charges in migration estimates. An additive hysteresis threshold $H>0$ on normalized scores can require:

$$
S(T,n_{\mathrm{new}})+H<S(T,n_{\mathrm{current}}).
$$

Combine this with a minimum number of resident slices to avoid oscillation. A percentage threshold needs an explicit denominator and zero handling; “10% better” is ambiguous for potentially negative scores.

Non-urgent work can start on an economical node, with upgrades reconsidered when progress is poor or deadline risk grows. Another policy finds an incumbent on a fast node and continues improvement or bound tightening on a cheaper node. Both require compatible recovery and positive estimated net benefit after migration.

Feedback can include objective value, bounds, gap, runtime, actual cost, and backend-supported search statistics. Estimate performance separately by model family, size, and backend. Compare objectives only for the same model fingerprint and objective, respecting minimization versus maximization. Missing gap is not zero, and warm starting does not guarantee a better result in every slice.

An EWMA can update runtime estimates for a task family:

$$
\widehat t_{k+1}=\lambda t_k+(1-\lambda)\widehat t_k,
\qquad 0<\lambda\le1.
$$

Track sample count, freshness, and outliers. Preserve the best trusted solution across slices instead of replacing it with a newer but worse result.

## 8. Budgets, protocol, and language parity

Reserve cost before dispatch and reconcile it against actual billing afterward. When funds are insufficient, policy may choose a cheaper compatible node, adjust concurrency or quantum, wait for more budget, return an acceptable incumbent, or terminate with an explicit reason. Budget exhaustion is not mathematical infeasibility.

Kotlin and Rust clients share server protocol semantics. Rotation remains a server-side policy; clients do not each implement their own scheduler. Task fields, node capabilities, recovery modes, and audit data follow a common protocol contract:

- Both clients agree on time units, defaults, errors, and recovery references for the same model.
- Wire durations use the agreed integer milliseconds; missing fields do not imply backend support.
- Unknown optional metadata can be ignored as specified, but unknown execution modes must not silently become supported modes.
- Cross-client checkpoint reading respects format versions, digests, model fingerprints, and backend compatibility.

This page does not invent Kotlin/Rust quantum-configuration calls. See [Remote Solving](./remote-solver) for client integration and actual source entry points; use extension fields only after their protocol version is released.

See the [remote-solver source](https://github.com/fuookami/ospf/tree/main/framework/remote-solver) and [design documentation](https://github.com/fuookami/ospf/blob/main/framework/remote-solver/design.md) for server implementation and deployment details.
