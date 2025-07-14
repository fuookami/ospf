import lodash from "lodash";
import dayjs from "dayjs";
import { SchemaDTO } from "./dto";

export type GanttSubItemVO = {
    name: string
    category: string
    startTime: string
    endTime: string
    info: Map<string, string>
}

export type GanttItemVO = {
    name: string
    category: string
    executor: string
    subItems: GanttSubItemVO[]
    scheduledStartTime: string
    scheduledEndTime: string
    startTime: string
    endTime: string
    renderStartTime: string
    renderEndTime: string
    produces: Map<string, string>
    consumption: Map<string, string>
    info: Map<string, string>
}

export type GanttLineVO = {
    name: string
    category: string,
    items: GanttItemVO[]
}

export type GanttChartVO = {
    startTime: string
    endTime: string
    linkInfo: string[]
    lines: GanttLineVO[]
}

export function dump(schema: SchemaDTO): GanttChartVO {
    const items: GanttItemVO[] = schema.tasks.map(task => {
        let scheduledStartTime = task.scheduledStartTime;
        if (!scheduledStartTime) {
            scheduledStartTime = task.startTime;
        }
        let scheduledEndTime = task.scheduledEndTime;
        if (!scheduledEndTime) {
            scheduledEndTime = task.endTime;
        }
        const produces: Map<string, string> = new Map();
        if (task.order) {
            produces.set("order", task.order);
        }
        if (task.produce) {
            produces.set("produce", task.produce);
        }
        if (task.products) {
            task.products!!.forEach((value, key) => {
                produces.set("product", `${key}, ${value}`);
            });
        }
        if (task.materials) {
            task.materials!!.forEach((value, key) => {
                produces.set("material", `${key}, ${value}`);
            });
        }
        return {
            name: task.name,
            category: task.category,
            executor: task.executor,
            subItems: task.subTasks.map(subTask => {
                return {
                    name: subTask.name,
                    category: subTask.category,
                    startTime: subTask.startTime,
                    endTime: subTask.endTime,
                    info: subTask.info
                }
            }),
            scheduledStartTime: scheduledStartTime,
            scheduledEndTime: scheduledEndTime,
            startTime: task.startTime,
            endTime: task.endTime,
            renderStartTime: lodash.min([scheduledStartTime, task.startTime]) as string,
            renderEndTime: lodash.max([scheduledEndTime, task.endTime]) as string,
            produces: produces,
            consumption: task.consumption,
            info: task.info
        };
    });
    const executors = lodash.uniq(items.map(item => item.executor));

    const lines: GanttLineVO[] = executors.map(executor => {
        return {
            name: executor,
            category: "Normal",
            items: items.filter(item => item.executor === executor).sort((lhs, rhs) => {
                if (lhs.renderStartTime < rhs.renderStartTime) {
                    return -1;
                } else if (lhs.renderStartTime > rhs.renderStartTime) {
                    return 1;
                } else {
                    return 0;
                }
            })
        };
    });

    return {
        startTime: lodash.minBy(items, "renderStartTime")!!.renderStartTime,
        endTime: lodash.maxBy(items, "renderEndTime")!!.renderEndTime,
        linkInfo: lodash.flatMap(schema.tasks, task => Object.keys(task.resources)),
        lines: lines
    };
}
