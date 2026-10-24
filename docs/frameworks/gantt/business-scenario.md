# Calendars and Rolling Recovery

After [your first activity schedule](./first-schedule), add downtime and varying capacity before execution facts and baselines. A fact is not a soft preference with a large penalty.

## 1. Expose Start-Only Calendar Checks

An activity lasts 2 h and consumes capacity 2 throughout. Allowed starts are 0, 2 and 4 h. Equipment capacity is normally 2, unavailable on `[1,2)`, and reduced to 1 on `[3,4)`.

| Start | Occupancy | Feasible? | Reason |
| --- | --- | --- | --- |
| 0 h | [0,2) | No | Overlaps downtime |
| 2 h | [2,4) | No | Insufficient capacity in the second half |
| 4 h | [4,6) | Yes | Enough capacity throughout |

Reuse the first-lesson at/q helpers and resource IDs, passing downtime and capacity overrides explicitly to ActivityResource. With half-open intervals, a start at hour 4 does not occupy [3,4).

## 2. Attach Calendars to Resources

| Meaning | Kotlin | Rust |
| --- | --- | --- |
| Downtime | `ActivityResource.unavailableTimes` | `ActivityResource.unavailable_times` |
| Varying capacity | `ActivityResource.capacities` | `ActivityResource.capacities` |
| Relative phase occupancy | `ActivityResourceUse` | `ActivityResourceUse` |
| Execution facts | `ActivityExecution` | `ActivityExecution` |
| Baseline/frozen choices | `ActivityBaseline` | `ActivityBaseline` |

Equipment and technicians may occupy only parts of an activity. Do not lock every resource for the entire activity for convenience, or check only availability at the start. Occupancy phases must match the business.

Inside the first-lesson runExample, use this as the machine entry in problem.resources. The at/q helpers and machine ID are defined by that function:

```kotlin
val calendarResource = ActivityResource(
    id = machine, capacity = q(2.0),
    unavailableTimes = listOf(TimeRange(at(1), at(2))),
    capacities = listOf(ActivityCapacity(TimeRange(at(3), at(4)), q(1.0)))
)
```

## 3. Separate Three Rolling-Horizon Inputs

Let now be 0 h. A running activity started at -1 h, finishes at 2 h and occupies future equipment on `[0,2)`. A completed activity ran from -3 to -2 h. Other activities remain decisions.

- **Execution facts:** retain original times/status; supply remaining future occupancy instead of rescheduling the whole running activity from 0 h.
- **Frozen choices:** committed starts/modes are hard restrictions. New downtime conflicting with them must yield a conflict/infeasibility, not a silent move for lower cost.
- **Adjustable baselines:** changes are allowed with configured adjustment costs. A baseline is not an execution fact and does not create new allowed starts.

Pass actual execution records through `executions` and committed or adjustable choices through `baselines`, retaining consistent task IDs in dependencies. Do not put completed work back into pending activities or charge full running occupancy again in the future window.

## 4. Make Two Controlled Changes

1. Remove the 4 h start from the calendar example. A mandatory non-cancellable activity has no feasible candidate; do not invent a 6 h start or automatic pause/resume.
2. Restore starts, add a frozen choice, then overlap it with downtime. Keep normal infeasibility distinct from technical input failure, and establish who may authorize unfreezing.

New starts, alternative resources and cancellation are different business decisions. Configure and validate them separately. Cancellation also requires dependency and material-event checks, not just removing a chart bar.

## 5. Validate Recovery Before Delivery

Retain completed facts, original running times and remaining occupancy. Preserve frozen choices, restrict adjustable activities to allowed candidates, validate resources/dependencies/inventory throughout, and recompute adjustment costs. A time-limited incumbent retains its real status.

Use input revisions/request IDs so old results cannot overwrite newer facts. Continue with [rolling optimization](/guide/rolling-optimization) and the [integration guide](../integration).

Repository regression commands live in [Verification](../verification). Complete the business input, call and result checks before using them.
